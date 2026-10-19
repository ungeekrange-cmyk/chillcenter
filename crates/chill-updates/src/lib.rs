//! Fail-closed component update primitives for Chill Center.
//!
//! This crate deliberately does not know about HTTP administration endpoints or
//! private signing keys. Callers inject the pinned Minisign public key and
//! expose only the bounded enums in this module.

mod archive;
mod install;
mod manifest;
mod scheduler;
mod state;
mod transport;
mod trust;

pub use archive::{ArchiveKind, ExtractionLimits, extract_archive};
pub use install::{ApplyRequest, ApplyResult, InstallHooks, InstallLayout, apply_staged};
pub use manifest::{
    Asset, AssetFormat, Compatibility, Component, DatabaseCompatibility, RendererCompatibility,
    RuntimeCompatibility, SignedManifest, VerificationContext, VerifiedManifest, VersionBounds,
    parse_and_verify_manifest, parse_strict_json,
};
pub use scheduler::{ScheduledAttempt, due_attempts, record_retry};
pub use state::{
    ComponentState, IntentKind, Operation, OperationStatus, PersistentState, Policy, PolicyMode,
    StateStore,
};
pub use transport::{GithubReleaseClient, ReleaseCandidate, TransportLimits};
pub use trust::{AntiRollbackStore, validate_root_owned_file};

use thiserror::Error;

pub const FIXED_GITHUB_OWNER: &str = "ungeekrange-cmyk";
pub const FIXED_GITHUB_REPOSITORY: &str = "chillcenter";

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("security policy refused the operation: {0}")]
    Security(String),
    #[error("compatibility check failed: {0}")]
    Incompatible(String),
    #[error("operation is busy: {0}")]
    Busy(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("archive error: {0}")]
    Archive(String),
    #[error("signature verification failed: {0}")]
    Signature(String),
}

pub type Result<T> = std::result::Result<T, UpdateError>;
