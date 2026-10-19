use crate::manifest::Component;
use crate::{FIXED_GITHUB_OWNER, FIXED_GITHUB_REPOSITORY, Result, UpdateError};
use reqwest::blocking::{Client, Response};
use reqwest::header::{ACCEPT, USER_AGENT};
use reqwest::redirect::{Attempt, Policy};
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct TransportLimits {
    pub max_pages: u8,
    pub max_redirects: usize,
    pub max_manifest_bytes: u64,
    pub max_signature_bytes: u64,
    pub max_asset_bytes: u64,
    pub timeout: Duration,
}

impl Default for TransportLimits {
    fn default() -> Self {
        Self {
            max_pages: 4,
            max_redirects: 5,
            max_manifest_bytes: 1024 * 1024,
            max_signature_bytes: 16 * 1024,
            max_asset_bytes: 2 * 1024 * 1024 * 1024,
            timeout: Duration::from_secs(60),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ReleaseCandidate {
    pub version: Version,
    pub tag: String,
    pub manifest_url: reqwest::Url,
    pub signature_url: reqwest::Url,
}

pub struct GithubReleaseClient {
    client: Client,
    limits: TransportLimits,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

impl GithubReleaseClient {
    pub fn new(limits: TransportLimits) -> Result<Self> {
        if limits.max_pages == 0 || limits.max_pages > 10 || limits.max_redirects > 10 {
            return Err(UpdateError::Invalid(
                "transport limits are outside hard bounds".into(),
            ));
        }
        let redirects = limits.max_redirects;
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(limits.timeout)
            .redirect(Policy::custom(move |attempt| {
                redirect_policy(attempt, redirects)
            }))
            .build()?;
        Ok(Self { client, limits })
    }

    /// Select the highest stable release from the fixed repository and bounded
    /// release pages. GitHub's global "latest" endpoint is never used.
    pub fn find_latest(&self, component: Component) -> Result<Option<ReleaseCandidate>> {
        let prefix = format!("{}-v", component.as_str());
        let manifest_name = format!("{}-manifest.json", component.as_str());
        let signature_name = format!("{manifest_name}.minisig");
        let mut best: Option<ReleaseCandidate> = None;
        for page in 1..=self.limits.max_pages {
            let url = format!(
                "https://api.github.com/repos/{FIXED_GITHUB_OWNER}/{FIXED_GITHUB_REPOSITORY}/releases?per_page=100&page={page}"
            );
            let releases: Vec<GithubRelease> = self.get_json(&url)?;
            let count = releases.len();
            for release in releases {
                if release.draft || release.prerelease || !release.tag_name.starts_with(&prefix) {
                    continue;
                }
                let Ok(version) = Version::parse(&release.tag_name[prefix.len()..]) else {
                    continue;
                };
                if !version.pre.is_empty() {
                    continue;
                }
                let manifest = release
                    .assets
                    .iter()
                    .find(|asset| asset.name == manifest_name);
                let signature = release
                    .assets
                    .iter()
                    .find(|asset| asset.name == signature_name);
                let (Some(manifest), Some(signature)) = (manifest, signature) else {
                    continue;
                };
                let candidate = ReleaseCandidate {
                    version,
                    tag: release.tag_name,
                    manifest_url: checked_release_url(&manifest.browser_download_url)?,
                    signature_url: checked_release_url(&signature.browser_download_url)?,
                };
                if best
                    .as_ref()
                    .is_none_or(|current| candidate.version > current.version)
                {
                    best = Some(candidate);
                }
            }
            if count < 100 {
                break;
            }
        }
        Ok(best)
    }

    pub fn download_manifest(&self, candidate: &ReleaseCandidate) -> Result<(Vec<u8>, String)> {
        let manifest = self.get_bounded(&candidate.manifest_url, self.limits.max_manifest_bytes)?;
        let signature =
            self.get_bounded(&candidate.signature_url, self.limits.max_signature_bytes)?;
        let signature = String::from_utf8(signature)
            .map_err(|_| UpdateError::Invalid("signature is not UTF-8".into()))?;
        Ok((manifest, signature))
    }

    /// Stream to a new file while checking both the signed byte count and
    /// signed SHA-256. Partial files are removed on every error.
    pub fn download_verified(
        &self,
        url: &str,
        destination: &Path,
        expected_bytes: u64,
        expected_sha256: &str,
    ) -> Result<()> {
        if expected_bytes == 0 || expected_bytes > self.limits.max_asset_bytes {
            return Err(UpdateError::Security(
                "asset byte length exceeds policy".into(),
            ));
        }
        let url = checked_release_url(url)?;
        let mut response = self.get(&url)?;
        if let Some(length) = response.content_length()
            && length != expected_bytes
        {
            return Err(UpdateError::Security(
                "HTTP asset length differs from signed manifest".into(),
            ));
        }
        if fs::symlink_metadata(destination).is_ok() {
            return Err(UpdateError::Security(
                "download destination already exists".into(),
            ));
        }
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)?;
        let result = (|| {
            let mut hasher = Sha256::new();
            let mut total = 0u64;
            let mut buffer = [0u8; 64 * 1024];
            loop {
                let read = response.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                total = total
                    .checked_add(read as u64)
                    .ok_or_else(|| UpdateError::Security("download length overflow".into()))?;
                if total > expected_bytes {
                    return Err(UpdateError::Security(
                        "download exceeded signed byte length".into(),
                    ));
                }
                hasher.update(&buffer[..read]);
                output.write_all(&buffer[..read])?;
            }
            if total != expected_bytes {
                return Err(UpdateError::Security("download was truncated".into()));
            }
            let actual = hex::encode(hasher.finalize());
            if !actual.eq_ignore_ascii_case(expected_sha256) {
                return Err(UpdateError::Security("download SHA-256 mismatch".into()));
            }
            output.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            drop(output);
            let _ = fs::remove_file(destination);
        }
        result
    }

    fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        let parsed = reqwest::Url::parse(url)
            .map_err(|_| UpdateError::Invalid("invalid GitHub API URL".into()))?;
        let bytes = self.get_bounded(&parsed, 2 * 1024 * 1024)?;
        crate::parse_strict_json(&bytes)
    }

    fn get_bounded(&self, url: &reqwest::Url, maximum: u64) -> Result<Vec<u8>> {
        let response = self.get(url)?;
        if response
            .content_length()
            .is_some_and(|length| length > maximum)
        {
            return Err(UpdateError::Security("HTTP response exceeds limit".into()));
        }
        let mut bytes = Vec::new();
        response.take(maximum + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > maximum {
            return Err(UpdateError::Security("HTTP response exceeds limit".into()));
        }
        Ok(bytes)
    }

    fn get(&self, url: &reqwest::Url) -> Result<Response> {
        require_initial_url(url)?;
        let response = self
            .client
            .get(url.clone())
            .header(USER_AGENT, "chillcenter-update/1")
            .header(ACCEPT, "application/vnd.github+json")
            .send()?
            .error_for_status()?;
        require_allowed_https(response.url())?;
        Ok(response)
    }
}

fn checked_release_url(value: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse(value)
        .map_err(|_| UpdateError::Invalid("invalid release URL".into()))?;
    let required_prefix =
        format!("/{FIXED_GITHUB_OWNER}/{FIXED_GITHUB_REPOSITORY}/releases/download/");
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || !url.path().starts_with(&required_prefix)
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(UpdateError::Security(
            "URL is outside fixed GitHub releases".into(),
        ));
    }
    Ok(url)
}

fn require_initial_url(url: &reqwest::Url) -> Result<()> {
    if url.host_str() == Some("api.github.com") {
        let prefix = format!("/repos/{FIXED_GITHUB_OWNER}/{FIXED_GITHUB_REPOSITORY}/releases");
        if url.scheme() == "https" && url.path() == prefix {
            return Ok(());
        }
    }
    checked_release_url(url.as_str()).map(|_| ())
}

fn require_allowed_https(url: &reqwest::Url) -> Result<()> {
    if url.scheme() != "https" || !allowed_host(url.host_str()) {
        return Err(UpdateError::Security(
            "redirect left HTTPS/GitHub CDN allowlist".into(),
        ));
    }
    Ok(())
}

fn allowed_host(host: Option<&str>) -> bool {
    matches!(
        host,
        Some(
            "api.github.com"
                | "github.com"
                | "release-assets.githubusercontent.com"
                | "objects.githubusercontent.com"
        )
    )
}

fn redirect_policy(attempt: Attempt<'_>, maximum: usize) -> reqwest::redirect::Action {
    if attempt.previous().len() >= maximum {
        return attempt.error("too many redirects");
    }
    if attempt.url().scheme() != "https" || !allowed_host(attempt.url().host_str()) {
        return attempt.error("redirect target is outside allowlist");
    }
    attempt.follow()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_fixed_release_urls_are_accepted() {
        assert!(checked_release_url("https://github.com/ungeekrange-cmyk/chillcenter/releases/download/server-v0.7.0/server.tar").is_ok());
        assert!(checked_release_url("http://github.com/ungeekrange-cmyk/chillcenter/releases/download/server-v0.7.0/server.tar").is_err());
        assert!(
            checked_release_url(
                "https://github.com/evil/chillcenter/releases/download/server-v0.7.0/server.tar"
            )
            .is_err()
        );
        assert!(checked_release_url("https://github.com/ungeekrange-cmyk/chillcenter/releases/download/server-v0.7.0/server.tar?x=1").is_err());
    }
}
