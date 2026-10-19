use crate::{Result, UpdateError};
use minisign_verify::{PublicKey, Signature};
use semver::Version;
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fmt;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const MAX_SIGNATURE_BYTES: usize = 16 * 1024;
const MAX_ASSETS: usize = 32;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "lowercase")]
pub enum Component {
    Mac,
    Server,
    Web,
    Themes,
}

impl Component {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mac => "mac",
            Self::Server => "server",
            Self::Web => "web",
            Self::Themes => "themes",
        }
    }
}

impl std::str::FromStr for Component {
    type Err = UpdateError;
    fn from_str(value: &str) -> Result<Self> {
        match value {
            "mac" => Ok(Self::Mac),
            "server" => Ok(Self::Server),
            "web" => Ok(Self::Web),
            "themes" => Ok(Self::Themes),
            _ => Err(UpdateError::Invalid("unknown component".into())),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VersionBounds {
    pub min: Version,
    pub max: Version,
}

impl VersionBounds {
    pub fn contains(&self, version: &Version) -> bool {
        self.min <= *version && *version <= self.max
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseCompatibility {
    pub current_schema: u32,
    pub target_schema: u32,
    pub read_min: u32,
    pub read_max: u32,
    pub write_min: u32,
    pub write_max: u32,
    pub rollback_min: u32,
    pub rollback_max: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RendererCompatibility {
    pub theme_format: u32,
    pub renderer_min: u32,
    pub renderer_max: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Compatibility {
    pub os_arch: Vec<String>,
    #[serde(default)]
    pub api: Option<VersionBounds>,
    #[serde(default)]
    pub required_server: Option<VersionBounds>,
    #[serde(default)]
    pub database: Option<DatabaseCompatibility>,
    #[serde(default)]
    pub renderer: Option<RendererCompatibility>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetFormat {
    Raw,
    Zip,
    Tar,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub target: String,
    pub url: String,
    pub byte_length: u64,
    pub sha256: String,
    pub format: AssetFormat,
    /// Base64 of the complete Minisign signature text expected by the Tauri
    /// updater. Required for Mac assets, forbidden for non-Mac assets.
    #[serde(default)]
    pub tauri_signature: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignedManifest {
    pub schema_version: u32,
    pub component: Component,
    pub version: Version,
    pub release_sequence: u64,
    pub release_tag: String,
    pub source_commit: String,
    pub issued_at: String,
    pub expires_at: String,
    pub key_id: String,
    pub compatibility: Compatibility,
    pub assets: Vec<Asset>,
}

#[derive(Clone, Debug)]
pub struct RuntimeCompatibility {
    pub api_version: Option<Version>,
    pub server_version: Option<Version>,
    pub database_schema: Option<u32>,
    pub renderer_version: Option<u32>,
    pub theme_format: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct VerificationContext<'a> {
    pub now: OffsetDateTime,
    pub expected_key_id: &'a str,
    pub component: Component,
    pub target: &'a str,
    pub installed_version: &'a Version,
    pub highest_sequence: u64,
    pub runtime: RuntimeCompatibility,
}

#[derive(Clone, Debug)]
pub struct VerifiedManifest {
    pub manifest: SignedManifest,
    pub selected_asset: Asset,
}

/// Verify the signature over the exact bytes before interpreting any manifest data.
pub fn parse_and_verify_manifest(
    bytes: &[u8],
    signature_text: &str,
    public_key_text: &str,
    context: &VerificationContext<'_>,
) -> Result<VerifiedManifest> {
    if bytes.is_empty() || bytes.len() > MAX_MANIFEST_BYTES {
        return Err(UpdateError::Invalid(
            "manifest size is outside bounds".into(),
        ));
    }
    if signature_text.is_empty() || signature_text.len() > MAX_SIGNATURE_BYTES {
        return Err(UpdateError::Invalid(
            "signature size is outside bounds".into(),
        ));
    }
    let key = PublicKey::decode(public_key_text)
        .map_err(|error| UpdateError::Signature(error.to_string()))?;
    let signature = Signature::decode(signature_text)
        .map_err(|error| UpdateError::Signature(error.to_string()))?;
    key.verify(bytes, &signature, false)
        .map_err(|error| UpdateError::Signature(error.to_string()))?;

    let manifest: SignedManifest = parse_strict_json(bytes)?;
    validate_manifest(manifest, context)
}

pub fn parse_strict_json<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = StrictValueSeed.deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(serde_json::from_value(value)?)
}

fn validate_manifest(
    manifest: SignedManifest,
    context: &VerificationContext<'_>,
) -> Result<VerifiedManifest> {
    if manifest.schema_version != 1 {
        return Err(UpdateError::Invalid("unsupported manifest schema".into()));
    }
    if manifest.component != context.component {
        return Err(UpdateError::Security("component substitution".into()));
    }
    if manifest.key_id != context.expected_key_id || manifest.key_id.len() > 128 {
        return Err(UpdateError::Security("unknown release key id".into()));
    }
    let expected_tag = format!("{}-v{}", manifest.component.as_str(), manifest.version);
    if manifest.release_tag != expected_tag {
        return Err(UpdateError::Invalid(
            "release tag does not bind component/version".into(),
        ));
    }
    if manifest.source_commit.len() != 40
        || !manifest
            .source_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(UpdateError::Invalid(
            "source_commit must be a lowercase full SHA-1".into(),
        ));
    }
    if manifest.version <= *context.installed_version {
        return Err(UpdateError::Security(
            "downgrade or reinstall refused".into(),
        ));
    }
    if manifest.release_sequence <= context.highest_sequence {
        return Err(UpdateError::Security(
            "release sequence replay refused".into(),
        ));
    }
    let issued = OffsetDateTime::parse(&manifest.issued_at, &Rfc3339)
        .map_err(|_| UpdateError::Invalid("issued_at is not RFC3339".into()))?;
    let expires = OffsetDateTime::parse(&manifest.expires_at, &Rfc3339)
        .map_err(|_| UpdateError::Invalid("expires_at is not RFC3339".into()))?;
    if issued > context.now + time::Duration::minutes(10)
        || expires <= context.now
        || expires <= issued
    {
        return Err(UpdateError::Security(
            "manifest time window is not acceptable".into(),
        ));
    }
    validate_compatibility(&manifest.compatibility, context)?;
    if manifest.assets.is_empty() || manifest.assets.len() > MAX_ASSETS {
        return Err(UpdateError::Invalid("asset count is outside bounds".into()));
    }
    let mut targets = BTreeSet::new();
    for asset in &manifest.assets {
        if !targets.insert(&asset.target) {
            return Err(UpdateError::Invalid("duplicate asset target".into()));
        }
        validate_asset(asset, &manifest.release_tag)?;
        match manifest.component {
            Component::Mac
                if asset
                    .tauri_signature
                    .as_ref()
                    .is_none_or(|value| value.is_empty() || value.len() > 16 * 1024) =>
            {
                return Err(UpdateError::Invalid(
                    "Mac asset requires a bounded Tauri signature".into(),
                ));
            }
            Component::Mac => {}
            _ if asset.tauri_signature.is_some() => {
                return Err(UpdateError::Invalid(
                    "Tauri signature is only valid for Mac assets".into(),
                ));
            }
            _ => {}
        }
    }
    let selected_asset = manifest
        .assets
        .iter()
        .find(|asset| asset.target == context.target)
        .cloned()
        .ok_or_else(|| UpdateError::Incompatible("release has no matching target".into()))?;
    Ok(VerifiedManifest {
        manifest,
        selected_asset,
    })
}

fn validate_compatibility(value: &Compatibility, context: &VerificationContext<'_>) -> Result<()> {
    if value.os_arch.len() > 16 || !value.os_arch.iter().any(|target| target == context.target) {
        return Err(UpdateError::Incompatible(
            "OS/architecture is not supported".into(),
        ));
    }
    if let Some(bounds) = &value.api {
        let api = context.runtime.api_version.as_ref().ok_or_else(|| {
            UpdateError::Incompatible("API version is required but unavailable".into())
        })?;
        if !bounds.contains(api) {
            return Err(UpdateError::Incompatible(
                "API version is outside supported range".into(),
            ));
        }
    }
    if let Some(bounds) = &value.required_server {
        let server = context.runtime.server_version.as_ref().ok_or_else(|| {
            UpdateError::Incompatible("server version is required but unavailable".into())
        })?;
        if !bounds.contains(server) {
            return Err(UpdateError::Incompatible(
                "server prerequisite is not installed".into(),
            ));
        }
    }
    if let Some(database) = &value.database {
        let current = context.runtime.database_schema.ok_or_else(|| {
            UpdateError::Incompatible("database schema is required but unavailable".into())
        })?;
        if current != database.current_schema
            || current < database.read_min
            || current > database.read_max
            || current < database.write_min
            || current > database.write_max
            || database.target_schema < database.rollback_min
            || database.target_schema > database.rollback_max
        {
            return Err(UpdateError::Incompatible(
                "database read/write/rollback gate failed".into(),
            ));
        }
        // Automatic update is deliberately binary-only. A reviewed migration uses a
        // separate manual process, never this helper.
        if database.target_schema != current {
            return Err(UpdateError::Incompatible(
                "database migration requires explicit manual review".into(),
            ));
        }
    }
    if let Some(renderer) = &value.renderer {
        let installed_renderer = context.runtime.renderer_version.ok_or_else(|| {
            UpdateError::Incompatible("renderer version is required but unavailable".into())
        })?;
        let format = context.runtime.theme_format.ok_or_else(|| {
            UpdateError::Incompatible("theme format is required but unavailable".into())
        })?;
        if format != renderer.theme_format
            || installed_renderer < renderer.renderer_min
            || installed_renderer > renderer.renderer_max
        {
            return Err(UpdateError::Incompatible(
                "theme renderer gate failed".into(),
            ));
        }
    }
    Ok(())
}

fn validate_asset(asset: &Asset, release_tag: &str) -> Result<()> {
    if asset.target.is_empty() || asset.target.len() > 128 || asset.byte_length == 0 {
        return Err(UpdateError::Invalid("invalid asset metadata".into()));
    }
    if asset.sha256.len() != 64 || hex::decode(&asset.sha256).is_err() {
        return Err(UpdateError::Invalid("asset SHA-256 is invalid".into()));
    }
    let url = reqwest::Url::parse(&asset.url)
        .map_err(|_| UpdateError::Invalid("asset URL is invalid".into()))?;
    let prefix = format!(
        "/{}/{}/releases/download/{release_tag}/",
        crate::FIXED_GITHUB_OWNER,
        crate::FIXED_GITHUB_REPOSITORY
    );
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || !url.path().starts_with(&prefix)
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(UpdateError::Security(
            "asset URL is outside the fixed GitHub release".into(),
        ));
    }
    Ok(())
}

/// serde_json normally accepts the last occurrence of a duplicate key. This
/// seed rejects duplicates recursively before typed deserialization.
struct StrictValueSeed;

impl<'de> DeserializeSeed<'de> for StrictValueSeed {
    type Value = Value;
    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictValueVisitor)
    }
}

struct StrictValueVisitor;

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = Value;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON without duplicate object keys")
    }
    fn visit_bool<E: de::Error>(self, value: bool) -> std::result::Result<Value, E> {
        Ok(Value::Bool(value))
    }
    fn visit_i64<E: de::Error>(self, value: i64) -> std::result::Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_u64<E: de::Error>(self, value: u64) -> std::result::Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_f64<E: de::Error>(self, value: f64) -> std::result::Result<Value, E> {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }
    fn visit_str<E: de::Error>(self, value: &str) -> std::result::Result<Value, E> {
        Ok(Value::String(value.into()))
    }
    fn visit_string<E: de::Error>(self, value: String) -> std::result::Result<Value, E> {
        Ok(Value::String(value))
    }
    fn visit_none<E: de::Error>(self) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_unit<E: de::Error>(self) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_some<D: Deserializer<'de>>(self, d: D) -> std::result::Result<Value, D::Error> {
        StrictValueSeed.deserialize(d)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element_seed(StrictValueSeed)? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> std::result::Result<Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate JSON key: {key}")));
            }
            values.insert(key, map.next_value_seed(StrictValueSeed)?);
        }
        Ok(Value::Object(values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_parser_rejects_nested_duplicate_keys() {
        let input = br#"{"a":{"x":1,"x":2}}"#;
        assert!(
            StrictValueSeed
                .deserialize(&mut serde_json::Deserializer::from_slice(input))
                .is_err()
        );
    }

    #[test]
    fn official_synthetic_minisign_vector_verifies_exact_bytes() {
        // Public example vector from minisign-verify. It is test-only and no
        // corresponding private key is shipped or generated by this project.
        let public =
            PublicKey::from_base64("RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3")
                .unwrap();
        let signature = Signature::decode("untrusted comment: signature from minisign secret key\nRWQf6LRCGA9i59SLOFxz6NxvASXDJeRtuZykwQepbDEGt87ig1BNpWaVWuNrm73YiIiJbq71Wi+dP9eKL8OC351vwIasSSbXxwA=\ntrusted comment: timestamp:1555779966\tfile:test\nQtKMXWyYcwdpZAlPF7tE2ENJkRd1ujvKjlj1m9RtHTBnZPa5WKU5uWRs5GoP5M/VqE81QFuMKI5k/SfNQUaOAA==").unwrap();
        // The upstream example predates prehashed signatures, so legacy mode is
        // enabled only for this fixture. Production verification above is false.
        public.verify(b"test", &signature, true).unwrap();
        assert!(public.verify(b"tampered", &signature, true).is_err());
    }
}
