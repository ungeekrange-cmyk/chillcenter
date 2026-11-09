//! Bounded, read-only home shelves. Never scan media paths or call providers.
use crate::Store;
use chill_catalog::model::Media;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};

const MAX_ROWS: usize = 10_000;
const MAX_SHELF: usize = 20;
#[derive(Clone, Serialize)]
pub struct HomeProgress {
    pub position_ms: u64,
    pub duration_ms: u64,
}
#[derive(Clone, Serialize)]
pub struct HomeItem {
    pub media: Media,
    pub series_key: Option<String>,
    pub season: Option<i32>,
    pub progress: Option<HomeProgress>,
    pub matches: Vec<String>,
    pub resume_position_ms: u64,
    pub episode_counts: Option<HomeEpisodeCounts>,
}
#[derive(Clone, Serialize)]
pub struct HomeEpisodeCounts {
    pub watched: usize,
    pub total: usize,
}
#[derive(Serialize)]
pub struct HomeCatalog {
    pub watch_revision: u64,
    pub continue_watching: Vec<HomeItem>,
    pub recently_added: Vec<HomeItem>,
    pub recommendations: Vec<HomeItem>,
}
#[derive(Clone)]
struct Row {
    media: Media,
    stable: String,
    relative_path: String,
    watched: bool,
    rating: Option<u8>,
    position: u64,
    duration: Option<u64>,
    last: Option<i64>,
    key: String,
}

fn normalized(values: &[String]) -> HashSet<String> {
    values
        .iter()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty())
        .collect()
}
fn common(a: &[String], b: &[String]) -> Vec<String> {
    let a = normalized(a);
    let mut seen = HashSet::new();
    b.iter()
        .filter(|v| {
            let k = v.trim().to_lowercase();
            a.contains(&k) && seen.insert(k)
        })
        .map(|v| v.trim().to_owned())
        .collect()
}
fn common_series(media: &mut Media) {
    if media.category == "serie"
        && let Some(metadata) = &media.series_metadata
    {
        media.genres = metadata.genres.clone();
        media.cast = metadata.cast.clone();
        media.directors = metadata.directors.clone();
        media.year = metadata.year;
        media.overview = metadata.overview.clone();
    }
}
fn features(media: &Media) -> (&[String], &[String], &[String]) {
    if media.category == "serie"
        && let Some(metadata) = &media.series_metadata
    {
        (&metadata.genres, &metadata.cast, &metadata.directors)
    } else {
        (&media.genres, &media.cast, &media.directors)
    }
}
fn matches(seed: &Media, candidate: &Media) -> Vec<String> {
    let (genres, cast, directors) = features(seed);
    let (other_genres, other_cast, other_directors) = features(candidate);
    let mut out = common(genres, other_genres);
    out.extend(common(cast, other_cast));
    out.extend(common(directors, other_directors));
    out.sort();
    out.dedup();
    out
}

fn score(seeds: &[&Row], candidate: &Row) -> (usize, Vec<String>) {
    let mut total = 0;
    let mut labels = Vec::new();
    for seed in seeds {
        let values = matches(&seed.media, &candidate.media);
        total += values.len();
        labels.extend(values);
    }
    labels.sort();
    labels.dedup();
    (total, labels)
}

impl Store {
    fn home_item(
        &self,
        profile: &str,
        row: &Row,
        series_key: Option<String>,
        season: Option<i32>,
        progress: Option<HomeProgress>,
        matches: Vec<String>,
    ) -> HomeItem {
        let mut media = row.media.clone();
        media.id = row.stable.clone();
        media.path = row.relative_path.clone();
        media.watched = row.watched;
        media.rating = row.rating;
        if media.poster.is_some() {
            media.poster = Some(format!(
                "/api/2026-10-14/profiles/{profile}/media/{}/artwork",
                media.id
            ));
        }
        if media.category == "serie" {
            common_series(&mut media);
            media.title = media.series.clone().unwrap_or(media.title);
            media.season = None;
            media.episode = None;
        }
        HomeItem {
            media,
            series_key,
            season,
            progress,
            matches,
            resume_position_ms: row.position,
            episode_counts: None,
        }
    }
    pub fn home_catalog(&mut self, profile: &str) -> Result<HomeCatalog, String> {
        self.home_catalog_with_durations(profile, &HashMap::new())
    }
    /// Container durations are read by the server, never inferred from TMDB.
    /// This read-only overlay does not rewrite playback history or measurements.
    pub fn home_catalog_with_durations(
        &mut self,
        profile: &str,
        durations: &HashMap<String, u64>,
    ) -> Result<HomeCatalog, String> {
        self.require_profile(profile)?;
        self.refresh_series_groups()?;
        // One bounded projection: never call profile_view (several SQL reads) for each raw row.
        let mut statement=self.personal.prepare(r#"SELECT m.data,i.stable_id,i.relative_path,COALESCE(s.watched,0),s.rating,COALESCE(s.position_ms,0),d.duration_ms,s.last_watched_at,CASE WHEN json_extract(m.data,'$.category')='serie' THEN CASE WHEN json_extract(m.data,'$.tmdb_id') IS NOT NULL THEN 'tmdb:tv:'||json_extract(m.data,'$.tmdb_id') ELSE COALESCE(g.source_key,'media:'||i.stable_id) END ELSE 'video:'||i.stable_id END FROM media m JOIN cc_media_identity i ON i.legacy_id=m.id LEFT JOIN cc_profile_state s ON s.profile_id=?1 AND s.stable_media_id=i.stable_id LEFT JOIN cc_resume_measurement d ON d.profile_id=?1 AND d.stable_media_id=i.stable_id LEFT JOIN cc_series_group g ON g.legacy_id=m.id ORDER BY m.id LIMIT 10001"#).map_err(|_|"Accueil indisponible.".to_string())?;
        let values = statement
            .query_map([profile], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, bool>(3)?,
                    r.get::<_, Option<u8>>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, Option<i64>>(6)?,
                    r.get::<_, Option<i64>>(7)?,
                    r.get::<_, String>(8)?,
                ))
            })
            .map_err(|_| "Accueil indisponible.".to_string())?;
        let mut all = Vec::new();
        for value in values {
            let (data, stable, relative_path, watched, rating, position, duration, last, key) =
                value.map_err(|_| "Accueil indisponible.".to_string())?;
            let duration = duration
                .map(|v| v.max(0) as u64)
                .filter(|v| *v > 0)
                .or_else(|| durations.get(&stable).copied().filter(|v| *v > 0));
            all.push(Row {
                media: serde_json::from_str(&data)
                    .map_err(|_| "Accueil indisponible.".to_string())?,
                stable,
                relative_path,
                watched,
                rating,
                position: position.max(0) as u64,
                duration,
                last,
                key,
            });
        }
        if all.len() > MAX_ROWS {
            return Err("Accueil trop volumineux.".into());
        }
        // Counting observes every member; availability chooses only a final playable proxy.
        let mut entities: BTreeMap<String, Vec<&Row>> = BTreeMap::new();
        for row in &all {
            entities.entry(row.key.clone()).or_default().push(row);
        }
        let mut continuing = Vec::new();
        for (key, rows) in &entities {
            if rows[0].media.category != "serie" {
                if let Some(row) = rows
                    .iter()
                    .copied()
                    .find(|r| r.media.available && !r.watched && meaningful_resume(r))
                {
                    continuing.push((
                        row.position,
                        self.home_item(
                            profile,
                            row,
                            None,
                            None,
                            // Legacy/imported resumes still have a valid position.
                            // Only a measured duration permits a percentage badge.
                            row.duration
                                .filter(|duration| *duration > 0)
                                .map(|duration| HomeProgress {
                                    position_ms: row.position,
                                    duration_ms: duration,
                                }),
                            vec![],
                        ),
                    ));
                }
                continue;
            }
            let mut seasons: BTreeMap<i32, Vec<&Row>> = BTreeMap::new();
            for row in rows {
                seasons
                    .entry(row.media.season.unwrap_or(-1))
                    .or_default()
                    .push(*row);
            }
            for (season, members) in seasons {
                let started = members.iter().any(|r| r.watched || meaningful_resume(r));
                let unfinished = members.iter().any(|r| !r.watched);
                if !started || !unfinished {
                    continue;
                }
                if let Some(proxy) = members
                    .iter()
                    .copied()
                    .filter(|r| r.media.available && !r.watched)
                    .max_by_key(|r| r.position)
                {
                    let progress =
                        (proxy.position > 0 && proxy.duration.unwrap_or(0) > 0).then(|| {
                            HomeProgress {
                                position_ms: proxy.position,
                                duration_ms: proxy.duration.unwrap(),
                            }
                        });
                    let mut item = self.home_item(
                        profile,
                        proxy,
                        Some(key.clone()),
                        (season >= 0).then_some(season),
                        progress,
                        vec![],
                    );
                    item.episode_counts = Some(HomeEpisodeCounts {
                        watched: members.iter().filter(|r| r.watched).count(),
                        total: members.len(),
                    });
                    continuing.push((proxy.position, item));
                    break;
                }
            }
        }
        continuing.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.media.title.cmp(&b.1.media.title))
                .then_with(|| a.1.media.id.cmp(&b.1.media.id))
        });
        let continue_watching = continuing
            .into_iter()
            .take(MAX_SHELF)
            .map(|(_, v)| v)
            .collect();
        let mut recent: BTreeMap<String, &Row> = BTreeMap::new();
        for row in &all {
            if !row.media.available {
                continue;
            }
            let entry = recent.entry(row.key.clone()).or_insert(row);
            if row.media.added_at > entry.media.added_at {
                *entry = row;
            }
        }
        let mut recently_added: Vec<_> = recent
            .into_iter()
            .map(|(key, row)| {
                self.home_item(
                    profile,
                    row,
                    (row.media.category == "serie").then_some(key),
                    None,
                    None,
                    vec![],
                )
            })
            .collect();
        recently_added.sort_by(|a, b| {
            b.media
                .added_at
                .cmp(&a.media.added_at)
                .then_with(|| a.media.title.cmp(&b.media.title))
                .then_with(|| a.media.id.cmp(&b.media.id))
        });
        recently_added.truncate(MAX_SHELF);
        let mut seeds: Vec<_> = all
            .iter()
            .filter(|r| r.watched && r.last.is_some())
            .collect();
        seeds.sort_by(|a, b| b.last.cmp(&a.last).then_with(|| a.stable.cmp(&b.stable)));
        let mut seed_entities = HashSet::new();
        seeds.retain(|row| seed_entities.insert(row.key.clone()));
        seeds.truncate(5);
        let excluded: HashSet<_> = seeds.iter().map(|r| r.key.clone()).collect();
        let mut candidates: HashMap<String, (usize, Vec<String>, &Row)> = HashMap::new();
        for row in &all {
            if !row.media.available || row.watched || excluded.contains(&row.key) {
                continue;
            }
            let (value, labels) = score(&seeds, row);
            if value == 0 {
                continue;
            }
            let replace = candidates.get(&row.key).is_none_or(|(old, _, proxy)| {
                value > *old || (value == *old && row.stable < proxy.stable)
            });
            if replace {
                candidates.insert(row.key.clone(), (value, labels, row));
            }
        }
        let mut recommendations: Vec<_> = candidates
            .into_iter()
            .map(|(key, (value, labels, row))| {
                (
                    value,
                    self.home_item(
                        profile,
                        row,
                        (row.media.category == "serie").then_some(key),
                        None,
                        None,
                        labels,
                    ),
                )
            })
            .collect();
        recommendations.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.media.title.cmp(&b.1.media.title))
                .then_with(|| a.1.media.id.cmp(&b.1.media.id))
        });
        let recommendations = recommendations
            .into_iter()
            .take(MAX_SHELF)
            .map(|(_, v)| v)
            .collect();
        Ok(HomeCatalog {
            watch_revision: self.sync_revision(profile)?.0,
            continue_watching,
            recently_added,
            recommendations,
        })
    }
}

fn meaningful_resume(row: &Row) -> bool {
    row.position > 0
        && row.duration.is_none_or(|duration| {
            // A brief trial is not an ongoing film. Do not mark it watched or erase
            // its saved position; it remains resumable from its detail page.
            row.position >= duration.div_ceil(20) && row.position < duration
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn media(name: &str) -> Media {
        chill_catalog::scanner::parse_file(
            std::path::Path::new("/fixture"),
            &std::path::Path::new("/fixture").join(name),
        )
    }
    fn episode(name: &str, season: i32, episode: i32, added_at: i64) -> Media {
        let mut m = media(name);
        m.category = "serie".into();
        m.series = Some("Show".into());
        m.season = Some(season);
        m.episode = Some(episode);
        m.added_at = added_at;
        m.genres = vec!["Drama".into()];
        m
    }

    #[test]
    fn continue_keeps_legacy_resumes_without_inventing_measured_progress() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy-home.sqlite");
        let mut store = Store::open(&path).unwrap();
        let profile = store.create_profile("A").unwrap().id;
        let other = store.create_profile("B").unwrap().id;
        let mut unavailable = media("Unavailable.mkv");
        unavailable.available = false;
        store
            .snapshot(vec![
                media("Legacy.mkv"),
                media("Measured.mkv"),
                media("Unstarted.mkv"),
                media("Completed.mkv"),
                unavailable,
            ])
            .unwrap();
        let ids: HashMap<_, _> = store
            .catalog(&profile)
            .unwrap()
            .into_iter()
            .map(|m| (m.title, m.id))
            .collect();
        for title in ["Legacy", "Measured", "Completed", "Unavailable"] {
            store.personal.execute("INSERT INTO cc_profile_state(profile_id,stable_media_id,position_ms) VALUES(?1,?2,?3)",
                rusqlite::params![profile, ids[title], if title == "Legacy" { 3_025_917 } else { 5_000 }]).unwrap();
        }
        store.personal.execute("INSERT INTO cc_resume_measurement(profile_id,stable_media_id,duration_ms) VALUES(?1,?2,100000)",
            rusqlite::params![profile, ids["Measured"]]).unwrap();
        store
            .set_watched(&profile, &ids["Completed"], true)
            .unwrap();
        let home = store.home_catalog(&profile).unwrap();
        assert_eq!(home.continue_watching.len(), 2);
        let legacy = home
            .continue_watching
            .iter()
            .find(|i| i.media.id == ids["Legacy"])
            .unwrap();
        assert!(
            legacy.progress.is_none(),
            "a resume does not supply a measured duration"
        );
        let measured = home
            .continue_watching
            .iter()
            .find(|i| i.media.id == ids["Measured"])
            .unwrap();
        let progress = measured.progress.as_ref().unwrap();
        assert_eq!(
            (progress.position_ms, progress.duration_ms),
            (5_000, 100_000)
        );
        assert!(
            store
                .home_catalog(&other)
                .unwrap()
                .continue_watching
                .is_empty()
        );
        assert_eq!(
            store
                .personal_state(&profile, &ids["Legacy"])
                .unwrap()
                .position_ms,
            3_025_917
        );
        drop(store);
        let mut reopened = Store::open(&path).unwrap();
        assert_eq!(
            reopened
                .home_catalog(&profile)
                .unwrap()
                .continue_watching
                .len(),
            2
        );
    }

    #[test]
    fn actual_container_duration_excludes_trials_without_changing_saved_positions() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("home.sqlite")).unwrap();
        let p = store.create_profile("A").unwrap().id;
        store
            .snapshot(vec![media("Trial.mkv"), media("Resume.mkv")])
            .unwrap();
        let items = store.catalog(&p).unwrap();
        let trial = items
            .iter()
            .find(|m| m.title == "Trial")
            .unwrap()
            .id
            .clone();
        let resume = items
            .iter()
            .find(|m| m.title == "Resume")
            .unwrap()
            .id
            .clone();
        for (id, pos) in [(&trial, 13000), (&resume, 3025917)] {
            store.personal.execute("INSERT INTO cc_profile_state(profile_id,stable_media_id,position_ms) VALUES(?1,?2,?3)",rusqlite::params![p,id,pos]).unwrap();
        }
        assert_eq!(store.home_catalog(&p).unwrap().continue_watching.len(), 2);
        let h = store
            .home_catalog_with_durations(
                &p,
                &HashMap::from([(trial.clone(), 6000000), (resume.clone(), 9000000)]),
            )
            .unwrap();
        assert_eq!(h.continue_watching.len(), 1);
        assert_eq!(h.continue_watching[0].media.id, resume);
        assert_eq!(
            h.continue_watching[0]
                .progress
                .as_ref()
                .unwrap()
                .duration_ms,
            9000000
        );
        assert_eq!(store.personal_state(&p, &trial).unwrap().position_ms, 13000);
        assert!(!store.personal_state(&p, &trial).unwrap().watched);
        assert!(
            store.playback_progress(&p, None).unwrap()["items"]
                .as_array()
                .unwrap()
                .is_empty(),
            "read-only metadata does not fabricate playback events"
        );
    }

    #[test]
    fn season_counts_and_atomic_watched_action_preserve_other_seasons_profiles_and_positions() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(&dir.path().join("counts.sqlite")).unwrap();
        let a = s.create_profile("A").unwrap().id;
        let b = s.create_profile("B").unwrap().id;
        s.snapshot(vec![
            episode("Show.S01E01.mkv", 1, 1, 1),
            episode("Show.S01E02.mkv", 1, 2, 1),
            episode("Show.S02E01.mkv", 2, 1, 1),
        ])
        .unwrap();
        let items = s.catalog(&a).unwrap();
        let first = items
            .iter()
            .find(|m| m.season == Some(1) && m.episode == Some(1))
            .unwrap()
            .id
            .clone();
        let second = items
            .iter()
            .find(|m| m.season == Some(1) && m.episode == Some(2))
            .unwrap()
            .id
            .clone();
        let later = items
            .iter()
            .find(|m| m.season == Some(2))
            .unwrap()
            .id
            .clone();
        s.set_watched(&a, &first, true).unwrap();
        s.personal.execute("INSERT INTO cc_profile_state(profile_id,stable_media_id,position_ms,rating) VALUES(?1,?2,123000,4)",rusqlite::params![a,second]).unwrap();
        let h = s.home_catalog(&a).unwrap();
        let c = h.continue_watching[0].episode_counts.as_ref().unwrap();
        assert_eq!((c.watched, c.total), (1, 2));
        assert!(
            s.set_group_watched(&a, &second, true, "season", h.watch_revision + 1)
                .is_err()
        );
        assert!(!s.personal_state(&a, &second).unwrap().watched);
        let ids = s
            .set_group_watched(&a, &second, true, "season", h.watch_revision)
            .unwrap();
        assert_eq!(ids.len(), 2);
        assert!(s.home_catalog(&a).unwrap().continue_watching.is_empty());
        assert!(!s.personal_state(&a, &later).unwrap().watched);
        assert!(!s.personal_state(&b, &second).unwrap().watched);
        let state = s.personal_state(&a, &second).unwrap();
        assert_eq!((state.position_ms, state.rating), (123000, Some(4)));
        let revision = s.sync_revision(&a).unwrap().0;
        s.set_group_watched(&a, &second, false, "season", revision)
            .unwrap();
        assert!(!s.personal_state(&a, &first).unwrap().watched);
        assert_eq!(s.personal_state(&a, &second).unwrap().position_ms, 123000);
    }

    #[test]
    fn failed_group_write_rolls_back_every_episode() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Store::open(&dir.path().join("atomic.sqlite")).unwrap();
        let p = s.create_profile("A").unwrap().id;
        s.snapshot(vec![
            episode("Show.S01E01.mkv", 1, 1, 1),
            episode("Show.S01E02.mkv", 1, 2, 1),
        ])
        .unwrap();
        let items = s.catalog(&p).unwrap();
        let second = &items.iter().find(|m| m.episode == Some(2)).unwrap().id;
        s.personal.execute_batch(&format!("CREATE TRIGGER refuse_second BEFORE INSERT ON cc_profile_state WHEN NEW.stable_media_id='{second}' BEGIN SELECT RAISE(ABORT,'fixture failure'); END;")).unwrap();
        let rev = s.sync_revision(&p).unwrap().0;
        assert!(
            s.set_group_watched(&p, &items[0].id, true, "season", rev)
                .is_err()
        );
        assert!(
            items
                .iter()
                .all(|m| !s.personal_state(&p, &m.id).unwrap().watched)
        );
    }

    #[test]
    fn recommendations_use_shared_series_metadata_in_both_directions() {
        let mut seed = episode("Show.S01E01.mkv", 1, 1, 1);
        seed.genres.clear();
        seed.cast.clear();
        seed.directors.clear();
        seed.series_metadata = Some(chill_catalog::model::SeriesMetadata {
            year: None,
            genres: vec!["Shared genre".into()],
            overview: String::new(),
            cast: vec!["Shared actor".into()],
            directors: vec!["Shared director".into()],
        });
        let mut candidate = media("Related.mkv");
        candidate.genres = vec!["Shared genre".into()];
        candidate.cast = vec!["Shared actor".into()];
        candidate.directors = vec!["Shared director".into()];
        assert_eq!(matches(&seed, &candidate).len(), 3);
        assert_eq!(matches(&candidate, &seed).len(), 3);
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("meta.sqlite")).unwrap();
        let profile = store.create_profile("A").unwrap().id;
        store.snapshot(vec![seed, candidate]).unwrap();
        let id = store
            .catalog(&profile)
            .unwrap()
            .into_iter()
            .find(|m| m.category == "serie")
            .unwrap()
            .id;
        store.set_watched(&profile, &id, true).unwrap();
        let result = store.home_catalog(&profile).unwrap();
        assert_eq!(result.recommendations[0].media.title, "Related");
        assert_eq!(result.recommendations[0].matches.len(), 3);
    }
    #[test]
    fn started_unknown_season_is_nullable_in_the_home_contract() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("unknown.sqlite")).unwrap();
        let profile = store.create_profile("A").unwrap().id;
        let mut first = episode("Show.E01.mkv", 1, 1, 1);
        first.season = None;
        let mut next = episode("Show.E02.mkv", 1, 2, 2);
        next.season = None;
        store.snapshot(vec![first, next]).unwrap();
        let id = store
            .catalog(&profile)
            .unwrap()
            .into_iter()
            .find(|m| m.episode == Some(1))
            .unwrap()
            .id;
        store.set_watched(&profile, &id, true).unwrap();
        let result = store.home_catalog(&profile).unwrap();
        assert_eq!(result.continue_watching.len(), 1);
        assert!(result.continue_watching[0].season.is_none());
        assert!(result.continue_watching[0].media.season.is_none());
    }

    #[test]
    fn home_groups_series_by_season_and_uses_latest_episode_for_recent() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("home.sqlite")).unwrap();
        let profile = store.create_profile("A").unwrap().id;
        let mut s1e1 = episode("Show.S01E01.mkv", 1, 1, 10);
        s1e1.available = false; // It still counts as the started member.
        let s1e2 = episode("Show.S01E02.mkv", 1, 2, 99);
        let s2e1 = episode("Show.S02E01.mkv", 2, 1, 20);
        store.snapshot(vec![s1e1, s1e2, s2e1]).unwrap();
        let id = store
            .catalog(&profile)
            .unwrap()
            .into_iter()
            .find(|m| m.episode == Some(1) && m.season == Some(1))
            .unwrap()
            .id;
        store.set_watched(&profile, &id, true).unwrap();
        let home = store.home_catalog(&profile).unwrap();
        assert_eq!(home.continue_watching.len(), 1);
        assert_eq!(home.continue_watching[0].season, Some(1));
        assert_eq!(home.continue_watching[0].media.title, "Show");
        assert!(home.continue_watching[0].media.season.is_none());
        assert_eq!(home.recently_added.len(), 1);
        assert_eq!(home.recently_added[0].media.added_at, 99);
    }

    #[test]
    fn home_history_is_profile_scoped_and_recommendations_are_local_similarity() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("home.sqlite")).unwrap();
        let a = store.create_profile("A").unwrap().id;
        let b = store.create_profile("B").unwrap().id;
        let mut watched = media("Watched.mkv");
        watched.genres = vec!["Drama".into()];
        watched.cast = vec!["Actor".into()];
        let mut related = media("Related.mkv");
        related.genres = vec!["Drama".into()];
        related.cast = vec!["Actor".into()];
        let mut unrelated = media("Unrelated.mkv");
        unrelated.genres = vec!["Comedy".into()];
        store.snapshot(vec![watched, related, unrelated]).unwrap();
        let id = store
            .catalog(&a)
            .unwrap()
            .into_iter()
            .find(|m| m.title == "Watched")
            .unwrap()
            .id;
        store.set_watched(&a, &id, true).unwrap();
        store.set_rating(&b, &id, Some(5), "video").unwrap();
        let home_a = store.home_catalog(&a).unwrap();
        assert_eq!(home_a.recommendations[0].media.title, "Related");
        assert!(home_a.recommendations[0].matches.contains(&"Drama".into()));
        let home_b = store.home_catalog(&b).unwrap();
        assert!(home_b.recommendations.is_empty());
        assert!(
            home_a
                .recently_added
                .iter()
                .all(|item| item.media.rating.is_none())
        );
        assert!(
            home_b
                .recently_added
                .iter()
                .any(|item| item.media.id == id && item.media.rating == Some(5))
        );
    }

    #[test]
    fn completed_show_with_untouched_next_season_is_not_continue_but_started_next_is() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("home.sqlite")).unwrap();
        let profile = store.create_profile("A").unwrap().id;
        store
            .snapshot(vec![
                episode("Show.S01E01.mkv", 1, 1, 1),
                episode("Show.S02E01.mkv", 2, 1, 2),
                episode("Show.S02E02.mkv", 2, 2, 3),
            ])
            .unwrap();
        let catalog = store.catalog(&profile).unwrap();
        let s1 = catalog
            .iter()
            .find(|m| m.season == Some(1))
            .unwrap()
            .id
            .clone();
        let s2 = catalog
            .iter()
            .find(|m| m.season == Some(2) && m.episode == Some(1))
            .unwrap()
            .id
            .clone();
        store.set_watched(&profile, &s1, true).unwrap();
        assert!(
            store
                .home_catalog(&profile)
                .unwrap()
                .continue_watching
                .is_empty()
        );
        store.set_watched(&profile, &s2, true).unwrap();
        let home = store.home_catalog(&profile).unwrap();
        assert_eq!(home.continue_watching.len(), 1);
        assert_eq!(home.continue_watching[0].season, Some(2));
    }

    #[test]
    fn manual_history_only_changes_on_real_transition_and_legacy_toggle_clears_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("home.sqlite");
        let mut store = Store::open(&path).unwrap();
        let profile = store.create_profile("A").unwrap().id;
        store.snapshot(vec![media("Movie.mkv")]).unwrap();
        let id = store.catalog(&profile).unwrap()[0].id.clone();
        store.set_watched(&profile, &id, true).unwrap();
        let db = rusqlite::Connection::open(&path).unwrap();
        let first: Option<i64> = db.query_row("SELECT last_watched_at FROM cc_profile_state WHERE profile_id=?1 AND stable_media_id=?2", rusqlite::params![profile,id], |r| r.get(0)).unwrap();
        assert!(first.is_some());
        store.set_watched(&profile, &id, true).unwrap();
        let stable: Option<i64> = db.query_row("SELECT last_watched_at FROM cc_profile_state WHERE profile_id=?1 AND stable_media_id=?2", rusqlite::params![profile,id], |r| r.get(0)).unwrap();
        assert_eq!(stable, first);
        db.execute(
            "UPDATE cc_profile_state SET watched=0 WHERE profile_id=?1 AND stable_media_id=?2",
            rusqlite::params![profile, id],
        )
        .unwrap();
        let cleared: Option<i64> = db.query_row("SELECT last_watched_at FROM cc_profile_state WHERE profile_id=?1 AND stable_media_id=?2", rusqlite::params![profile,id], |r| r.get(0)).unwrap();
        assert_eq!(cleared, None);
        store.set_watched(&profile, &id, true).unwrap();
        let fresh: Option<i64> = db.query_row("SELECT last_watched_at FROM cc_profile_state WHERE profile_id=?1 AND stable_media_id=?2", rusqlite::params![profile,id], |r| r.get(0)).unwrap();
        assert!(fresh.is_some());
    }
}
