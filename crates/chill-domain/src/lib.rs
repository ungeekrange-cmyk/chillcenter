pub mod administration;
pub mod ai;
pub mod catalog_browser;
pub mod fingerprint;
pub mod import;
pub mod legacy;
pub mod metadata_edit;
pub mod reassociation;
pub mod provider_updates;
pub mod snapshot;
pub mod store;
pub mod types;
pub use store::Store;
pub use types::*;

/// Safe, typed boundary for the retained legacy string-result catalogue APIs.
/// The HTTP layer never parses or exposes arbitrary database/provider details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreFailure {
    ProfileNotFound,
    MediaNotFound,
    ProfileConflict,
    ProfileNameTaken,
    ImportPlanStale,
    DatabaseOffline,
    RequestTimeout,
    AiConfirmationRequired,
    AiPlanInvalid,
    InvalidOperation,
}
impl From<String> for StoreFailure {
    fn from(error: String) -> Self {
        match error.as_str() {
            "IMPORT_PLAN_STALE" => Self::ImportPlanStale,
            "PROFILE_NOT_FOUND" | "Profil introuvable." => Self::ProfileNotFound,
            "Vidéo introuvable dans le catalogue." | "Vidéo introuvable." => Self::MediaNotFound,
            "PROFILE_CONFLICT" => Self::ProfileConflict,
            "La fiche a changé entre-temps."
            | "La playlist a changé entre-temps."
            | "La saga a changé entre-temps."
            | "PLAYLIST_NAME_CONFLICT"
            | "La liste a changé entre-temps. Actualisez-la avant de la réorganiser." => {
                Self::ProfileConflict
            }
            "Ce nom de profil est déjà utilisé." => Self::ProfileNameTaken,
            "Erreur de base locale. Vos données n’ont pas été remplacées." => {
                Self::DatabaseOffline
            }
            "REQUEST_TIMEOUT" => Self::RequestTimeout,
            "AI_CONFIRMATION_REQUIRED" => Self::AiConfirmationRequired,
            "AI_CONTEXT_INVALID" | "AI_CONTEXT_TOO_LARGE" | "AI_PLAN_INVALID" | "AI_SAGA_CLEAR_AMBIGUOUS" | "AI_SAGA_FILM_ONLY" | "AI_SAGA_LIMIT" => Self::AiPlanInvalid,
            "AI_COMMON_STALE" | "AI_PERSONAL_STALE" | "AI_MEDIA_STALE" => Self::ProfileConflict,
            _ => Self::InvalidOperation,
        }
    }
}
