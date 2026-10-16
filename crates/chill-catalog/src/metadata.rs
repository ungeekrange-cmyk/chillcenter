use crate::model::{Media, SeasonProvider, SeriesPatch, SeriesProvider};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{future::Future, path::Path, time::Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub id: i64,
    pub title: String,
    pub year: Option<i32>,
    pub overview: String,
    pub poster: Option<String>,
    pub kind: String,
}

/// Provider candidates together with the query path that produced them.
/// This is internal job context, not part of the HTTP candidate response.
#[derive(Debug, Clone)]
pub struct CandidateSearch {
    pub candidates: Vec<Candidate>,
    pub fallback_used: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtworkCandidate {
    pub title: String,
    pub source: String,
    pub poster_path: String,
    pub poster: String,
}

fn image_path_valid(path: &str) -> bool {
    let Some(leaf) = path.strip_prefix('/') else {
        return false;
    };
    path.len() <= 200
        && leaf.ends_with(".jpg")
        && leaf.len() > 4
        && leaf
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
        && !leaf.contains("..")
}

fn artwork_results(results: &Value, source: &str, output: &mut Vec<ArtworkCandidate>) {
    for r in results["results"].as_array().into_iter().flatten().take(20) {
        if r["adult"].as_bool() == Some(true) || text(r, "media_type") == "person" {
            continue;
        }
        let title = r["title"]
            .as_str()
            .or_else(|| r["name"].as_str())
            .unwrap_or("");
        if title.is_empty() || title.len() > 500 {
            continue;
        }
        for key in ["poster_path", "backdrop_path"] {
            if output.len() >= 24 {
                return;
            }
            let Some(path) = r[key].as_str().filter(|p| image_path_valid(p)) else {
                continue;
            };
            if output.iter().any(|item| item.poster_path == path) {
                continue;
            }
            output.push(ArtworkCandidate {
                title: title.to_owned(),
                source: format!(
                    "{source} · {}",
                    if key == "poster_path" {
                        "affiche"
                    } else {
                        "visuel"
                    }
                ),
                poster_path: path.to_owned(),
                poster: format!("https://image.tmdb.org/t/p/w342{path}"),
            });
        }
    }
}

pub struct Tmdb {
    client: reqwest::Client,
    credential: Credential,
}
enum Credential {
    Bearer(String),
    ApiKey(String),
}
/// Safe failure class for bounded background retries. No request URL, response
/// body or credential is retained in an error.
#[derive(Debug, Clone)]
pub struct ProviderFailure {
    pub retryable: bool,
    message: String,
}
impl ProviderFailure {
    fn transient(message: &str) -> Self {
        Self {
            retryable: true,
            message: message.into(),
        }
    }
}
impl From<&str> for ProviderFailure {
    fn from(message: &str) -> Self {
        Self {
            retryable: false,
            message: message.into(),
        }
    }
}
impl From<String> for ProviderFailure {
    fn from(message: String) -> Self {
        Self {
            retryable: false,
            message,
        }
    }
}
impl From<ProviderFailure> for String {
    fn from(error: ProviderFailure) -> Self {
        error.message
    }
}
async fn body_limited(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>, ProviderFailure> {
    if response.content_length().is_some_and(|n| n > limit as u64) {
        return Err("Fiche TMDB trop volumineuse.".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ProviderFailure::transient("Réponse TMDB interrompue."))?
    {
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err("Réponse TMDB trop volumineuse.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}
fn search_query(media: &Media, query: &str) -> String {
    if !query.trim().is_empty() {
        query.trim().to_owned()
    } else {
        let label = if media.category == "serie" {
            media.series.as_deref().unwrap_or(&media.title)
        } else {
            &media.title
        };
        crate::scanner::title_for_search(label)
    }
}

/// Plan at most one fallback lookup for automatic filename-derived queries.
/// Manual text is intentional and must never be rewritten or retried.
fn search_queries(media: &Media, query: &str) -> Vec<String> {
    if !query.trim().is_empty() {
        return vec![query.trim().to_owned()];
    }
    let label = if media.category == "serie" {
        media.series.as_deref().unwrap_or(&media.title)
    } else {
        &media.title
    };
    let primary = search_query(media, query);
    let fallback = crate::scanner::title_for_search_fallback(label);
    let mut queries = vec![primary];
    if let Some(fallback) = fallback.filter(|fallback| fallback != &queries[0]) {
        queries.push(fallback);
    }
    queries
}

fn candidates_from_results(results: &Value, kind: &str) -> Vec<Candidate> {
    results["results"]
        .as_array()
        .into_iter()
        .flatten()
        .take(10)
        .filter_map(|r| {
            Some(Candidate {
                id: r["id"].as_i64()?,
                title: text(r, if kind == "tv" { "name" } else { "title" }).into(),
                year: text(
                    r,
                    if kind == "tv" {
                        "first_air_date"
                    } else {
                        "release_date"
                    },
                )
                .get(..4)
                .and_then(|y| y.parse().ok()),
                overview: text(r, "overview").into(),
                poster: r["poster_path"]
                    .as_str()
                    .map(|p| format!("https://image.tmdb.org/t/p/w342{p}")),
                kind: kind.into(),
            })
        })
        .collect()
}

/// Select a provider candidate only on a unique exact automatic title match.
/// If the provider reports that its bounded fallback query was used, the
/// fallback label is considered only after there is no primary exact match.
/// This keeps a duplicate or mismatched primary result ambiguous.
fn exact_matches(media: &Media, candidates: &[Candidate], label: &str) -> Vec<Candidate> {
    let kind = if media.category == "serie" { "tv" } else { "movie" };
    let normalized = |value: &str| {
        value
            .chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    candidates
        .iter()
        .filter(|candidate| {
            candidate.kind == kind
                && normalized(&candidate.title) == normalized(label)
                && media.year.is_none_or(|year| candidate.year == Some(year))
        })
        .cloned()
        .collect()
}

pub fn exact_automatic_candidate(media: &Media, search: &CandidateSearch) -> Option<Candidate> {
    let source_label = media.series.as_deref().unwrap_or(&media.title);
    let primary = crate::scanner::title_for_search(source_label);
    let primary_matches = exact_matches(media, &search.candidates, &primary);
    if !primary_matches.is_empty() {
        return (primary_matches.len() == 1).then(|| primary_matches.into_iter().next().unwrap());
    }
    if !search.fallback_used {
        return None;
    }
    let fallback = crate::scanner::title_for_search_fallback(source_label)?;
    let matches = exact_matches(media, &search.candidates, &fallback);
    (matches.len() == 1).then(|| matches.into_iter().next().unwrap())
}

async fn candidate_search_with<F, Fut>(
    media: &Media,
    kind: &str,
    queries: Vec<String>,
    fallback_delay: Duration,
    mut fetch: F,
) -> Result<CandidateSearch, ProviderFailure>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<Value, ProviderFailure>>,
{
    let has_fallback = queries.len() > 1;
    for (index, query) in queries.into_iter().enumerate() {
        if index > 0 && !fallback_delay.is_zero() {
            tokio::time::sleep(fallback_delay).await;
        }
        let candidates = candidates_from_results(&fetch(query).await?, kind);
        if index == 0 {
            let primary = crate::scanner::title_for_search(
                media.series.as_deref().unwrap_or(&media.title),
            );
            // A unique primary match wins, and multiple exact primary matches
            // remain ambiguous. Only zero exact matches permit the one retry.
            if !exact_matches(media, &candidates, &primary).is_empty() {
                return Ok(CandidateSearch {
                    candidates,
                    fallback_used: false,
                });
            }
            if has_fallback {
                continue;
            }
        }
        return Ok(CandidateSearch {
            candidates,
            fallback_used: index > 0,
        });
    }
    Ok(CandidateSearch {
        candidates: Vec::new(),
        fallback_used: false,
    })
}

fn names(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut result = Vec::new();
    for name in values {
        let name = name.trim();
        if !name.is_empty() && name.len() <= 200 && !result.iter().any(|n| n == name) {
            result.push(name.to_owned());
        }
        if result.len() == 100 {
            break;
        }
    }
    result
}

fn presentation(media: &mut Media, details: &Value, episode: Option<&Value>, kind: &str) {
    if kind == "tv" {
        media.series_overview = text(details, "overview").to_owned();
    }
    let source = episode.unwrap_or(details);
    media.runtime_minutes = if kind == "movie" || episode.is_some() {
        source["runtime"]
            .as_i64()
            .filter(|n| (1..=10000).contains(n))
            .map(|n| n as i32)
    } else {
        None
    };
    let episode_cast = source["credits"]["cast"]
        .as_array()
        .filter(|a| !a.is_empty());
    let cast = episode_cast.or_else(|| details["credits"]["cast"].as_array());
    media.cast = names(
        cast.into_iter()
            .flatten()
            .chain(source["guest_stars"].as_array().into_iter().flatten())
            .filter_map(|person| person["name"].as_str().map(str::to_owned)),
    );
    // TV creators are not necessarily episode directors; do not label them as such.
    media.directors = names(
        source["credits"]["crew"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|person| text(person, "job") == "Director")
            .filter_map(|person| person["name"].as_str().map(str::to_owned)),
    );
}

impl Tmdb {
    pub fn new(token: String) -> Result<Self, String> {
        let token = token.trim();
        if token.is_empty() {
            return Err("Ajoutez votre jeton de lecture TMDB dans les réglages.".into());
        }
        let credential = if let Some(token) = token.strip_prefix("Bearer ") {
            if valid_bearer(token) {
                Credential::Bearer(token.into())
            } else {
                return Err("Identifiant TMDB invalide.".into());
            }
        } else if valid_v3_key(token) {
            Credential::ApiKey(token.into())
        } else if valid_bearer(token) {
            Credential::Bearer(token.into())
        } else {
            return Err("Identifiant TMDB invalide.".into());
        };
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!(
                "Cinefolio/",
                env!("CARGO_PKG_VERSION"),
                " (local media catalog)"
            ))
            .build()
            .map_err(|_| "Client TMDB indisponible.")?;
        Ok(Self { client, credential })
    }

    async fn get(&self, endpoint: &str, query: &[(&str, &str)]) -> Result<Value, String> {
        self.get_checked(endpoint, query)
            .await
            .map_err(String::from)
    }
    async fn get_checked(
        &self,
        endpoint: &str,
        query: &[(&str, &str)],
    ) -> Result<Value, ProviderFailure> {
        let request = self
            .client
            .get(format!("https://api.themoviedb.org/3/{endpoint}"));
        let request = match &self.credential {
            Credential::Bearer(token) => request.bearer_auth(token),
            Credential::ApiKey(key) => request.query(&[("api_key", key)]),
        };
        let response = request
            .query(&[("language", "fr-FR")])
            .query(query)
            .send()
            .await
            .map_err(|_| {
                ProviderFailure::transient(
                    "TMDB inaccessible. Les métadonnées précédentes sont conservées.",
                )
            })?;
        if response.status().as_u16() == 401 {
            return Err(
                "Jeton TMDB refusé. Vérifiez le jeton de lecture API dans les réglages.".into(),
            );
        }
        if !response.status().is_success() {
            return Err(http_failure(
                response.status().as_u16(),
                "TMDB n’a pas pu répondre. Réessayez plus tard.",
            ));
        }
        serde_json::from_slice(&body_limited(response, 2 * 1024 * 1024).await?)
            .map_err(|_| ProviderFailure::from("Réponse TMDB invalide."))
    }

    pub async fn candidates(&self, media: &Media, query: &str) -> Result<Vec<Candidate>, String> {
        self.candidates_for_job(media, query)
            .await
            .map_err(String::from)
    }
    pub async fn candidates_for_job(
        &self,
        media: &Media,
        query: &str,
    ) -> Result<Vec<Candidate>, ProviderFailure> {
        Ok(self
            .candidates_for_job_with_context(media, query)
            .await?
            .candidates)
    }
    pub async fn candidates_for_job_with_context(
        &self,
        media: &Media,
        query: &str,
    ) -> Result<CandidateSearch, ProviderFailure> {
        let kind = if media.category == "serie" {
            "tv"
        } else {
            "movie"
        };
        let queries = search_queries(media, query);
        if queries.iter().any(|query| query.is_empty() || query.len() > 500) {
            return Err("Recherche TMDB trop longue.".into());
        }
        candidate_search_with(media, kind, queries, Duration::from_secs(1), |query| async move {
            self.get_checked(
                &format!("search/{kind}"),
                &[("query", query.as_str()), ("include_adult", "false")],
            )
            .await
        })
        .await
    }

    pub async fn artwork_candidates(&self, query: &str) -> Result<Vec<ArtworkCandidate>, String> {
        let query = query.trim();
        if query.is_empty() || query.len() > 500 {
            return Err("Saisissez un thème de 500 caractères maximum.".into());
        }
        let params = [("query", query), ("include_adult", "false")];
        let keyword_params = [("query", query)];
        let (collections, titles, keywords) = tokio::try_join!(
            self.get("search/collection", &params),
            self.get("search/multi", &params),
            self.get("search/keyword", &keyword_params),
        )?;
        let mut output = Vec::new();
        artwork_results(&collections, "Collection TMDB", &mut output);
        artwork_results(&titles, "Film / série TMDB", &mut output);
        let ids = keywords["results"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|k| k["id"].as_i64().filter(|id| *id > 0))
            .take(3)
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join("|");
        if !ids.is_empty() && output.len() < 24 {
            let params = [("with_keywords", ids.as_str()), ("include_adult", "false")];
            let (movies, series) = tokio::try_join!(
                self.get("discover/movie", &params),
                self.get("discover/tv", &params),
            )?;
            artwork_results(&movies, "Thème TMDB", &mut output);
            artwork_results(&series, "Thème TMDB", &mut output);
        }
        Ok(output)
    }

    pub async fn download_artwork(
        &self,
        poster_path: &str,
        cache: &Path,
    ) -> Result<String, String> {
        self.download_artwork_checked(poster_path, cache)
            .await
            .map_err(String::from)
    }
    async fn download_artwork_checked(
        &self,
        poster_path: &str,
        cache: &Path,
    ) -> Result<String, ProviderFailure> {
        if !image_path_valid(poster_path) {
            return Err("Image TMDB invalide.".into());
        }
        let response = self
            .client
            .get(format!("https://image.tmdb.org/t/p/w342{poster_path}"))
            .send()
            .await
            .map_err(|_| {
                ProviderFailure::transient(
                    "Image TMDB inaccessible. L’image précédente est conservée.",
                )
            })?;
        if !response.status().is_success() {
            return Err(http_failure(
                response.status().as_u16(),
                "Image TMDB indisponible.",
            ));
        }
        let bytes = body_limited(response, 8 * 1024 * 1024).await?;
        crate::poster::store(&bytes, cache).map_err(ProviderFailure::from)
    }

    /// Fetch a provider poster without refreshing any metadata.  The fixed
    /// TMDB image host and `download_artwork` validation are intentionally
    /// shared with saga artwork selection.
    pub async fn restore_poster(
        &self,
        kind: &str,
        id: i64,
        cache: &Path,
    ) -> Result<String, String> {
        if id <= 0 || !["movie", "tv"].contains(&kind) {
            return Err("Identifiant TMDB invalide.".into());
        }
        let details = self.get(&format!("{kind}/{id}"), &[]).await?;
        let path = details["poster_path"]
            .as_str()
            .ok_or("TMDB n’a pas fourni d’affiche. Votre affiche personnelle est conservée.")?;
        self.download_artwork(path, cache).await
    }

    pub async fn enrich(
        &self,
        media: Media,
        id: i64,
        kind: &str,
        cache: &Path,
    ) -> Result<Media, String> {
        self.enrich_checked(media, id, kind, cache, false)
            .await
            .map_err(String::from)
    }
    pub async fn enrich_for_job(
        &self,
        media: Media,
        id: i64,
        kind: &str,
        cache: &Path,
    ) -> Result<Media, ProviderFailure> {
        self.enrich_checked(media, id, kind, cache, true).await
    }
    async fn enrich_checked(
        &self,
        mut media: Media,
        id: i64,
        kind: &str,
        cache: &Path,
        strict_artwork: bool,
    ) -> Result<Media, ProviderFailure> {
        if id <= 0 || !["movie", "tv"].contains(&kind) {
            return Err("Identifiant TMDB invalide.".into());
        }
        if (media.category == "serie") != (kind == "tv") {
            return Err("Cette correspondance ne convient pas à la catégorie de la vidéo.".into());
        }
        let details = self
            .get_checked(
                &format!("{kind}/{id}"),
                &[("append_to_response", "credits")],
            )
            .await?;
        let name = text(&details, if kind == "tv" { "name" } else { "title" });
        if name.is_empty() {
            return Err("Fiche TMDB sans titre.".into());
        }
        if kind == "tv" {
            media.series = Some(name.into());
        } else {
            media.title = name.into();
        }
        media.year = text(
            &details,
            if kind == "tv" {
                "first_air_date"
            } else {
                "release_date"
            },
        )
        .get(..4)
        .and_then(|y| y.parse().ok());
        media.overview = text(&details, "overview").into();
        media.genres = details["genres"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|g| g["name"].as_str().map(str::to_owned))
            .collect();
        if let Some(saga) = details["belongs_to_collection"]["name"].as_str() {
            media.saga = Some(saga.to_owned());
        }
        media.tmdb_id = Some(id);
        let mut episode_details = None;
        if kind == "tv"
            && let (Some(season), Some(episode)) = (media.season, media.episode)
        {
            // An unavailable episode does not discard valid series metadata.
            if let Ok(ep) = self
                .get_checked(
                    &format!("tv/{id}/season/{season}/episode/{episode}"),
                    &[("append_to_response", "credits")],
                )
                .await
            {
                if !text(&ep, "name").is_empty() {
                    media.title = text(&ep, "name").into();
                }
                if !text(&ep, "overview").is_empty() {
                    media.overview = text(&ep, "overview").into();
                }
                episode_details = Some(ep);
            }
        }
        presentation(&mut media, &details, episode_details.as_ref(), kind);
        if !media.poster_custom {
            media.poster = None;
        }
        if !media.poster_custom
            && let Some(poster_path) = details["poster_path"].as_str()
        {
            // A changed association must not keep an unrelated old TMDB poster.
            // Only TMDB's fixed image host; no URL or filesystem path supplied by AI.
            match self.download_artwork_checked(poster_path, cache).await {
                Ok(poster) => media.poster = Some(poster),
                Err(error) if strict_artwork => return Err(error),
                Err(_) => {}
            }
        }
        Ok(media)
    }

    pub async fn best(&self, media: &Media) -> Result<Option<Candidate>, String> {
        let search = self.candidates_for_job_with_context(media, "").await?;
        Ok(exact_automatic_candidate(media, &search))
    }

    /// Fetch only the TV series, never episode-specific data.
    pub async fn series_metadata(
        &self,
        sample: &Media,
        id: i64,
        cache: &Path,
    ) -> Result<SeriesProvider, String> {
        let mut series = sample.clone();
        series.category = "serie".into();
        series.season = None;
        series.episode = None;
        series.poster_custom = false;
        series.poster = None;
        let enriched = self.enrich(series, id, "tv", cache).await?;
        series_provider(&enriched)
    }

    /// Fetch a season only after the user has selected the parent TV show.
    /// TMDB has no standalone season search endpoint; accepting arbitrary text
    /// here would silently associate a season of a homonymous show.
    pub async fn season_metadata(
        &self,
        tv_id: i64,
        season: i32,
        cache: &Path,
    ) -> Result<SeasonProvider, String> {
        if tv_id <= 0 || !(0..=500).contains(&season) {
            return Err("Identifiant de saison TMDB invalide.".into());
        }
        let details = self
            .get_checked(&format!("tv/{tv_id}/season/{season}"), &[])
            .await
            .map_err(String::from)?;
        let name = text(&details, "name").trim();
        if name.is_empty() {
            return Err("Saison TMDB sans titre.".into());
        }
        let poster = match details["poster_path"].as_str() {
            Some(path) => Some(
                self.download_artwork_checked(path, cache)
                    .await
                    .map_err(String::from)?,
            ),
            None => None,
        };
        Ok(SeasonProvider {
            tv_id,
            season,
            name: name.into(),
            overview: text(&details, "overview").into(),
            poster,
        })
    }
}

fn http_failure(status: u16, message: &str) -> ProviderFailure {
    if status == 408 || status == 429 || (500..=599).contains(&status) {
        ProviderFailure::transient(message)
    } else {
        ProviderFailure::from(message)
    }
}

fn valid_v3_key(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_bearer(value: &str) -> bool {
    (20..=2048).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && !byte.is_ascii_whitespace())
}

#[cfg(test)]
mod credential_tests {
    use super::*;
    #[test]
    fn job_retry_classes_follow_http_status_without_retaining_response_details() {
        for status in [408, 429, 500, 503] {
            assert!(http_failure(status, "safe").retryable);
        }
        for status in [400, 401, 403, 404, 413] {
            assert!(!http_failure(status, "safe").retryable);
        }
        assert!(!ProviderFailure::from("Invalid provider payload").retryable);
    }

    #[test]
    fn accepts_explicit_tmdb_credential_shapes_without_retaining_the_input() {
        assert!(Tmdb::new("aabbccddeeff00112233445566778899".into()).is_ok());
        assert!(Tmdb::new(format!("Bearer {}", "x".repeat(64))).is_ok());
        assert!(Tmdb::new("not a credential".into()).is_err());
    }
}

fn series_provider(enriched: &Media) -> Result<SeriesProvider, String> {
    let name = enriched.series.clone().ok_or("Fiche de série sans nom.")?;
    let tmdb_id = enriched
        .tmdb_id
        .filter(|id| *id > 0)
        .ok_or("Identifiant de série invalide.")?;
    let patch = SeriesPatch {
        name: name.clone(),
        year: enriched.year.unwrap_or(0),
        genres: enriched.genres.clone(),
        overview: enriched.series_overview.clone(),
        cast: enriched.cast.clone(),
        directors: enriched.directors.clone(),
    };
    Ok(SeriesProvider {
        name,
        metadata: patch.validate(enriched)?,
        tmdb_id,
        poster: enriched.poster.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn theme_artwork_is_bounded_deduplicated_and_rejects_people_adult_and_unsafe_paths() {
        let mut output = Vec::new();
        let results = serde_json::json!({"results":[
            {"name":"Marvel Cinematic Universe","poster_path":"/marvel.jpg","backdrop_path":"/marvel-wide.jpg"},
            {"title":"Film","poster_path":"/marvel.jpg"},
            {"name":"Person","media_type":"person","poster_path":"/person.jpg"},
            {"title":"Adult","adult":true,"poster_path":"/adult.jpg"},
            {"name":"Unsafe","poster_path":"/../evil.jpg","backdrop_path":"//evil.jpg"}
        ]});
        artwork_results(&results, "Thème TMDB", &mut output);
        assert_eq!(output.len(), 2);
        assert_eq!(output[0].title, "Marvel Cinematic Universe");
        assert_eq!(
            output[0].poster,
            "https://image.tmdb.org/t/p/w342/marvel.jpg"
        );
        assert_eq!(output[1].poster_path, "/marvel-wide.jpg");
        let many = serde_json::json!({"results":(0..30).map(|i|serde_json::json!({"name":"Film","poster_path":format!("/a{i}.jpg"),"backdrop_path":format!("/b{i}.jpg")})).collect::<Vec<_>>()});
        artwork_results(&many, "Film", &mut output);
        assert_eq!(output.len(), 24);
        for bad in [
            "https://evil/x.jpg",
            "/x.jpg?token=x",
            "/x%2f.jpg",
            "/x#y.jpg",
            "/x.png",
            "/x/evil.jpg",
            "/..jpg",
            "/.jpg",
        ] {
            assert!(!image_path_valid(bad), "accepted {bad}");
        }
        assert!(!image_path_valid(&format!("/{}.jpg", "x".repeat(200))));
    }
    fn movie() -> Media {
        crate::scanner::parse_file(Path::new("/nas"), Path::new("/nas/Alien.mkv"))
    }
    #[test]
    fn explicit_query_can_correct_a_series_association() {
        let mut media = movie();
        media.category = "serie".into();
        media.series = Some("Wrong association".into());
        assert_eq!(search_query(&media, " New association "), "New association");
        assert_eq!(search_query(&media, ""), "Wrong association");
    }
    #[test]
    fn automatic_query_cleans_legacy_release_titles_but_manual_query_is_not_rewritten() {
        let mut media = movie();
        for (dirty, expected) in [
            ("Attack the Block - - Vosta", "Attack the Block"),
            ("Birdman () - vost DDP 5 1", "Birdman"),
            ("Blade Runner 2049", "Blade Runner 2049"),
        ] {
            media.title = dirty.into();
            assert_eq!(search_query(&media, ""), expected);
            assert_eq!(search_query(&media, dirty), dirty);
        }
    }
    #[test]
    fn automatic_filename_lookup_has_one_conservative_fallback_but_manual_lookup_has_none() {
        let mut media = movie();
        media.title = "Attack the Block - - (2011) - - Audio - VOSTA".into();
        assert_eq!(
            search_queries(&media, ""),
            [
                "Attack the Block (2011)".to_owned(),
                "Attack the Block".to_owned()
            ]
        );
        assert_eq!(search_queries(&media, "Attack the Block"), ["Attack the Block"]);

        media.title = "Spider-Man - No Way Home (2021) - 1080p".into();
        assert_eq!(
            search_queries(&media, ""),
            ["Spider-Man - No Way Home (2021)"]
        );
    }
    #[test]
    fn candidates_keep_provider_years_for_exact_year_disambiguation() {
        let candidates = candidates_from_results(
            &serde_json::json!({"results":[
                {"id": 1, "title":"Doctor Who", "release_date":"2005-03-26"},
                {"id": 2, "title":"Doctor Who", "release_date":"2023-05-06"}
            ]}),
            "movie",
        );
        assert_eq!(candidates[0].year, Some(2005));
        assert_eq!(candidates[1].year, Some(2023));
    }
    #[tokio::test]
    async fn automatic_lookup_retries_only_after_no_primary_exact_match() {
        let mut media = movie();
        media.title = "Attack the Block -- (2011) -- Audio -- VOSTA".into();
        media.year = Some(2011);
        let result = |title: &str| {
            serde_json::json!({"results":[{
                "id": 1, "title": title, "release_date": "2011-01-01"
            }]})
        };

        let mut calls = Vec::new();
        let mut responses = vec![result("Unrelated"), result("Attack the Block")].into_iter();
        let search = candidate_search_with(
            &media,
            "movie",
            search_queries(&media, ""),
            Duration::ZERO,
            |query| {
                calls.push(query);
                std::future::ready(Ok(responses.next().unwrap()))
            },
        )
        .await
        .unwrap();
        assert_eq!(calls, ["Attack the Block (2011)", "Attack the Block"]);
        assert!(search.fallback_used);
        assert_eq!(
            exact_automatic_candidate(&media, &search).map(|candidate| candidate.title),
            Some("Attack the Block".into())
        );

        let primary = "Attack the Block (2011)";
        let mut calls = Vec::new();
        let duplicates = serde_json::json!({"results":[
            {"id": 1, "title": primary, "release_date": "2011-01-01"},
            {"id": 2, "title": primary, "release_date": "2011-01-01"}
        ]});
        let search = candidate_search_with(
            &media,
            "movie",
            search_queries(&media, ""),
            Duration::ZERO,
            |query| {
                calls.push(query);
                std::future::ready(Ok(duplicates.clone()))
            },
        )
        .await
        .unwrap();
        assert_eq!(calls, [primary]);
        assert!(!search.fallback_used);
        assert!(exact_automatic_candidate(&media, &search).is_none());

        let mut calls = Vec::new();
        let search = candidate_search_with(
            &media,
            "movie",
            search_queries(&media, ""),
            Duration::ZERO,
            |query| {
                calls.push(query);
                std::future::ready(Ok(result(primary)))
            },
        )
        .await
        .unwrap();
        assert_eq!(calls, [primary]);
        assert!(!search.fallback_used);
        assert!(exact_automatic_candidate(&media, &search).is_some());

        let mut calls = Vec::new();
        let _ = candidate_search_with(
            &media,
            "movie",
            search_queries(&media, "Manual choice"),
            Duration::ZERO,
            |query| {
                calls.push(query);
                std::future::ready(Ok(result("Unrelated")))
            },
        )
        .await
        .unwrap();
        assert_eq!(calls, ["Manual choice"]);
    }
    #[test]
    fn movie_presentation_reads_runtime_cast_and_only_directors() {
        let mut media = movie();
        let details = serde_json::json!({"runtime":117,"credits":{"cast":[{"name":"Sigourney Weaver"},{"name":"Sigourney Weaver"}],"crew":[{"name":"Ridley Scott","job":"Director"},{"name":"Someone","job":"Producer"}]}});
        presentation(&mut media, &details, None, "movie");
        assert_eq!(media.runtime_minutes, Some(117));
        assert_eq!(media.cast, ["Sigourney Weaver"]);
        assert_eq!(media.directors, ["Ridley Scott"]);
    }
    #[test]
    fn episode_uses_its_own_runtime_and_director_with_series_cast_fallback() {
        let mut media = movie();
        let details = serde_json::json!({"overview":"Whole series synopsis","episode_run_time":[60],"credits":{"cast":[{"name":"Series actor"}],"crew":[{"name":"Series director","job":"Director"}]}});
        let episode = serde_json::json!({"runtime":42,"guest_stars":[{"name":"Guest"}],"credits":{"crew":[{"name":"Episode director","job":"Director"}]}});
        presentation(&mut media, &details, Some(&episode), "tv");
        assert_eq!(media.runtime_minutes, Some(42));
        assert_eq!(media.cast, ["Series actor", "Guest"]);
        assert_eq!(media.directors, ["Episode director"]);
        assert_eq!(media.series_overview, "Whole series synopsis");
        presentation(&mut media, &details, None, "tv");
        assert_eq!(media.runtime_minutes, None);
    }
    #[test]
    fn absent_or_invalid_presentation_is_not_invented() {
        let mut media = movie();
        presentation(
            &mut media,
            &serde_json::json!({"runtime":-1}),
            None,
            "movie",
        );
        assert_eq!(media.runtime_minutes, None);
        assert!(media.cast.is_empty());
        assert!(media.directors.is_empty());
    }

    #[test]
    fn series_provider_uses_shared_synopsis_and_rejects_invalid_fields() {
        let mut media = movie();
        media.category = "serie".into();
        media.series = Some("Dark".into());
        media.tmdb_id = Some(42);
        media.overview = "Episode summary".into();
        media.series_overview = "Whole series".into();
        media.cast = vec!["Actor".into()];
        let p = series_provider(&media).unwrap();
        assert_eq!(p.name, "Dark");
        assert_eq!(p.metadata.overview, "Whole series");
        assert_eq!(p.metadata.cast, ["Actor"]);
        media.series = Some("x".repeat(501));
        assert!(series_provider(&media).is_err());
    }
}
