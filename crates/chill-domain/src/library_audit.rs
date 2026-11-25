use crate::Store;
use chill_catalog::{database::series_source_key, model::Media};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditKind {
    Duplicates,
    MissingEpisodes,
    Unavailable,
    SingleTitleSagas,
}

#[derive(Serialize)]
pub struct AuditPage<T> {
    pub items: Vec<T>,
    pub has_more: bool,
    pub last_scan: Option<i64>,
}

#[derive(Serialize)]
pub struct AuditMedia {
    pub id: String,
    pub title: String,
    pub relative_path: String,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuditIssue {
    Duplicate {
        media: Vec<AuditMedia>,
        count: usize,
    },
    Missing {
        anchor_id: String,
        series: String,
        season: i32,
        from: i32,
        through: i32,
    },
    Saga {
        id: String,
        name: String,
        revision: u64,
        member: AuditMedia,
    },
}

fn series_identity(media: &Media) -> Option<String> {
    media
        .tmdb_id
        .map(|id| format!("tmdb:tv:{id}"))
        .or_else(|| series_source_key(media))
}

fn window<T>(values: impl IntoIterator<Item = T>, offset: usize, limit: usize) -> (Vec<T>, bool) {
    let mut items: Vec<_> = values.into_iter().skip(offset).take(limit + 1).collect();
    let more = items.len() > limit;
    items.truncate(limit);
    (items, more)
}

struct Season<'a> {
    anchor: &'a Media,
    episodes: BTreeSet<i32>,
}

fn missing_ranges(
    episodes: &BTreeSet<i32>,
    aired: impl IntoIterator<Item = (i32, bool)>,
) -> Vec<(i32, i32)> {
    let mut missing = BTreeSet::new();
    let mut previous = 0;
    for &number in episodes {
        missing.extend(previous + 1..number);
        previous = number;
    }
    for (number, has_aired) in aired {
        if has_aired && !episodes.contains(&number) {
            missing.insert(number);
        } else if !has_aired {
            missing.remove(&number);
        }
    }
    let mut ranges: Vec<(i32, i32)> = Vec::new();
    for number in missing {
        match ranges.last_mut() {
            Some((_, end)) if *end + 1 == number => *end = number,
            _ => ranges.push((number, number)),
        }
    }
    ranges
}

#[derive(Default, Deserialize, Serialize)]
pub struct ScanOutcome {
    pub last_scan_failed: bool,
    pub failed_root: Option<usize>,
}

impl Store {
    pub fn scan_outcome(&self) -> Result<ScanOutcome, String> {
        let data: Option<String> = self
            .personal
            .query_row(
                "SELECT value FROM settings WHERE key='library_audit_scan_outcome'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| "Bilan du scan indisponible.")?;
        data.map(|value| serde_json::from_str(&value).map_err(|_| "Bilan du scan invalide.".into()))
            .transpose()
            .map(|report| report.unwrap_or_default())
    }

    pub fn record_scan_outcome(&self, outcome: &ScanOutcome) -> Result<(), String> {
        let data = serde_json::to_string(outcome).map_err(|_| "Bilan du scan invalide.")?;
        self.personal
            .execute(
                "INSERT INTO settings(key,value) VALUES('library_audit_scan_outcome',?1)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                params![data],
            )
            .map_err(|_| "Enregistrement du bilan impossible.")?;
        Ok(())
    }

    pub fn last_catalog_scan(&self) -> Option<i64> {
        self.catalogue.last_scan()
    }

    fn audit_media(&self, media: &Media) -> Result<AuditMedia, String> {
        Ok(AuditMedia {
            id: self.stable_id(&media.id)?,
            title: media.title.clone(),
            relative_path: media.id.clone(),
        })
    }

    pub fn library_audit(
        &mut self,
        profile: &str,
        kind: AuditKind,
        offset: usize,
        limit: usize,
        today: &str,
    ) -> Result<AuditPage<AuditIssue>, String> {
        self.require_profile(profile)?;
        let limit = limit.clamp(1, 48);
        let last_scan = self.catalogue.last_scan();
        let media = self.catalogue.list()?;
        let (items, has_more) = match kind {
            AuditKind::Duplicates => {
                let mut groups = BTreeMap::<_, Vec<_>>::new();
                for media in &media {
                    let key = match media.category.as_str() {
                        "film" => media.tmdb_id.map(|id| (format!("tmdb:movie:{id}"), 0, 0)),
                        "serie" => series_identity(media)
                            .zip(media.season.zip(media.episode))
                            .map(|(series, (season, episode))| (series, season, episode)),
                        _ => None,
                    };
                    if let Some(key) = key {
                        groups.entry(key).or_default().push(media);
                    }
                }
                let (groups, more) = window(
                    groups.into_values().filter(|rows| rows.len() > 1),
                    offset,
                    limit,
                );
                let issues = groups
                    .into_iter()
                    .map(|rows| {
                        Ok(AuditIssue::Duplicate {
                            count: rows.len(),
                            media: rows
                                .into_iter()
                                .take(48)
                                .map(|m| self.audit_media(m))
                                .collect::<Result<_, _>>()?,
                        })
                    })
                    .collect::<Result<_, String>>()?;
                (issues, more)
            }
            AuditKind::MissingEpisodes => {
                let mut seasons = BTreeMap::new();
                for media in media.iter().filter(|media| media.category == "serie") {
                    let Some((season, episode)) = media.season.zip(media.episode) else {
                        continue;
                    };
                    if !(1..=500).contains(&season) || !(1..=1000).contains(&episode) {
                        continue;
                    }
                    if let Some(series) = series_identity(media) {
                        seasons
                            .entry((series, season))
                            .or_insert_with(|| Season {
                                anchor: media,
                                episodes: BTreeSet::new(),
                            })
                            .episodes
                            .insert(episode);
                    }
                }
                let mut cursor = 0;
                let mut issues = Vec::new();
                'seasons: for ((_, number), season) in seasons {
                    let provider = match season.anchor.tmdb_id {
                        Some(id) => self.season_provider(id, number)?,
                        None => None,
                    };
                    let aired = provider
                        .iter()
                        .flat_map(|data| &data.episode_air_dates)
                        .filter(|(number, _)| (1..=1000).contains(*number))
                        .map(|(&number, date)| (number, date.as_str() <= today));
                    for (from, through) in missing_ranges(&season.episodes, aired) {
                        if cursor >= offset {
                            issues.push(AuditIssue::Missing {
                                anchor_id: self.stable_id(&season.anchor.id)?,
                                series: season
                                    .anchor
                                    .series
                                    .as_ref()
                                    .unwrap_or(&season.anchor.title)
                                    .clone(),
                                season: number,
                                from,
                                through,
                            });
                            if issues.len() > limit {
                                break 'seasons;
                            }
                        }
                        cursor += 1;
                    }
                }
                let more = issues.len() > limit;
                issues.truncate(limit);
                (issues, more)
            }
            AuditKind::SingleTitleSagas => {
                let (sagas, more) = window(
                    self.sagas()?
                        .into_iter()
                        .filter(|saga| saga.media_ids.len() == 1),
                    offset,
                    limit,
                );
                let issues = sagas
                    .into_iter()
                    .map(|saga| {
                        let member = self.media(profile, &saga.media_ids[0])?;
                        Ok(AuditIssue::Saga {
                            id: saga.id,
                            name: saga.name,
                            revision: saga.revision,
                            member: AuditMedia {
                                id: member.id,
                                title: member.title,
                                relative_path: member.path,
                            },
                        })
                    })
                    .collect::<Result<_, String>>()?;
                (issues, more)
            }
            AuditKind::Unavailable => {
                return Err("Utiliser la page des vidéos indisponibles.".into());
            }
        };
        Ok(AuditPage {
            items,
            has_more,
            last_scan,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_ranges_join_gaps_and_aired_episodes_but_keep_existing_numbers() {
        assert_eq!(
            missing_ranges(
                &BTreeSet::from([1, 3, 6]),
                [(3, true), (5, false), (7, true), (8, true)]
            ),
            [(2, 2), (4, 4), (7, 8)]
        );
        assert!(missing_ranges(&BTreeSet::from([1]), [(1, true)]).is_empty());
    }
}
