# Legacy automatic updates / Mises à jour automatiques d'une installation existante

2026-10-21. Explicit user authorization: activate **server and web** automatic
updates at **02:00 Pi local time**, after backup, without NAS or unrelated
settings changes. This is an administrator adoption procedure, not an initial
install, a new product release or an API change. Never run `install-server.sh`
against this existing installation.

## Frozen inputs / Entrées immuables

Use only independently verified signed release **v0.7.2**, source `a38d4b1`.
The public key is the existing full Minisign identity `18FD790B48C202F8` from
`updates/release.pub`. Verify manifests/archive signatures with an independently
installed Minisign, then compare raw helper/unit hashes against signed targets
again after root-private staging on the Pi, before any execution.

| Target | SHA-256 |
| --- | --- |
| Existing server / baseline payload | `cdde3ed064b79258ac17de3734c6750dd844e0f150bfcad8652f9d7e9b4cadea` |
| Signed updater helper | `b9d522381e5afe93ffdfb3beefecb5de1c0455df48dcbe0b7581ebbc99dfd281` |
| Signed updater service | `a1874229701c02f8549676f8d7f90038c9e79d9d782a7f43eb4172d1975ca52d` |
| Signed updater timer | `cc60e261750f07e3106ca47da67f65f2c62471cca60513f1f369076ae1018b0b` |
| Signed web ZIP | `5b2131a3ac8a20ca3a6398f69ff36e3697810f06f065971558d52a97e89b8a17` |

Both components are already installed at0.7.2/sequence3. Adoption must not
relabel an arbitrary tree: require the exact server digest and every current
web file's path/content to match the verified signed ZIP. Refuse links, special
files, duplicate/traversal ZIP names, unexpected files or insufficient local
space. This operation does not install a newer component.

## Preparation and backup / Préparation et sauvegarde

1. Read existing config locally to derive data directory, service UID/GID,
   listen port and TLS name. Do not print secrets or commit household values.
   Validate root-owned safe parents and absence of pre-existing updater config,
   helper, trust store, update units, adoption drop-in or component slots. Existing
   signed units' data paths must match the real data directory; otherwise stop.
2. Retain root-private baseline snapshots of server config, units/drop-ins,
   complete legacy web, TLS file hashes, updater state if present, logical SQLite
   data fingerprints and current versions. Do not traverse/mount the NAS.
3. Require idle API state; never interrupt active work. Stop only Chill Center.
   Run its existing binary's `--backup --config` as its non-root service user in
   a uniquely named systemd oneshot. The job and private transcript survive SSH
   loss. Wait for the same job's terminal exit0; verify the UUID receipt,
   manifest/files/hashes. Never restart/recover over a live backup job.

## Adoption / Activation

- Preserve the flat binary/web and base unit; create root-owned baseline slots
  `server/releases/0.7.2-3/payload` and `web/releases/0.7.2-3` with byte-identical
  copies and relative `current` links. No hardlinks or source-tree renames.
- Create only updater-specific trust/cache/runtime roots and root-owned
  `updates.json`, `release.pub`, public `health-ca.pem`. The latter copies the
  existing public certificate, never a private key. Keep `server.json`, library
  locations, account/group membership, network, TLS identity and existing
  systemd hardening/drop-ins byte-identical.
- Initialize the new root-only anti-rollback ledger for the verified server/web
  baseline at sequence3 with no pending transaction. Do not overwrite a prior
  trust ledger. Preserve existing policy/operation state; if no prior persisted
  state exists, initialize it: server/web automatic02:00, Mac defaults retained,
  themes manual so this two-component authorization does not silently activate
  automatic theme installation. Client theme/personal settings remain untouched.
- Configure the helper with the existing UID/GID and data/backup paths. Health
  uses strict HTTPS with a root-owned copy of the existing certificate and fixed
  private resolution derived from the existing listen config; never `-k` or an
  alternate API/version/repository/key. Certificate renewal later requires the
  administrator to refresh this root-owned health trust copy after validation.
- Install the signed helper and signed update service/timer. Add only
  `deploy/updates/40-legacy-update-slots.conf` under the existing service's drop-in
  directory. It changes only executable selection and adds the published
  pre-start recovery commands; it must not replace the base unit, NAS read-only
  paths, mount dependencies, working directory or Bonjour exception.
- Validate real systemd syntax, reload, start the same verified server payload,
  and verify strict TLS, API/version, web bytes and preserved data/settings.
  Run helper `inspect`, explicit server/web checks and a bounded initial tick:
  installed current releases must report no update, not download/reinstall.
- Enable the persistent minutely timer only after those checks. The published
  engine owns the daily02:00 policy, OS-local timezone/DST, missed-run handling,
  bounded retries, busy deferral, signed manifests and rollback. A minutely timer
  wake is not a minutely GitHub check. Record today's admission once so the next
  routine check is tomorrow02:00. Do not activate automatic theme installation.

## Rollback and verification / Retour arrière et vérification

Before timer activation, a failure leaves scheduling disabled. Do not race an
active helper or backup. With both quiescent, remove only the newly introduced
adoption drop-in, reload and start the retained flat binary/base unit. Retain all
backups, slot files, trust receipts and staged evidence; quarantine updater config
only if disabling a partial adoption. Never erase a root acceptance ledger to
permit replay. No database migration occurs. Independently verify the exact
installed artifacts, strict TLS/API, unchanged original settings/data/web, unit
hardening, policies, journal and next daily check. A future higher-version
installation/power failure is not proven by a successful NO_UPDATE check.

## Français

Seuls serveur et web deviennent automatiques à02:00 dans le fuseau Linux.
Sauvegarde privée cohérente avant adoption; conservation des fichiers plats,
unités, NAS, données, certificats et réglages personnels. Les mêmes octets0.7.2
sont placés dans des slots versionnés; le helper signé vérifie les futures
publications compatibles et diffère l'installation si une lecture/travail est
actif. Les thèmes ne sont pas activés automatiquement par cette demande.
Le premier tick peut constater l'horaire manqué aujourd'hui sans réinstaller
la version courante; le passage quotidien suivant est demain à02:00locale.
Ne pas confondre cette activation avec le cron hebdomadaire des paquets Debian.

Le README indiquait encore0.7.1 malgré `versions.json` et les releases0.7.2 :
liens FR/EN corrigés, avec test de cohérence des quatre paquets indépendants.
Les thèmes restent0.7.1. Aucun tag ou paquet public n'est réécrit.
