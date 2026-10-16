//! Read-only recommendations from a client mirror. The historical related API
//! stays unchanged; a series is a catalogue entity, never an episode result.
use crate::Store;
use chill_catalog::model::Media;
use rusqlite::params;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize)]
pub struct Recommendation {
    #[serde(flatten)]
    pub media: Media,
    pub series_key: Option<String>,
    pub matches: Vec<String>,
    #[serde(skip)]
    group_key: String,
}
#[derive(Default)]
pub struct RecommendationQuery {
    pub categories: Vec<String>,
    pub watched: Option<bool>,
    pub available: Option<bool>,
}
fn db(_: rusqlite::Error) -> String {
    "Suggestions locales indisponibles.".into()
}
fn common(a: &[String], b: &[String]) -> Vec<String> {
    let keys: HashSet<_> = a
        .iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    let mut seen = HashSet::new();
    b.iter()
        .filter(|s| {
            let key = s.trim().to_lowercase();
            keys.contains(&key) && seen.insert(key)
        })
        .map(|s| s.trim().to_owned())
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
impl Store {
    pub fn recommendation_catalog(
        &self,
        profile: &str,
        query: &RecommendationQuery,
    ) -> Result<Vec<Recommendation>, String> {
        self.require_profile(profile)?;
        if query.categories.len() > 4
            || query
                .categories
                .iter()
                .any(|c| !chill_catalog::model::CATEGORIES.contains(&c.as_str()))
        {
            return Err("Catégories de conseil invalides.".into());
        }
        self.refresh_series_groups()?;
        // Group before applying watched filters: one watched episode does not
        // make a whole show watched. Availability chooses a playable proxy only.
        let mut statement=self.personal.prepare(r#"WITH base AS (
          SELECT m.data,m.id legacy,i.stable_id,COALESCE(s.watched,0) seen,
          CASE WHEN json_extract(m.data,'$.category')='serie' THEN
            CASE WHEN json_extract(m.data,'$.tmdb_id') IS NOT NULL THEN 'tmdb:tv:'||json_extract(m.data,'$.tmdb_id')
            ELSE COALESCE(g.source_key,'media:'||i.stable_id) END
          ELSE 'video:'||i.stable_id END entity_key
          FROM media m JOIN cc_media_identity i ON i.legacy_id=m.id
          LEFT JOIN cc_series_group g ON g.legacy_id=m.id
          LEFT JOIN cc_profile_state s ON s.stable_media_id=i.stable_id AND s.profile_id=?1
        ), counted AS (SELECT *,MIN(seen) OVER(PARTITION BY entity_key) all_seen FROM base),
        ranked AS (SELECT *,ROW_NUMBER() OVER(PARTITION BY entity_key ORDER BY
          json_extract(data,'$.poster') IS NULL,legacy) n FROM counted
          WHERE (?2='[]' OR json_extract(data,'$.category') IN (SELECT value FROM json_each(?2)))
          AND (?3 IS NULL OR all_seen=?3)
          AND (?4 IS NULL OR json_extract(data,'$.available')=?4))
        SELECT data,entity_key,all_seen FROM ranked WHERE n=1 ORDER BY entity_key LIMIT 10001"#).map_err(db)?;
        let categories =
            serde_json::to_string(&query.categories).map_err(|_| "Catégories invalides.")?;
        let rows = statement
            .query_map(
                params![profile, categories, query.watched, query.available],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, bool>(2)?,
                    ))
                },
            )
            .map_err(db)?;
        let mut result = Vec::new();
        for row in rows {
            let (data, key, seen) = row.map_err(db)?;
            let media: Media =
                serde_json::from_str(&data).map_err(|_| "Catalogue local endommagé.")?;
            let mut media = self.profile_view(profile, media)?;
            if media.category == "serie"
                && media
                    .series
                    .as_ref()
                    .is_none_or(|name| name.trim().is_empty())
            {
                continue; // Do not disguise an ungrouped/unnamed episode as a show.
            }
            if media.category != "serie" && (media.season.is_some() || media.episode.is_some()) {
                continue;
            }
            let series_key = if media.category == "serie" {
                common_series(&mut media);
                media.title = media
                    .series
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| media.title.clone());
                media.season = None;
                media.episode = None;
                media.watched = seen;
                media.rating = None;
                Some(
                    media
                        .tmdb_id
                        .map(|tv| format!("tmdb:tv:{tv}"))
                        .unwrap_or_else(|| format!("media:{}", media.id)),
                )
            } else {
                None
            };
            result.push(Recommendation {
                media,
                series_key,
                matches: vec![],
                group_key: key,
            });
        }
        if result.len() > 10_000 {
            return Err("Catalogue de conseil trop volumineux.".into());
        }
        Ok(result)
    }
    pub fn recommendations(
        &mut self,
        profile: &str,
        id: &str,
    ) -> Result<Vec<Recommendation>, String> {
        let mut source = self.media(profile, id)?;
        common_series(&mut source);
        self.refresh_series_groups()?;
        let source_key: Option<String> = if source.category == "serie" {
            Some(self.personal.query_row("SELECT CASE WHEN json_extract(m.data,'$.tmdb_id') IS NOT NULL THEN 'tmdb:tv:'||json_extract(m.data,'$.tmdb_id') ELSE COALESCE(g.source_key,'media:'||i.stable_id) END FROM media m JOIN cc_media_identity i ON i.legacy_id=m.id LEFT JOIN cc_series_group g ON g.legacy_id=m.id WHERE i.stable_id=?1",[id],|r|r.get(0)).map_err(db)?)
        } else {
            None
        };
        let mut scored = Vec::new();
        for mut item in self.recommendation_catalog(
            profile,
            &RecommendationQuery {
                available: Some(true),
                ..Default::default()
            },
        )? {
            if item.media.id == id || source_key.as_deref() == Some(item.group_key.as_str()) {
                continue;
            }
            let mut score = 0;
            for (label, values) in [
                ("Genres", common(&source.genres, &item.media.genres)),
                (
                    "Réalisation",
                    common(&source.directors, &item.media.directors),
                ),
                ("Distribution", common(&source.cast, &item.media.cast)),
            ] {
                score += values.len();
                if !values.is_empty() {
                    item.matches
                        .push(format!("{label} : {}", values.join(", ")));
                }
            }
            if score > 0 {
                scored.push((score, item));
            }
        }
        // Year proximity or saga membership alone are not metadata matches.
        scored.sort_by(|(sa, a), (sb, b)| {
            sb.cmp(sa)
                .then_with(|| {
                    a.media
                        .title
                        .to_lowercase()
                        .cmp(&b.media.title.to_lowercase())
                })
                .then_with(|| a.media.id.cmp(&b.media.id))
        });
        Ok(scored.into_iter().take(60).map(|(_, item)| item).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn media(file: &str) -> Media {
        chill_catalog::scanner::parse_file(
            std::path::Path::new("/fixture"),
            &std::path::Path::new("/fixture").join(file),
        )
    }
    #[test]
    fn related_matches_people_and_genres_not_year_and_groups_homonymous_series() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("db.sqlite")).unwrap();
        let p = store.create_profile("A").unwrap().id;
        let mut source = media("Source.mkv");
        source.directors = vec!["Director".into()];
        source.cast = vec!["Actor".into()];
        source.genres = vec!["Drama".into()];
        source.year = Some(2024);
        let mut cast = media("Cast.mkv");
        cast.cast = vec![" actor ".into(), "ACTOR".into()];
        let mut director = media("Director.mkv");
        director.directors = vec!["director".into()];
        let mut highest = media("Highest.mkv");
        highest.cast = source.cast.clone();
        highest.directors = source.directors.clone();
        highest.genres = source.genres.clone();
        let mut year = media("Year.mkv");
        year.year = Some(2024);
        let mut unnamed = media("Unknown.S01E01.mkv");
        unnamed.category = "serie".into();
        unnamed.series = None;
        unnamed.genres = vec!["Drama".into()];
        let mut episodes = vec![];
        for (file, tv) in [
            ("Old/Same.S01E01.mkv", 1),
            ("Old/Same.S01E02.mkv", 1),
            ("New/Same.S01E01.mkv", 2),
        ] {
            let mut m = media(file);
            m.category = "serie".into();
            m.series = Some("Same".into());
            m.tmdb_id = Some(tv);
            m.genres = vec!["Drama".into()];
            episodes.push(m);
        }
        store
            .snapshot(
                [
                    vec![source, cast, year, director, highest, unnamed],
                    episodes,
                ]
                .concat(),
            )
            .unwrap();
        let all = store.catalog(&p).unwrap();
        let id = all.iter().find(|m| m.title == "Source").unwrap().id.clone();
        let result = store.recommendations(&p, &id).unwrap();
        assert_eq!(result.len(), 5);
        assert_eq!(result[0].media.title, "Highest");
        assert!(
            result
                .iter()
                .any(|r| r.matches == ["Réalisation : director"])
        );
        assert!(!result.iter().any(|r| r.media.title == "Year"));
        let shows: Vec<_> = result
            .iter()
            .filter(|r| r.media.category == "serie")
            .collect();
        assert_eq!(shows.len(), 2);
        assert_ne!(shows[0].series_key, shows[1].series_key);
        assert!(shows.iter().all(|r| r.media.title == "Same"
            && r.media.episode.is_none()
            && r.media.season.is_none()));
        assert!(result.iter().any(|r| r.matches == ["Distribution : actor"]));
        let first = all.iter().find(|m| m.tmdb_id == Some(1)).unwrap();
        assert!(
            store
                .recommendations(&p, &first.id)
                .unwrap()
                .iter()
                .all(|r| r.series_key.as_deref() != Some("tmdb:tv:1"))
        );
    }
    #[test]
    fn grouped_watched_filter_is_profile_scoped_and_uses_series_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("db.sqlite")).unwrap();
        let p = store.create_profile("A").unwrap().id;
        let other = store.create_profile("B").unwrap().id;
        let mut a = media("Show.S01E01.mkv");
        let mut b = media("Show.S01E02.mkv");
        for m in [&mut a, &mut b] {
            m.category = "serie".into();
            m.series = Some("Show".into());
            m.series_metadata = Some(chill_catalog::model::SeriesMetadata {
                year: Some(2020),
                genres: vec!["Genre".into()],
                overview: String::new(),
                cast: vec!["Cast".into()],
                directors: vec!["Director".into()],
            });
        }
        store.snapshot(vec![a, b]).unwrap();
        let ids = store.catalog(&p).unwrap();
        store.set_watched(&p, &ids[0].id, true).unwrap();
        let query = RecommendationQuery {
            categories: vec!["serie".into()],
            watched: Some(false),
            available: Some(true),
        };
        let result = store.recommendation_catalog(&p, &query).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].media.directors, ["Director"]);
        assert_eq!(result[0].media.cast, ["Cast"]);
        assert!(
            result[0]
                .series_key
                .as_deref()
                .unwrap()
                .starts_with("media:")
        );
        assert!(
            store.recommendations(&p, &ids[0].id).unwrap().is_empty(),
            "another episode of the same unknown show is never a recommendation"
        );
        assert!(
            store.media(&p, &ids[0].id).unwrap().episode.is_some(),
            "grouping never rewrites episode metadata"
        );
        store.set_watched(&p, &ids[1].id, true).unwrap();
        assert!(store.recommendation_catalog(&p, &query).unwrap().is_empty());
        assert_eq!(
            store.recommendation_catalog(&other, &query).unwrap().len(),
            1
        );
    }
}
