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
}
#[derive(Serialize)]
pub struct HomeCatalog {
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
fn matches(seed: &Media, candidate: &Media) -> Vec<String> {
    let mut out = common(&seed.genres, &candidate.genres);
    out.extend(common(&seed.cast, &candidate.cast));
    out.extend(common(&seed.directors, &candidate.directors));
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
        }
    }
    pub fn home_catalog(&mut self, profile: &str) -> Result<HomeCatalog, String> {
        self.require_profile(profile)?;
        self.refresh_series_groups()?;
        // One bounded projection: never call profile_view (several SQL reads) for each raw row.
        let mut statement=self.personal.prepare(r#"SELECT m.data,i.stable_id,i.relative_path,COALESCE(s.watched,0),COALESCE(s.position_ms,0),d.duration_ms,s.last_watched_at,CASE WHEN json_extract(m.data,'$.category')='serie' THEN CASE WHEN json_extract(m.data,'$.tmdb_id') IS NOT NULL THEN 'tmdb:tv:'||json_extract(m.data,'$.tmdb_id') ELSE COALESCE(g.source_key,'media:'||i.stable_id) END ELSE 'video:'||i.stable_id END FROM media m JOIN cc_media_identity i ON i.legacy_id=m.id LEFT JOIN cc_profile_state s ON s.profile_id=?1 AND s.stable_media_id=i.stable_id LEFT JOIN cc_resume_measurement d ON d.profile_id=?1 AND d.stable_media_id=i.stable_id LEFT JOIN cc_series_group g ON g.legacy_id=m.id ORDER BY m.id LIMIT 10001"#).map_err(|_|"Accueil indisponible.".to_string())?;
        let values = statement
            .query_map([profile], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, bool>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, Option<i64>>(5)?,
                    r.get::<_, Option<i64>>(6)?,
                    r.get::<_, String>(7)?,
                ))
            })
            .map_err(|_| "Accueil indisponible.".to_string())?;
        let mut all = Vec::new();
        for value in values {
            let (data, stable, relative_path, watched, position, duration, last, key) =
                value.map_err(|_| "Accueil indisponible.".to_string())?;
            all.push(Row {
                media: serde_json::from_str(&data)
                    .map_err(|_| "Accueil indisponible.".to_string())?,
                stable,
                relative_path,
                watched,
                position: position.max(0) as u64,
                duration: duration.map(|v| v.max(0) as u64),
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
                if let Some(row) = rows.iter().copied().find(|r| {
                    r.media.available && !r.watched && r.position > 0 && r.duration.unwrap_or(0) > 0
                }) {
                    continuing.push((
                        row.position,
                        self.home_item(
                            profile,
                            row,
                            None,
                            None,
                            Some(HomeProgress {
                                position_ms: row.position,
                                duration_ms: row.duration.unwrap(),
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
                let started = members.iter().any(|r| r.watched || r.position > 0);
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
                    continuing.push((
                        proxy.position,
                        self.home_item(
                            profile,
                            proxy,
                            Some(key.clone()),
                            Some(season),
                            progress,
                            vec![],
                        ),
                    ));
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
            continue_watching,
            recently_added,
            recommendations,
        })
    }
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
        let home_a = store.home_catalog(&a).unwrap();
        assert_eq!(home_a.recommendations[0].media.title, "Related");
        assert!(home_a.recommendations[0].matches.contains(&"Drama".into()));
        assert!(store.home_catalog(&b).unwrap().recommendations.is_empty());
    }
}
