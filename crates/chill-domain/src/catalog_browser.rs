//! SQL-paged grouped catalogue. Result materialisation is bounded to101 rows.
use crate::store::SeriesCounters;
use crate::{
    Store,
    store::{CatalogPage, MAX_CATALOG_PAGE},
};
use chill_catalog::model::Media;
use rusqlite::params;
use serde::Deserialize;
use std::collections::BTreeMap;
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BrowserQuery {
    pub offset: usize,
    pub limit: usize,
    pub category: Option<String>,
    pub q: Option<String>,
    pub genre: Option<String>,
    pub year: Option<i32>,
    pub watched: Option<bool>,
    pub sort: Option<String>,
    pub grouped: bool,
    pub available: Option<bool>,
}
fn db(_: rusqlite::Error) -> String {
    "Erreur de base locale. Vos données n’ont pas été remplacées.".into()
}
impl Store {
    pub fn browse_catalog(&self, profile: &str, q: &BrowserQuery) -> Result<CatalogPage, String> {
        self.require_profile(profile)?;
        let limit = if q.limit == 0 { 48 } else { q.limit };
        if limit > MAX_CATALOG_PAGE
            || q.offset > i64::MAX as usize
            || q.q.as_ref().is_some_and(|v| v.len() > 500)
            || q.genre.as_ref().is_some_and(|v| v.len() > 200)
            || q.category
                .as_deref()
                .is_some_and(|v| !chill_catalog::model::CATEGORIES.contains(&v))
            || q.year
                .is_some_and(|v| v != 0 && !(1850..=2200).contains(&v))
        {
            return Err("Filtre ou page catalogue invalide.".into());
        }
        let order = match q.sort.as_deref() {
            None => "legacy",
            Some("title_asc") => "label COLLATE NOCASE ASC,legacy",
            Some("title_desc") => "label COLLATE NOCASE DESC,legacy",
            Some("year_asc") => "sort_year IS NULL,sort_year ASC,label COLLATE NOCASE,legacy",
            Some("year_desc") => "sort_year IS NULL,sort_year DESC,label COLLATE NOCASE,legacy",
            Some("added_desc") => "added DESC,label COLLATE NOCASE,legacy",
            Some("rating_desc") => "rating IS NULL,rating DESC,label COLLATE NOCASE,legacy",
            _ => return Err("Tri inconnu.".into()),
        };
        let needle =
            q.q.as_deref()
                .unwrap_or("")
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
        // All-series counters are calculated before filters. A search matching an
        // episode therefore still returns its series, not a flat episode card.
        let sql = format!(
            r#"WITH base AS (
   SELECT m.id AS legacy,m.data,i.stable_id,
    json_extract(m.data,'$.category') category,
    CASE WHEN json_extract(m.data,'$.category')='serie' THEN COALESCE(json_extract(m.data,'$.series'),json_extract(m.data,'$.title')) END series_key,
    COALESCE(json_extract(m.data,'$.series_metadata.year'),json_extract(m.data,'$.year')) sort_year,
    COALESCE(json_extract(m.data,'$.series_metadata.genres'),json_extract(m.data,'$.genres'),'[]') genres,
    COALESCE(s.watched,0) seen,s.rating rating,json_extract(m.data,'$.added_at') added
   FROM media m JOIN cc_media_identity i ON i.legacy_id=m.id LEFT JOIN cc_profile_state s ON s.stable_media_id=i.stable_id AND s.profile_id=?1
  ), full_series AS (SELECT series_key,MIN(seen) all_seen,COUNT(DISTINCT COALESCE(json_extract(data,'$.season'),-1)) seasons,COUNT(*) episodes,SUM(1-seen) unwatched FROM base WHERE category='serie' GROUP BY series_key),
  eligible AS (
   SELECT b.*,CASE WHEN ?2 AND category='serie' THEN series_key ELSE json_extract(data,'$.title') END label,
    CASE WHEN ?2 AND category='serie' THEN (SELECT all_seen FROM full_series f WHERE f.series_key=b.series_key) ELSE seen END visible_seen
   FROM base b WHERE (?3 IS NULL OR category=?3)
    AND (?4='' OR json_extract(data,'$.title') LIKE '%'||?4||'%' ESCAPE '\' OR series_key LIKE '%'||?4||'%' ESCAPE '\')
    AND (?5 IS NULL OR EXISTS(SELECT 1 FROM json_each(genres) WHERE value=?5))
    AND (?6 IS NULL OR sort_year=?6 OR (?6=0 AND sort_year IS NULL))
    AND (?7 IS NULL OR json_extract(data,'$.available')=?7)
  ), ranked AS (SELECT *,ROW_NUMBER() OVER(PARTITION BY CASE WHEN ?2 AND category='serie' THEN 'series:'||series_key ELSE stable_id END ORDER BY legacy) n FROM eligible WHERE (?8 IS NULL OR visible_seen=?8))
  SELECT data,visible_seen,COALESCE((SELECT seasons FROM full_series f WHERE f.series_key=ranked.series_key),0),COALESCE((SELECT episodes FROM full_series f WHERE f.series_key=ranked.series_key),0),COALESCE((SELECT unwatched FROM full_series f WHERE f.series_key=ranked.series_key),0) FROM ranked WHERE n=1 ORDER BY {order} LIMIT ?9 OFFSET ?10"#
        );
        let mut stmt = self.personal.prepare(&sql).map_err(db)?;
        let rows = stmt
            .query_map(
                params![
                    profile,
                    q.grouped,
                    q.category,
                    needle,
                    q.genre,
                    q.year,
                    q.available,
                    q.watched,
                    limit as i64 + 1,
                    q.offset as i64
                ],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, bool>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, i64>(4)?,
                    ))
                },
            )
            .map_err(db)?;
        let mut series_counters = BTreeMap::new();
        let mut items = rows
            .map(|r| {
                let (text, seen, seasons, episodes, unwatched) = r.map_err(db)?;
                let media: Media =
                    serde_json::from_str(&text).map_err(|_| "Catalogue local endommagé.")?;
                let mut item = self.profile_view(profile, media)?;
                if q.grouped && item.category == "serie" {
                    item.title = item.series.clone().unwrap_or_else(|| item.title.clone());
                    item.watched = seen;
                    let checked = |value: i64| {
                        u64::try_from(value).map_err(|_| "Compteur de série invalide.".to_string())
                    };
                    series_counters.insert(
                        item.id.clone(),
                        SeriesCounters {
                            seasons: checked(seasons)?,
                            episodes: checked(episodes)?,
                            unwatched: checked(unwatched)?,
                        },
                    );
                    if let Some(metadata) = &item.series_metadata {
                        item.overview = metadata.overview.clone();
                        item.genres = metadata.genres.clone();
                        item.year = metadata.year;
                    } else {
                        item.overview = item.series_overview.clone();
                    }
                }
                Ok(item)
            })
            .collect::<Result<Vec<_>, String>>()?;
        let has_more = items.len() > limit;
        items.truncate(limit);
        series_counters.retain(|id, _| items.iter().any(|item| &item.id == id));
        Ok(CatalogPage {
            items,
            has_more,
            series_counters,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn episode_search_groups_before_paging_and_profile_filters_use_full_series() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("db.sqlite")).unwrap();
        let p = store.create_profile("A").unwrap().id;
        let other = store.create_profile("B").unwrap().id;
        let root = std::path::Path::new("/fixture");
        let mut a = chill_catalog::scanner::parse_file(root, &root.join("One.S01E01.mkv"));
        let mut b = chill_catalog::scanner::parse_file(root, &root.join("One.S01E02.mkv"));
        for m in [&mut a, &mut b] {
            m.category = "serie".into();
            m.series = Some("One".into());
            m.genres = vec!["Comédie".into()];
            m.year = Some(2026);
        }
        a.title = "Premier".into();
        b.title = "Finale".into();
        store.snapshot(vec![a, b]).unwrap();
        let ids = store.catalog(&p).unwrap();
        store.set_watched(&p, &ids[0].id, true).unwrap();
        let q = BrowserQuery {
            grouped: true,
            q: Some("Finale".into()),
            genre: Some("Comédie".into()),
            year: Some(2026),
            watched: Some(false),
            ..Default::default()
        };
        let page = store.browse_catalog(&p, &q).unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].title, "One");
        assert!(!page.items[0].watched);
        let counters = &page.series_counters[&page.items[0].id];
        assert_eq!(
            (counters.seasons, counters.episodes, counters.unwatched),
            (1, 2, 1)
        );
        store.set_watched(&p, &ids[1].id, true).unwrap();
        assert!(store.browse_catalog(&p, &q).unwrap().items.is_empty());
        let other_page = store.browse_catalog(&other, &q).unwrap();
        assert_eq!(other_page.items.len(), 1);
        assert_eq!(
            other_page.series_counters[&other_page.items[0].id].unwatched,
            2
        );
        let flat = store
            .browse_catalog(
                &p,
                &BrowserQuery {
                    limit: 1,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(flat.items.len(), 1);
        assert!(flat.has_more);
        assert!(flat.series_counters.is_empty());
        let literal = store
            .browse_catalog(
                &p,
                &BrowserQuery {
                    q: Some("%".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(literal.items.is_empty());
    }
}
