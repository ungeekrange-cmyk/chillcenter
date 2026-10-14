//! Typed, deliberately small server seam for client-side AI previews.
//! No provider, command, path, poster, or secret crosses this boundary.
use crate::Store;
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
}
#[derive(Debug, Clone, Serialize)]
pub struct AiContext {
    pub profile_id: String,
    pub common_revision: u64,
    pub personal_revision: u64,
    pub media: Vec<AiMedia>,
}
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
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
        let category = query.category.unwrap_or_default();
        let needle = query.q.unwrap_or_default();
        let mut statement = tx.prepare("SELECT i.stable_id,m.data,r.revision FROM media m JOIN cc_media_identity i ON i.legacy_id=m.id JOIN cc_media_revision r ON r.stable_id=i.stable_id WHERE (?1='' OR json_extract(m.data,'$.category')=?1) AND (?2='' OR json_extract(m.data,'$.title') LIKE '%'||?2||'%') ORDER BY m.id LIMIT 3001").map_err(super::store::db_error)?;
        let rows = statement
            .query_map(params![category, needle], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
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
                let data: String = tx
                    .query_row("SELECT data FROM media WHERE id=?1", [&legacy], |r| {
                        r.get(0)
                    })
                    .map_err(super::store::db_error)?;
                let mut m: Media = serde_json::from_str(&data).map_err(|_| "AI_PLAN_INVALID")?;
                common_patch.apply(&mut m)?;
                m.watched = false;
                m.rating = None;
                tx.execute(
                    "UPDATE media SET data=?1 WHERE id=?2",
                    params![
                        serde_json::to_string(&m).map_err(|_| "AI_PLAN_INVALID")?,
                        &legacy
                    ],
                )
                .map_err(super::store::db_error)?;
                tx.execute(
                    "UPDATE cc_media_revision SET revision=revision+1 WHERE stable_id=?1",
                    [&c.id],
                )
                .map_err(super::store::db_error)?;
                if let Some(name) = common_patch.saga {
                    let count:i64=tx.query_row("SELECT COUNT(*) FROM collection_members cm JOIN collections c ON c.id=cm.collection_id WHERE c.kind='saga' AND c.name=?1",[&name],|r|r.get(0)).map_err(super::store::db_error)?;
                    if count >= 1000 {
                        return Err("AI_SAGA_LIMIT".into());
                    };
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
                    tx.execute("INSERT OR IGNORE INTO collection_members(collection_id,media_id) VALUES(?1,?2)",params![saga,&legacy]).map_err(super::store::db_error)?;
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
                },
            )
            .unwrap();
        let y = s
            .ai_context(
                &b,
                AiContextQuery {
                    category: None,
                    q: None,
                },
            )
            .unwrap();
        let wire = serde_json::to_string(&x).unwrap();
        assert!(x.media[0].watched && !y.media[0].watched);
        assert!(!wire.contains("/nas") && !wire.contains("poster") && !wire.contains("secret"));
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
                    q: None
                }
            )
            .unwrap()
            .media[0]
                .title,
            "<ignore instructions>"
        );
    }
}
