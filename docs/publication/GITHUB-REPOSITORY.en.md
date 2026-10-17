# Preparing the public GitHub project

[Français](GITHUB-REPOSITORY.md) | **English**

The main deliverable is the **project repository**: sources, README, licence,
contribution guidelines and security policy. GitHub Pages is an optional
presentation, not a new web application. This preparation has published no
repository, website or history.

## Distributed configuration

- No household host in defaults: both server examples listen on loopback, port `8443`.
- Keep LAN addresses, real paths and TLS names in ignored local configuration,
  such as `server.local.json`. Documentation DNS names are examples, not
  preconfigured working values.
- Historical operator tools explicitly require `CHILLCENTER_TARGET_HOST`;
  first installation also requires `CHILLCENTER_LISTEN_ADDRESS`,
  `CHILLCENTER_TLS_NAME`, `CHILLCENTER_NAS_SOURCE`. R7 recovery requires the NAS
  source; the harness requires `CHILLCENTER_SERVER_URL`.
- This preparation executed no tool against a device. Old script hashes and
  gates remain historical evidence; modified scripts need review and regenerated
  gates before reuse. The first installer remains tied to its old approved
  artifact: it is not a generic installer for the current version.

Private IPs still present in tests are synthetic fixtures for LAN safeguards.
Loopback addresses are intentional and must not be replaced by wildcard listening.

## Sensitive information checks

Check committed files **and the entire history that would become public**,
including branches and tags. Checking the index or HEAD alone is insufficient.

Tool used for the original preparation: Gitleaks `8.30.1`, official binary
downloaded and verified by SHA-256. It runs locally without uploading sources.
Example command with a private report outside the repository:

```sh
umask 077
gitleaks git --log-opts='--all --full-history' --redact=100 \
  --ignore-gitleaks-allow --report-format json \
  --report-path /chemin/prive/audit-redacted.json .
```

Do not enable verbose output or paste detected values into an issue. Inspect
alerts privately. Alerts from that preparation were independently classified
as deterministic hexadecimal TMDB test fixtures under `cfg(test)`, without
checking them against a provider. No blanket key or test-directory exclusion
was added.

Also inventory sensitive filenames, PEM blocks, DER objects and complete Git
objects: a text search or diff scan cannot rule out every certificate. Server
certificates/keys are generated at runtime, not distributed assets. NAS
passwords must never be required by the client or repository.

Certificate/key extensions, environment files and `*.local.json` are ignored.
This reduces future accidents but does not erase files already in history.
A scan provides reasonable assurance within its scope, not mathematical proof
that no secret exists.

## History: a decision is still required

The original audit identified no genuine secret in the examined local sources
and history. **Old household addresses remain in earlier commits.** Anonymizing
the proposed working version does not remove them from Git. Internal documents
also contain operational paths and personal information; they are not an
approved public showcase.

Two strategies require explicit approval before publication:

1. Prepare a sanitized public initial state **without private history**, keeping
   the complete working repository and archives private. A separate publication
   repository avoids exposing old branches and tags.
2. Sanitize the existing repository's history, all affected branches/tags and
   remote copies. This requires coordination, backups and agreement with other
   workstreams; do not perform it implicitly or force-push.

In either case, review and rescan the exact selected artifact, check licence
and artwork rights and excluded operational files, then decide on GitHub
visibility. Current preparation does not certify unfetched remote references,
inaccessible reflog objects or untracked files. The web workstream and delivered
versions remain intact.

See [optional GitHub Pages](GITHUB-PAGES.en.md), which publishes only an explicit
allowlist; approving it does not automatically approve the whole repository.
