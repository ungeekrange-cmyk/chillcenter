# BUG0115 — Native updater configuration aborts startup

- Priority: P1, discovered before v0.7 publication.
- Affected source: `6041f4d` and earlier component-update candidates.
- Correction: `3913267e98f82eeca5f7c627490fbdd019636177`.
- Status: FIXED_IN_SOURCE / NATIVE_RETEST_PENDING; not a delivered release.

## Observation and cause

Launching the isolated real Tauri QA bundle on macOS 27.0.1 / Apple Silicon
aborted before any confirmation dialog. The setup error was:

```text
failed to initialize plugin `updater`:
Error deserializing 'plugins.updater' within your Tauri configuration:
missing field `pubkey`
```

Tauri deserializes the plugin's configuration before the updater builder applies
its programmatic pinned public key. The JSON contained `requireSignedVersion`
but omitted the mandatory `pubkey`. Compilation and non-GUI tests alone did not
exercise this initialization seam. The failure was not a Pi/network/certificate
problem and no normal installed application was replaced.

## Correction and regression

The embedded JSON now includes the exact public release key, in the base64
representation expected by the SDK. The existing programmatic constant remains
unchanged; no renderer-controlled key/URL or new trust authority is introduced.
No private key is embedded.

`embedded_updater_configuration_deserializes_before_builder_overrides` loads the
actual JSON through the actual SDK Config deserializer. It checks the exact
decoded public key, signed-version requirement, empty endpoint list and disabled
insecure transport/certificate/hostname/downgrade switches.

The isolated AppKit confirmation/install/restart journey must still run. Its
first failed launch is retained as adverse evidence rather than relabelled PASS.

## Résumé français

Le vrai démarrage Tauri échouait avant l'affichage, car le SDK exige `pubkey`
dans le JSON avant d'appliquer la clé publique fixée par le code. Le champ
public manquant et un test de désérialisation du SDK ont été ajoutés. Aucun
secret ni contournement TLS ; essai natif final encore requis avant publication.
