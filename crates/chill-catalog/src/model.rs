use serde::{Deserialize, Serialize};

pub const CATEGORIES: [&str; 4] = ["film", "serie", "spectacle", "documentaire"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Media {
    pub id: String,
    pub path: String,
    pub title: String,
    pub category: String,
    pub year: Option<i32>,
    pub series: Option<String>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub genres: Vec<String>,
    pub overview: String,
    pub poster: Option<String>,
    #[serde(default)]
    pub poster_custom: bool,
    #[serde(default)]
    pub runtime_minutes: Option<i32>,
    #[serde(default)]
    pub cast: Vec<String>,
    #[serde(default)]
    pub directors: Vec<String>,
    #[serde(default)]
    pub series_overview: String,
    /// Manually corrected series fields, distinct from each episode's metadata.
    #[serde(default)]
    pub series_metadata: Option<SeriesMetadata>,
    #[serde(default)]
    pub series_manual: bool,
    #[serde(default)]
    pub rating: Option<u8>,
    pub saga: Option<String>,
    pub watched: bool,
    pub available: bool,
    pub added_at: i64,
    pub tmdb_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub root: String,
    pub auto_refresh: bool,
    pub theme: String,
    pub player: String,
    pub tmdb_token: String,
    pub tmdb_configured: bool,
    pub codex_path: String,
    pub claude_path: String,
    pub ai_provider: String,
    pub thumbnail_size: i32,
    pub view: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            root: String::new(),
            auto_refresh: true,
            theme: "system".into(),
            player: if std::path::Path::new("/Applications/VLC.app").exists() {
                "VLC".into()
            } else {
                String::new()
            },
            tmdb_token: String::new(),
            tmdb_configured: false,
            codex_path: "codex".into(),
            claude_path: "claude".into(),
            ai_provider: "codex".into(),
            thumbnail_size: 180,
            view: "grid".into(),
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !["system", "dark", "light"].contains(&self.theme.as_str())
            || !["codex", "claude"].contains(&self.ai_provider.as_str())
            || !["grid", "list"].contains(&self.view.as_str())
            || !(100..=360).contains(&self.thumbnail_size)
        {
            return Err("Réglages invalides.".into());
        }
        if !self.root.is_empty() && !std::path::Path::new(&self.root).is_absolute() {
            return Err("Choisissez le chemin absolu du dossier NAS monté sur le Mac.".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genres: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    #[serde(
        default,
        deserialize_with = "nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub season: Option<Option<i32>>,
    #[serde(
        default,
        deserialize_with = "nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub episode: Option<Option<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saga: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_minutes: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cast: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_overview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watched: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeriesMetadata {
    pub year: Option<i32>,
    pub genres: Vec<String>,
    pub overview: String,
    pub cast: Vec<String>,
    pub directors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SeriesProvider {
    pub name: String,
    pub metadata: SeriesMetadata,
    pub tmdb_id: i64,
    pub poster: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeriesPatch {
    pub name: String,
    pub year: i32,
    pub genres: Vec<String>,
    pub overview: String,
    pub cast: Vec<String>,
    pub directors: Vec<String>,
}

impl SeriesPatch {
    pub fn validate(&self, sample: &Media) -> Result<SeriesMetadata, String> {
        if self.name.trim().is_empty() || self.name.len() > 500 {
            return Err("Nom de série invalide.".into());
        }
        let mut checked = sample.clone();
        MediaPatch {
            year: Some(self.year),
            genres: Some(self.genres.clone()),
            overview: Some(self.overview.clone()),
            cast: Some(self.cast.clone()),
            directors: Some(self.directors.clone()),
            ..Default::default()
        }
        .apply(&mut checked)?;
        Ok(SeriesMetadata {
            year: checked.year,
            genres: checked.genres,
            overview: checked.overview,
            cast: checked.cast,
            directors: checked.directors,
        })
    }
}

fn nullable<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<i32>>, D::Error> {
    Option::<i32>::deserialize(d).map(Some)
}

impl MediaPatch {
    pub fn apply(&self, media: &mut Media) -> Result<(), String> {
        if let Some(value) = self.rating {
            if value > 5 {
                return Err("La note doit être comprise entre 1 et 5 étoiles.".into());
            }
            media.rating = (value != 0).then_some(value);
        }
        if let Some(v) = &self.title {
            if v.trim().is_empty() || v.len() > 500 {
                return Err("Titre invalide.".into());
            }
            media.title = v.trim().into();
        }
        if let Some(v) = &self.category {
            if !CATEGORIES.contains(&v.as_str()) {
                return Err("Catégorie inconnue.".into());
            }
            if (media.category == "serie") != (v == "serie") {
                // Movie and TV IDs occupy different TMDB namespaces.
                media.tmdb_id = None;
                media.series_metadata = None;
                media.series_manual = false;
                media.series_overview.clear();
                if !media.poster_custom {
                    media.poster = None;
                }
            }
            media.category = v.clone();
        }
        if let Some(v) = self.year {
            if v != 0 && !(1800..=2200).contains(&v) {
                return Err("Année invalide.".into());
            }
            media.year = (v != 0).then_some(v);
        }
        if let Some(v) = &self.genres {
            if v.len() > 30 || v.iter().any(|g| g.len() > 100) {
                return Err("Genres invalides.".into());
            }
            media.genres = v
                .iter()
                .map(|g| g.trim().to_string())
                .filter(|g| !g.is_empty())
                .collect();
        }
        if let Some(v) = &self.series {
            if v.len() > 500 {
                return Err("Nom de série trop long.".into());
            }
            let next = (!v.trim().is_empty()).then(|| v.trim().to_owned());
            if media.series != next {
                media.series_metadata = None;
                media.series_overview.clear();
                media.series_manual = next.is_some();
            }
            media.series = next;
        }
        if let Some(v) = self.season {
            if v.is_some_and(|n| !(0..=1000).contains(&n)) {
                return Err("Saison invalide.".into());
            }
            media.season = v;
        }
        if let Some(v) = self.episode {
            if v.is_some_and(|n| !(0..=10000).contains(&n)) {
                return Err("Épisode invalide.".into());
            }
            media.episode = v;
        }
        if let Some(v) = &self.saga {
            if v.len() > 200 {
                return Err("Nom de saga trop long.".into());
            }
            media.saga = (!v.trim().is_empty()).then(|| v.trim().into());
        }
        if let Some(v) = &self.overview {
            if v.len() > 20000 {
                return Err("Résumé trop long.".into());
            }
            media.overview = v.clone();
        }
        if let Some(v) = self.watched {
            media.watched = v;
        }
        if let Some(v) = &self.series_overview {
            if v.len() > 20000 {
                return Err("Synopsis de série trop long.".into());
            }
            media.series_overview = v.clone();
        }
        if let Some(v) = self.runtime_minutes {
            if !(0..=10000).contains(&v) {
                return Err("Durée invalide (minutes).".into());
            }
            media.runtime_minutes = (v != 0).then_some(v);
        }
        for (value, destination) in [
            (&self.cast, &mut media.cast),
            (&self.directors, &mut media.directors),
        ] {
            if let Some(names) = value {
                if names.len() > 100 || names.iter().any(|name| name.len() > 200) {
                    return Err("Liste de personnes trop longue.".into());
                }
                *destination = names
                    .iter()
                    .map(|name| name.trim().to_owned())
                    .filter(|name| !name.is_empty())
                    .collect();
            }
        }
        Ok(())
    }
}

/// Rebase downloaded metadata over edits made while the network request was in flight.
pub fn merge_enrichment(original: &Media, latest: &Media, mut downloaded: Media) -> Media {
    macro_rules! preserve {
        ($($field:ident),*) => { $(if latest.$field != original.$field { downloaded.$field = latest.$field.clone(); })* };
    }
    preserve!(
        title,
        category,
        year,
        genres,
        series,
        season,
        episode,
        saga,
        overview,
        poster,
        poster_custom,
        runtime_minutes,
        cast,
        directors,
        series_overview,
        tmdb_id
    );
    if latest.poster_custom {
        downloaded.poster = latest.poster.clone();
        downloaded.poster_custom = true;
    }
    // A previously assigned custom saga is always retained, including an explicit concurrent clear.
    if latest.saga == original.saga && original.saga.is_some() {
        downloaded.saga = original.saga.clone();
    }
    downloaded.watched = latest.watched;
    downloaded.rating = latest.rating;
    downloaded.available = latest.available;
    downloaded.path = latest.path.clone();
    // Provider episode refreshes never replace manual, series-level corrections.
    downloaded.series_metadata = latest.series_metadata.clone();
    downloaded.series_manual = latest.series_manual;
    if latest.series_manual || latest.series_metadata.is_some() {
        downloaded.series = latest.series.clone();
    }
    if latest.series_metadata.is_some() {
        downloaded.series_overview = latest.series_overview.clone();
    }
    downloaded
}

/// An explicit new provider association may replace a prior manual grouping,
/// but never a correction made while that selection was downloading.
pub fn merge_association(original: &Media, latest: &Media, downloaded: Media) -> Media {
    let replace = original.category == "serie"
        && latest.category == original.category
        && downloaded.category == "serie"
        && downloaded.tmdb_id != original.tmdb_id
        && latest.tmdb_id == original.tmdb_id
        && latest.series == original.series
        && latest.series_metadata == original.series_metadata
        && latest.series_overview == original.series_overview
        && latest.series_manual == original.series_manual;
    let provider_series = downloaded.series.clone();
    let provider_overview = downloaded.series_overview.clone();
    let mut merged = merge_enrichment(original, latest, downloaded);
    if replace {
        merged.series = provider_series;
        merged.series_overview = provider_overview;
        merged.series_metadata = None;
        merged.series_manual = false;
    }
    merged
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub media_ids: Vec<String>,
    #[serde(default)]
    pub poster: Option<String>,
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
