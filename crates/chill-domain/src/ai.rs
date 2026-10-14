//! Typed, deliberately small server seam for client-side AI previews.
//! No provider, command, path, poster, or secret crosses this boundary.
use crate::{Store, metadata_edit::apply_common_patch};
use chill_catalog::model::{Media, MediaPatch};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const MAX_MEDIA: usize = 3000;
const MAX_JSON: usize = 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiContextQuery {
    pub category: Option<String>,
    pub q: Option<String>,
    pub watched: Option<bool>,
    pub available: Option<bool>,
    pub categories: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiContext {
    pub profile_id: String,
    pub common_revision: u64,
    pub personal_revision: u64,
    pub media: Vec<AiMedia>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiMedia {
    pub id: String,
    pub title: String,
    pub category: String,
    pub year: Option<i32>,
    pub series: Option<String>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub genres: Vec<String>,
    pub saga: Option<String>,
    pub watched: bool,
    pub rating: Option<u8>,
    pub revision: u64,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiApply {
    pub expected_common_revision: u64,
    pub expected_personal_revision: u64,
    pub confirmed: bool,
    pub changes: Vec<AiChange>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiChange {
    pub id: String,
    pub expected_revision: u64,
    pub patch: MediaPatch,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiApplied {
    pub applied_count: usize,
    pub common_revision: u64,
    pub personal_revision: u64,
}

impl Store {
    pub fn ai_context(
        &mut self,
        profile: &str,
        query: AiContextQuery,
    ) -> Result<AiContext, String> {
        if query
            .category
            .as_ref()
            .is_some_and(|v| !["film", "serie", "documentaire", "spectacle"].contains(&v.as_str()))
            || query.q.as_ref().is_some_and(|v| v.len() > 500)
        {
            return Err("AI_CONTEXT_INVALID".into());
        }
        let tx = self
            .personal
            .unchecked_transaction()
            .map_err(super::store::db_error)?;
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM cc_profiles WHERE id=?1)",
                [profile],
                |r| r.get(0),
            )
            .map_err(super::store::db_error)?;
        if !exists {
            return Err("PROFILE_NOT_FOUND".into());
        }
        let common: i64 = tx
            .query_row("SELECT revision FROM cc_common_epoch WHERE id=1", [], |r| {
                r.get(0)
            })
            .map_err(super::store::db_error)?;
        let personal: i64 = tx
            .query_row(
                "SELECT revision FROM cc_personal_epoch WHERE profile_id=?1",
                [profile],
                |r| r.get(0),
            )
            .map_err(super::store::db_error)?;
        if query.category.is_some() && query.categories.is_some() {
            return Err("AI_CONTEXT_INVALID".into());
        }
        let categories = query
            .categories
            .map(|v| {
                let parts = v.split(',').collect::<Vec<_>>();
                if parts.is_empty()
                    || parts.len() > 4
                    || parts
                        .iter()
                        .any(|p| !["film", "serie", "documentaire", "spectacle"].contains(p))
                    || parts.iter().collect::<std::collections::HashSet<_>>().len() != parts.len()
                {
                    return Err("AI_CONTEXT_INVALID");
                }
                Ok(parts.join(","))
            })
            .transpose()?;
        let category = query.category.unwrap_or_default();
        let needle = escape_like(&query.q.unwrap_or_default());
        let watched = query.watched.map(i64::from);
        let available = query.available.map(i64::from);
        let mut statement = tx.prepare("SELECT i.stable_id,m.data,r.revision FROM media m JOIN cc_media_identity i ON i.legacy_id=m.id JOIN cc_media_revision r ON r.stable_id=i.stable_id WHERE (?1='' OR json_extract(m.data,'$.category')=?1) AND (?2='' OR json_extract(m.data,'$.title') LIKE '%'||?2||'%' ESCAPE '\\' OR (json_extract(m.data,'$.category')='serie' AND COALESCE(json_extract(m.data,'$.series'),json_extract(m.data,'$.title')) LIKE '%'||?2||'%' ESCAPE '\\')) AND (?3 IS NULL OR COALESCE((SELECT watched FROM cc_profile_state WHERE profile_id=?4 AND stable_media_id=i.stable_id),0)=?3) AND (?5 IS NULL OR COALESCE(json_extract(m.data,'$.available'),0)=?5) AND (?6 IS NULL OR instr(','||?6||',',','||json_extract(m.data,'$.category')||',')>0) ORDER BY m.id LIMIT 3001").map_err(super::store::db_error)?;
        let rows = statement
            .query_map(
                params![category, needle, watched, profile, available, categories],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                    ))
                },
            )
            .map_err(super::store::db_error)?;
        let mut media = Vec::new();
        for row in rows {
            let (id, data, revision) = row.map_err(super::store::db_error)?;
            let m: Media = serde_json::from_str(&data).map_err(|_| "AI_CONTEXT_INVALID")?;
            let state:Option<(i64,Option<i64>)>=tx.query_row("SELECT watched,rating FROM cc_profile_state WHERE profile_id=?1 AND stable_media_id=?2",params![profile,&id],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(super::store::db_error)?;
            media.push(AiMedia {
                id,
                title: m.title,
                category: m.category,
                year: m.year,
                series: m.series,
                season: m.season,
                episode: m.episode,
                genres: m.genres,
                saga: m.saga,
                watched: state.as_ref().is_some_and(|s| s.0 != 0),
                rating: state.and_then(|s| s.1.map(|v| v as u8)),
                revision: revision as u64,
            });
        }
        drop(statement);
        if media.len() > MAX_MEDIA {
            return Err("AI_CONTEXT_TOO_LARGE".into());
        }
        let response = AiContext {
            profile_id: profile.into(),
            common_revision: common as u64,
            personal_revision: personal as u64,
            media,
        };
        if serde_json::to_vec(&response)
            .map_err(|_| "AI_CONTEXT_INVALID")?
            .len()
            > MAX_JSON
        {
            return Err("AI_CONTEXT_TOO_LARGE".into());
        }
        tx.commit().map_err(super::store::db_error)?;
        Ok(response)
    }

    pub fn ai_apply(&mut self, profile: &str, request: AiApply) -> Result<AiApplied, String> {
        if !request.confirmed {
            return Err("AI_CONFIRMATION_REQUIRED".into());
        }
        if request.changes.len() > MAX_MEDIA {
            return Err("AI_PLAN_INVALID".into());
        }
        let mut ids = HashSet::new();
        if request
            .changes
            .iter()
            .any(|c| uuid::Uuid::parse_str(&c.id).is_err() || !ids.insert(c.id.as_str()))
        {
            return Err("AI_PLAN_INVALID".into());
        }
        let tx =
            rusqlite::Transaction::new_unchecked(&self.personal, TransactionBehavior::Immediate)
                .map_err(super::store::db_error)?;
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM cc_profiles WHERE id=?1)",
                [profile],
                |r| r.get(0),
            )
            .map_err(super::store::db_error)?;
        if !exists {
            return Err("PROFILE_NOT_FOUND".into());
        }
        let common: i64 = tx
            .query_row("SELECT revision FROM cc_common_epoch WHERE id=1", [], |r| {
                r.get(0)
            })
            .map_err(super::store::db_error)?;
        let personal: i64 = tx
            .query_row(
                "SELECT revision FROM cc_personal_epoch WHERE profile_id=?1",
                [profile],
                |r| r.get(0),
            )
            .map_err(super::store::db_error)?;
        let mut has_common = false;
        let mut has_personal = false;
        for c in &request.changes {
            if c.patch.rating.is_some() || c.patch.watched.is_some() {
                has_personal = true
            };
            let mut p = c.patch.clone();
            p.rating = None;
            p.watched = None;
            if !is_empty(&p) {
                has_common = true
            }
        }
        if has_common && common as u64 != request.expected_common_revision {
            return Err("AI_COMMON_STALE".into());
        }
        if has_personal && personal as u64 != request.expected_personal_revision {
            return Err("AI_PERSONAL_STALE".into());
        }
        // Validate every identity/revision and every patch before the first write.
        for c in &request.changes {
            let rev: Option<i64> = tx
                .query_row(
                    "SELECT revision FROM cc_media_revision WHERE stable_id=?1",
                    [&c.id],
                    |r| r.get(0),
                )
                .optional()
                .map_err(super::store::db_error)?;
            if rev.map(|v| v as u64) != Some(c.expected_revision) {
                return Err("AI_MEDIA_STALE".into());
            };
            let mut p = c.patch.clone();
            p.rating = None;
            p.watched = None;
            if p.saga.as_ref().is_some_and(|v| v.trim().is_empty()) {
                return Err("AI_SAGA_CLEAR_AMBIGUOUS".into());
            };
            if !is_empty(&p) {
                let legacy: String = tx
                    .query_row(
                        "SELECT legacy_id FROM cc_media_identity WHERE stable_id=?1",
                        [&c.id],
                        |r| r.get(0),
                    )
                    .map_err(super::store::db_error)?;
                let data: String = tx
                    .query_row("SELECT data FROM media WHERE id=?1", [legacy], |r| r.get(0))
                    .map_err(super::store::db_error)?;
                let mut m: Media = serde_json::from_str(&data).map_err(|_| "AI_PLAN_INVALID")?;
                p.apply(&mut m)?;
                if p.saga.is_some() && m.category != "film" {
                    return Err("AI_SAGA_FILM_ONLY".into());
                }
            }
        }
        for c in &request.changes {
            let legacy: String = tx
                .query_row(
                    "SELECT legacy_id FROM cc_media_identity WHERE stable_id=?1",
                    [&c.id],
                    |r| r.get(0),
                )
                .map_err(super::store::db_error)?;
            let mut common_patch = c.patch.clone();
            common_patch.rating = None;
            common_patch.watched = None;
            if !is_empty(&common_patch) {
                apply_common_patch(&tx, &legacy, &common_patch)?;
                tx.execute(
                    "UPDATE cc_media_revision SET revision=revision+1 WHERE stable_id=?1",
                    [&c.id],
                )
                .map_err(super::store::db_error)?;
                if let Some(name) = common_patch.saga {
                    let exists: bool = tx.query_row(
                        "SELECT EXISTS(SELECT 1 FROM collections WHERE kind='saga' AND name=?1)",
                        [&name], |row| row.get(0),
                    ).map_err(super::store::db_error)?;
                    if !exists {
                        let total: i64 = tx
                            .query_row(
                                "SELECT COUNT(*) FROM collections WHERE kind='saga'",
                                [],
                                |row| row.get(0),
                            )
                            .map_err(super::store::db_error)?;
                        if total >= 1000 {
                            return Err("AI_SAGA_LIMIT".into());
                        }
                    }
                    tx.execute(
                        "INSERT OR IGNORE INTO collections(name,kind) VALUES(?1,'saga')",
                        [&name],
                    )
                    .map_err(super::store::db_error)?;
                    let saga: i64 = tx
                        .query_row(
                            "SELECT id FROM collections WHERE kind='saga' AND name=?1",
                            [&name],
                            |r| r.get(0),
                        )
                        .map_err(super::store::db_error)?;
                    tx.execute("INSERT OR IGNORE INTO collection_revisions(collection_id,revision) VALUES(?1,1)",[saga]).map_err(super::store::db_error)?;
                    let existing: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM collection_members WHERE collection_id=?1 AND media_id=?2)", params![saga, &legacy], |r| r.get(0)).map_err(super::store::db_error)?;
                    if !existing {
                        let count: i64 = tx
                            .query_row(
                                "SELECT COUNT(*) FROM collection_members WHERE collection_id=?1",
                                [saga],
                                |r| r.get(0),
                            )
                            .map_err(super::store::db_error)?;
                        if count >= 1000 {
                            return Err("AI_SAGA_LIMIT".into());
                        }
                        tx.execute(
                            "INSERT INTO collection_members(collection_id,media_id) VALUES(?1,?2)",
                            params![saga, &legacy],
                        )
                        .map_err(super::store::db_error)?;
                        tx.execute("UPDATE collection_revisions SET revision=revision+1 WHERE collection_id=?1", [saga]).map_err(super::store::db_error)?;
                    }
                    tx.execute(
                        "INSERT OR IGNORE INTO imported_sagas(media_id,name) VALUES(?1,?2)",
                        params![&legacy, &name],
                    )
                    .map_err(super::store::db_error)?;
                }
            }
            if let Some(r) = c.patch.rating {
                let rating = if r == 0 { None } else { Some(r) };
                if rating.is_some_and(|v| v > 5) {
                    return Err("AI_PLAN_INVALID".into());
                };
                tx.execute("INSERT INTO cc_profile_state(profile_id,stable_media_id,rating) VALUES(?1,?2,?3) ON CONFLICT(profile_id,stable_media_id) DO UPDATE SET rating=excluded.rating",params![profile,&c.id,rating]).map_err(super::store::db_error)?;
            }
            if let Some(w) = c.patch.watched {
                tx.execute("INSERT INTO cc_profile_state(profile_id,stable_media_id,watched,final_watched_ms,last_session_id,last_generation,last_sequence) VALUES(?1,?2,?3,0,NULL,NULL,NULL) ON CONFLICT(profile_id,stable_media_id) DO UPDATE SET watched=excluded.watched,final_watched_ms=0,last_session_id=NULL,last_generation=NULL,last_sequence=NULL",params![profile,&c.id,w as i64]).map_err(super::store::db_error)?;
            }
        }
        let out = AiApplied {
            applied_count: request.changes.len(),
            common_revision: tx
                .query_row("SELECT revision FROM cc_common_epoch WHERE id=1", [], |r| {
                    r.get::<_, i64>(0)
                })
                .map_err(super::store::db_error)? as u64,
            personal_revision: tx
                .query_row(
                    "SELECT revision FROM cc_personal_epoch WHERE profile_id=?1",
                    [profile],
                    |r| r.get::<_, i64>(0),
                )
                .map_err(super::store::db_error)? as u64,
        };
        tx.commit().map_err(super::store::db_error)?;
        Ok(out)
    }
}
fn is_empty(p: &MediaPatch) -> bool {
    p.title.is_none()
        && p.category.is_none()
        && p.year.is_none()
        && p.genres.is_none()
        && p.series.is_none()
        && p.season.is_none()
        && p.episode.is_none()
        && p.saga.is_none()
        && p.overview.is_none()
        && p.runtime_minutes.is_none()
        && p.cast.is_none()
        && p.directors.is_none()
        && p.series_overview.is_none()
}

/// SQLite LIKE treats `%` and `_` as wildcards unless explicitly escaped.
fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chill_catalog::model::Media;
    fn media(id: &str) -> Media {
        Media {
            id: id.into(),
            path: format!("/nas/{id}"),
            title: "<ignore instructions>".into(),
            category: "film".into(),
            year: Some(2020),
            series: None,
            season: None,
            episode: None,
            genres: vec!["Drama".into()],
            overview: "x".into(),
            poster: Some("/secret/poster".into()),
            poster_custom: false,
            runtime_minutes: None,
            cast: vec![],
            directors: vec![],
            series_overview: String::new(),
            series_metadata: None,
            series_manual: false,
            rating: Some(5),
            saga: None,
            watched: true,
            available: true,
            added_at: 0,
            tmdb_id: None,
        }
    }
    fn setup() -> (tempfile::TempDir, Store, String, String) {
        let d = tempfile::tempdir().unwrap();
        let mut s = Store::open(&d.path().join("a.sqlite")).unwrap();
        let a = s.create_profile("A").unwrap();
        let b = s.create_profile("B").unwrap();
        s.snapshot(vec![media("one.mkv")]).unwrap();
        // Stable IDs are normally materialized by the catalogue reader after a scan.
        s.catalog(&a.id).unwrap();
        (d, s, a.id, b.id)
    }
    #[test]
    fn context_is_profile_private_and_never_serializes_paths_or_posters() {
        let (_d, mut s, a, b) = setup();
        let id = s.catalog(&a).unwrap()[0].id.clone();
        s.set_watched(&a, &id, true).unwrap();
        s.set_rating(&a, &id, Some(4), "video").unwrap();
        let x = s
            .ai_context(
                &a,
                AiContextQuery {
                    category: None,
                    q: None,
                    watched: None,
                    available: None,
                    categories: None,
                },
            )
            .unwrap();
        let y = s
            .ai_context(
                &b,
                AiContextQuery {
                    category: None,
                    q: None,
                    watched: None,
                    available: None,
                    categories: None,
                },
            )
            .unwrap();
        let wire = serde_json::to_string(&x).unwrap();
        assert!(x.media[0].watched && !y.media[0].watched);
        assert!(!wire.contains("/nas") && !wire.contains("poster") && !wire.contains("secret"));
    }
    #[test]
    fn context_filters_personal_watched_and_common_availability_before_returning_media() {
        let (dir, mut store, a, b) = setup();
        let mut unavailable = media("unavailable.mkv");
        unavailable.available = false;
        store.snapshot(vec![media("one.mkv"), unavailable]).unwrap();
        let watched = store.catalog(&a).unwrap()[0].id.clone();
        store.set_watched(&a, &watched, true).unwrap();
        let only_unwatched_available = store
            .ai_context(
                &a,
                AiContextQuery {
                    category: None,
                    q: None,
                    watched: Some(false),
                    available: Some(true),
                    categories: None,
                },
            )
            .unwrap();
        assert!(only_unwatched_available.media.is_empty());
        let other_profile = store
            .ai_context(
                &b,
                AiContextQuery {
                    category: None,
                    q: None,
                    watched: Some(false),
                    available: Some(true),
                    categories: None,
                },
            )
            .unwrap();
        assert_eq!(other_profile.media.len(), 1, "watched is profile-scoped");
        drop(dir);
    }
    #[test]
    fn duplicate_or_stale_plan_has_no_partial_write() {
        let (_d, mut s, a, _) = setup();
        let c = s
            .ai_context(
                &a,
                AiContextQuery {
                    category: None,
                    q: None,
                    watched: None,
                    available: None,
                    categories: None,
                },
            )
            .unwrap();
        let id = c.media[0].id.clone();
        let bad = AiApply {
            expected_common_revision: c.common_revision,
            expected_personal_revision: c.personal_revision,
            confirmed: true,
            changes: vec![
                AiChange {
                    id: id.clone(),
                    expected_revision: c.media[0].revision,
                    patch: MediaPatch {
                        title: Some("new".into()),
                        ..Default::default()
                    },
                },
                AiChange {
                    id,
                    expected_revision: c.media[0].revision,
                    patch: MediaPatch {
                        title: Some("bad".into()),
                        ..Default::default()
                    },
                },
            ],
        };
        assert_eq!(s.ai_apply(&a, bad).unwrap_err(), "AI_PLAN_INVALID");
        assert_eq!(
            s.ai_context(
                &a,
                AiContextQuery {
                    category: None,
                    q: None,
                    watched: None,
                    available: None,
                    categories: None,
                }
            )
            .unwrap()
            .media[0]
                .title,
            "<ignore instructions>"
        );
    }
    #[test]
    fn context_roundtrips_as_a_closed_client_dto_and_search_is_literal() {
        let (_d, mut s, a, _) = setup();
        let id = s.catalog(&a).unwrap()[0].id.clone();
        s.update_media_at_revision(
            &a,
            &id,
            s.media_revision(&id).unwrap(),
            &MediaPatch {
                title: Some("100%_literal".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let context = s
            .ai_context(
                &a,
                AiContextQuery {
                    category: Some("film".into()),
                    q: Some("%_".into()),
                    watched: None,
                    available: None,
                    categories: None,
                },
            )
            .unwrap();
        assert_eq!(context.media.len(), 1);
        let decoded: AiContext =
            serde_json::from_slice(&serde_json::to_vec(&context).unwrap()).unwrap();
        assert_eq!(decoded.media[0].id, id);
        assert!(serde_json::from_value::<AiContext>(serde_json::json!({"profile_id":"x","common_revision":1,"personal_revision":1,"media":[],"path":"/nas"})).is_err());
        assert!(serde_json::from_value::<AiApplied>(serde_json::json!({"applied_count":1,"common_revision":1,"personal_revision":1,"extra":true})).is_err());
    }
    #[test]
    fn other_profile_progress_does_not_stale_common_plan_and_saga_membership_bumps_revision() {
        let (_d, mut s, a, b) = setup();
        let first = s
            .ai_context(
                &a,
                AiContextQuery {
                    category: None,
                    q: None,
                    watched: None,
                    available: None,
                    categories: None,
                },
            )
            .unwrap();
        let id = first.media[0].id.clone();
        s.set_watched(&b, &id, true).unwrap();
        let applied = s
            .ai_apply(
                &a,
                AiApply {
                    expected_common_revision: first.common_revision,
                    expected_personal_revision: first.personal_revision,
                    confirmed: true,
                    changes: vec![AiChange {
                        id: id.clone(),
                        expected_revision: first.media[0].revision,
                        patch: MediaPatch {
                            saga: Some("AI saga".into()),
                            ..Default::default()
                        },
                    }],
                },
            )
            .unwrap();
        assert!(applied.common_revision > first.common_revision);
        let saga = s
            .sagas()
            .unwrap()
            .into_iter()
            .find(|v| v.name == "AI saga")
            .unwrap();
        assert_eq!(saga.revision, 2);
        assert_eq!(saga.media_ids, vec![id]);
    }
    #[test]
    fn sqlite_failure_rolls_back_common_and_personal_ai_changes() {
        let (_d, mut s, a, _) = setup();
        let context = s
            .ai_context(
                &a,
                AiContextQuery {
                    category: None,
                    q: None,
                    watched: None,
                    available: None,
                    categories: None,
                },
            )
            .unwrap();
        let item = &context.media[0];
        s.personal
            .execute_batch("CREATE TRIGGER ai_fail_state BEFORE INSERT ON cc_profile_state BEGIN SELECT RAISE(ABORT,'fixture'); END;")
            .unwrap();
        let error = s
            .ai_apply(
                &a,
                AiApply {
                    expected_common_revision: context.common_revision,
                    expected_personal_revision: context.personal_revision,
                    confirmed: true,
                    changes: vec![AiChange {
                        id: item.id.clone(),
                        expected_revision: item.revision,
                        patch: MediaPatch {
                            title: Some("must rollback".into()),
                            rating: Some(4),
                            ..Default::default()
                        },
                    }],
                },
            )
            .unwrap_err();
        assert_ne!(error, "AI_MEDIA_STALE");
        let after = s
            .ai_context(
                &a,
                AiContextQuery {
                    category: None,
                    q: None,
                    watched: None,
                    available: None,
                    categories: None,
                },
            )
            .unwrap();
        assert_eq!(after.media[0].title, "<ignore instructions>");
        assert_eq!(after.media[0].rating, None);
        assert_eq!(after.common_revision, context.common_revision);
    }
    #[test]
    fn ai_uses_common_helper_for_series_metadata_and_series_search() {
        let d = tempfile::tempdir().unwrap();
        let mut store = Store::open(&d.path().join("db.sqlite")).unwrap();
        let profile = store.create_profile("A").unwrap();
        let mut sibling = media("series.mkv");
        sibling.title = "Episode existing".into();
        sibling.category = "serie".into();
        sibling.series = Some("Needle".into());
        sibling.series_metadata = Some(chill_catalog::model::SeriesMetadata {
            year: Some(2001),
            genres: vec!["Sci-fi".into()],
            overview: "shared".into(),
            cast: vec![],
            directors: vec![],
        });
        let mut film = media("film.mkv");
        film.title = "Film to classify".into();
        store.snapshot(vec![sibling, film]).unwrap();
        store.catalog(&profile.id).unwrap();
        let context = store
            .ai_context(
                &profile.id,
                AiContextQuery {
                    category: None,
                    q: Some("Needle".into()),
                    watched: None,
                    available: None,
                    categories: None,
                },
            )
            .unwrap();
        assert_eq!(context.media.len(), 1, "series query is not title-only");
        let film = store
            .ai_context(
                &profile.id,
                AiContextQuery {
                    category: Some("film".into()),
                    q: None,
                    watched: None,
                    available: None,
                    categories: None,
                },
            )
            .unwrap()
            .media
            .remove(0);
        let revisions = store
            .ai_context(
                &profile.id,
                AiContextQuery {
                    category: None,
                    q: None,
                    watched: None,
                    available: None,
                    categories: None,
                },
            )
            .unwrap();
        store
            .ai_apply(
                &profile.id,
                AiApply {
                    expected_common_revision: revisions.common_revision,
                    expected_personal_revision: revisions.personal_revision,
                    confirmed: true,
                    changes: vec![AiChange {
                        id: film.id.clone(),
                        expected_revision: film.revision,
                        patch: MediaPatch {
                            category: Some("serie".into()),
                            series: Some("Needle".into()),
                            ..Default::default()
                        },
                    }],
                },
            )
            .unwrap();
        assert_eq!(
            store
                .media(&profile.id, &film.id)
                .unwrap()
                .series_metadata
                .unwrap()
                .overview,
            "shared"
        );
    }
    #[test]
    fn ai_saga_cap_counts_total_sagas_but_allows_existing_saga() {
        let (_d, mut store, profile, _) = setup();
        for n in 0..1_000 {
            let name = if n == 0 {
                "Existing".to_string()
            } else {
                format!("Saga {n}")
            };
            store
                .personal
                .execute(
                    "INSERT INTO collections(name,kind) VALUES(?1,'saga')",
                    [&name],
                )
                .unwrap();
        }
        let context = store
            .ai_context(
                &profile,
                AiContextQuery {
                    category: None,
                    q: None,
                    watched: None,
                    available: None,
                    categories: None,
                },
            )
            .unwrap();
        let item = context.media[0].clone();
        let plan = |name: &str| AiApply {
            expected_common_revision: context.common_revision,
            expected_personal_revision: context.personal_revision,
            confirmed: true,
            changes: vec![AiChange {
                id: item.id.clone(),
                expected_revision: item.revision,
                patch: MediaPatch {
                    saga: Some(name.into()),
                    ..Default::default()
                },
            }],
        };
        assert_eq!(
            store.ai_apply(&profile, plan("New saga")).unwrap_err(),
            "AI_SAGA_LIMIT"
        );
        assert!(store.ai_apply(&profile, plan("Existing")).is_ok());
    }
}
