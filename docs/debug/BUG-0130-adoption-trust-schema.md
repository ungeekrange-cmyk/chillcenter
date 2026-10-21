# BUG0130 — new adoption ledger initialized with the policy schema

- Date:2026-10-21, operator `/root`, branch `codex/pi-automatic-updates`.
- Scope: one-off legacy-adoption input; no application/helper binary change.
- Status: **VERIFIED_OPERATIONAL**.

The root config and service policy state use schema1, but the privileged trust
ledger uses **schema2** (`chill-updates/src/trust.rs::validate_state`). The new
adoption ledger was incorrectly initialized with tag1. The signed helper's
`inspect` failed closed with `invalid anti-rollback state` before daemon reload,
server start, timer activation, candidate acceptance or any automatic install.

H1 confirmed: compare the new ledger's tag1 with the published helper source's
required tag2. The components already contained exactly server/web highest3,
`pending:null`, derived from verified signed0.7.2 manifests. No previous trust
ledger existed and no sequence had been accepted by the helper during this
failed attempt.

Correction: retain the refused initializer in the private adoption evidence;
assert its entire expected contents, then atomically/fsync change **only** the
tag to2. Do not delete/reset the ledger or lower either sequence. The same signed
helper subsequently accepted `inspect`; recovery pre-starts and strict TLS/API
start passed. Real server/web checks and the first tick reported `no_update`,
root highest sequences remained3, and no install operation was admitted.

The runbook now explicitly separates schema1 config/policy and schema2 trust
and requires `inspect` before publishing the adoption drop-in. The helper's
strict parser, signatures, permissions and anti-rollback checks are unchanged.

FR : erreur d'initialisation opérateur, refus de sécurité correct du helper.
Registre neuf corrigé sans aucun recul de séquence; pas de migration SQLite,
changement de certificat, réinstallation ou nouvelle release. Le helper de
l'archive0.7.2 conserve son numéro interne de crate0.1.0 : les octets et leur
provenance signée, non ce libellé interne, identifient l'artefact installé.
