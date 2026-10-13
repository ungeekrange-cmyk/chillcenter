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
    DatabaseOffline,
    RequestTimeout,
    InvalidOperation,
}
impl From<String> for StoreFailure {
    fn from(error: String) -> Self {
        match error.as_str() {
            "PROFILE_NOT_FOUND" | "Profil introuvable." => Self::ProfileNotFound,
            "Vidéo introuvable dans le catalogue." | "Vidéo introuvable." => Self::MediaNotFound,
            "PROFILE_CONFLICT" => Self::ProfileConflict,
            "Ce nom de profil est déjà utilisé." => Self::ProfileNameTaken,
            "Erreur de base locale. Vos données n’ont pas été remplacées." => {
                Self::DatabaseOffline
            }
            "REQUEST_TIMEOUT" => Self::RequestTimeout,
            _ => Self::InvalidOperation,
        }
    }
}
