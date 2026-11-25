use crate::Store;
use chill_catalog::{database::series_source_key, model::Media};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Film,
    Series,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Discovery {
    Familiar,
    Different,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mood {
    Light,
    Intimate,
    Spectacle,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DuoQuery {
    pub guest_profile_id: String,
    pub format: Option<Format>,
    pub minutes: Option<u32>,
    pub discovery: Option<Discovery>,
    pub mood: Option<Mood>,
}

#[derive(Serialize)]
pub struct DuoSuggestion {
    pub media: Media,
    pub watched_by_one: bool,
    pub minutes: Option<u32>,
    pub reason: &'static str,
    pub position_ms: u64,
}

#[derive(Default)]
struct History {
    watched: bool,
    rating: Option<u8>,
    duration_ms: Option<u64>,
}

fn genres(media: &Media) -> &[String] {
    if media.category == "serie"
        && let Some(metadata) = media.series_metadata.as_ref()
        && !metadata.genres.is_empty()
    {
        return &metadata.genres;
    }
    &media.genres
}

fn add_genres(taste: &mut BTreeMap<String, u32>, media: &Media, watched: bool, rating: Option<u8>) {
    let weight = u32::from(watched) + u32::from(rating.unwrap_or_default().saturating_sub(3)) * 2;
    if weight == 0 {
        return;
    }
    for name in genres(media) {
        let score = taste.entry(name.trim().to_lowercase()).or_default();
        *score = score.saturating_add(weight).min(50);
    }
}

fn affinity(taste: &BTreeMap<String, u32>, media: &Media) -> u32 {
    genres(media)
        .iter()
        .map(|name| {
            taste
                .get(&name.trim().to_lowercase())
                .copied()
                .unwrap_or_default()
        })
        .max()
        .unwrap_or_default()
}

fn mood_fits(mood: Mood, media: &Media) -> bool {
    let wanted: &[&str] = match mood {
        Mood::Light => &["comedy", "comédie", "family", "familial", "animation"],
        Mood::Intimate => &["drama", "drame", "romance", "documentary", "documentaire"],
        Mood::Spectacle => &[
            "action",
            "adventure",
            "aventure",
            "science fiction",
            "science-fiction",
            "fantasy",
            "fantastique",
        ],
    };
    genres(media)
        .iter()
        .any(|name| wanted.contains(&name.trim().to_lowercase().as_str()))
}

impl Store {
    fn duo_history(&self, profile: &str) -> Result<BTreeMap<String, History>, String> {
        self.require_profile(profile)?;
        let db = |_| "Historique de recommandation indisponible.".to_string();
        let mut query = self.personal.prepare(
            "SELECT s.stable_media_id,s.watched,s.rating,d.duration_ms
             FROM cc_profile_state s LEFT JOIN cc_resume_measurement d USING(profile_id,stable_media_id)
             WHERE s.profile_id=?1"
        ).map_err(db)?;
        query
            .query_map([profile], |row| {
                Ok((
                    row.get(0)?,
                    History {
                        watched: row.get(1)?,
                        rating: row.get(2)?,
                        duration_ms: row
                            .get::<_, Option<i64>>(3)?
                            .and_then(|value| u64::try_from(value).ok()),
                    },
                ))
            })
            .map_err(db)?
            .collect::<Result<_, _>>()
            .map_err(db)
    }

    pub fn duo_recommendations(
        &mut self,
        profile: &str,
        query: &DuoQuery,
    ) -> Result<Vec<DuoSuggestion>, String> {
        if profile == query.guest_profile_id
            || query
                .minutes
                .is_some_and(|minutes| !(10..=360).contains(&minutes))
        {
            return Err("Choix de séance à deux invalide.".into());
        }
        let owner = self.duo_history(profile)?;
        let guest = self.duo_history(&query.guest_profile_id)?;
        let library = self.catalog(profile)?;
        let mut first = BTreeMap::new();
        let mut second = BTreeMap::new();
        let mut groups: BTreeMap<String, Vec<&Media>> = BTreeMap::new();
        let mut tasted = [BTreeSet::new(), BTreeSet::new()];
        for media in &library {
            let grouped = if media.category == "serie" {
                media
                    .tmdb_id
                    .map(|id| format!("tv:{id}"))
                    .or_else(|| series_source_key(media))
            } else if media.category == "film" {
                Some(format!("film:{}", media.id))
            } else {
                None
            };
            let Some(key) = grouped else { continue };
            for (index, (history, taste)) in [(&owner, &mut first), (&guest, &mut second)]
                .into_iter()
                .enumerate()
            {
                if let Some(state) = history.get(&media.id)
                    && (state.watched || state.rating.is_some_and(|rating| rating >= 4))
                    && tasted[index].insert(key.clone())
                {
                    add_genres(taste, media, state.watched, state.rating);
                }
            }
            let in_format = match query.format {
                None => true,
                Some(Format::Series) => media.category == "serie",
                Some(Format::Film) => media.category == "film",
            };
            if media.available && in_format {
                groups.entry(key).or_default().push(media);
            }
        }
        let mut ranked = Vec::new();
        for (_, mut group) in groups {
            let seen_by = |media: &Media| {
                u8::from(owner.get(&media.id).is_some_and(|s| s.watched))
                    + u8::from(guest.get(&media.id).is_some_and(|s| s.watched))
            };
            group.sort_by_key(|item| {
                (
                    seen_by(item),
                    item.season.unwrap_or(i32::MAX),
                    item.episode.unwrap_or(i32::MAX),
                    &item.id,
                )
            });
            let Some(media) = group.into_iter().find(|item| seen_by(item) < 2) else {
                continue;
            };
            let watched_by_one = seen_by(media) == 1;
            let measured = owner
                .get(&media.id)
                .and_then(|state| state.duration_ms)
                .or_else(|| guest.get(&media.id).and_then(|state| state.duration_ms))
                .filter(|ms| *ms > 0)
                .and_then(|ms| u32::try_from(ms.div_ceil(60_000)).ok());
            let minutes = measured.or_else(|| {
                media
                    .runtime_minutes
                    .and_then(|n| u32::try_from(n).ok())
                    .filter(|n| *n > 0)
            });
            if query
                .minutes
                .zip(minutes)
                .is_some_and(|(budget, duration)| duration > budget)
            {
                continue;
            }
            let (mut a, mut b) = (affinity(&first, media), affinity(&second, media));
            let familiar = a > 0 && b > 0;
            if matches!(query.discovery, Some(Discovery::Different)) {
                a = 50 - a;
                b = 50 - b;
            }
            let mood = query.mood.is_some_and(|value| mood_fits(value, media));
            let reason = if mood {
                "mood"
            } else if familiar {
                "shared_genres"
            } else {
                "discovery"
            };
            ranked.push((
                (
                    u8::from(minutes.is_none() && query.minutes.is_some()),
                    seen_by(media),
                    Reverse(mood),
                    Reverse(a.min(b)),
                    Reverse(a + b),
                    Reverse(media.added_at),
                ),
                media,
                minutes,
                watched_by_one,
                reason,
            ));
        }
        ranked.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.id.cmp(&right.1.id))
        });
        ranked
            .into_iter()
            .take(9)
            .map(|(_, media, minutes, watched_by_one, reason)| {
                Ok(DuoSuggestion {
                    media: media.clone(),
                    watched_by_one,
                    minutes,
                    reason,
                    position_ms: self.playback_start_state(profile, &media.id)?.position_ms,
                })
            })
            .collect()
    }
}
