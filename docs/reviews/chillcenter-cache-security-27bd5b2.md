# Revue sécurité — cache lecteur Mac `27bd5b2`

Date : 2026-10-14  
Rôle : revue indépendante, lecture seule du candidat  
Base : `c58da7e614b54890e0ef0207f4679d9b02cfca0e`  
Candidat : `27bd5b24b101ac7028bbd3ad8a2f81159bd40e03`  
Répertoire inspecté : `/private/tmp/chillcenter-native-ui.KZRHlE/repo`

## Verdict

**APPROVED** pour le delta client exact ci-dessus, avec les restrictions suivantes : aucun changement serveur/Pi, aucun changement de confiance TLS ou de protocole, un seul lecteur actif par processus, et validation RSS/NAS réelle encore requise avant de présenter 60 s d'avance comme une mesure obtenue sur les médias cibles. Toute modification ultérieure du lecteur, des options libmpv, du transport, des baux, de TLS ou des références de conteneur rend cette approbation périmée.

Aucun finding HIGH/CRITICAL et aucun blocage de sécurité. Le delta est borné et ne change ni dépendance, ni authentification/autorisation, ni accès NAS, ni surface serveur.

## Constats priorisés

### LOW — budget mémoire client à confirmer en conditions réelles

Les plafonds libmpv sont explicites : 128 MiB avant + 64 MiB arrière, soit 192 MiB de paquets compressés par lecteur. La mémoire décodée, le rendu, le webview et le transport s'ajoutent à ce budget. Il s'agit d'une borne du cache, pas d'une borne RSS globale. L'ADR-0010 l'indique correctement. Condition de livraison : mesurer le RSS du bundle Mac pendant les essais NAS, notamment MKV/MP4 1080p, et conserver un seul `Player` actif par processus.

### INFO — 30 s arrière est une cible variable, pas une garantie

libmpv borne l'arrière en octets, pas en temps. À débit plus élevé, 64 MiB couvrent moins de 30 s ; à débit plus faible, davantage. L'ADR-0010 documente cette limite et n'annonce pas une durée exacte. `buffered_ahead_seconds()` lit uniquement `demuxer-cache-duration`; une valeur absente ou invalide devient `None`, sans inventer 60 s.

### INFO — surcharge des options maintenue dans le périmètre sûr actuel

`PlayerOptions.extra` peut encore ajuster `cache`/`cache-secs`, mais ne peut pas modifier `demuxer-max-bytes`, `demuxer-max-back-bytes`, `demuxer-readahead-secs` ou `demuxer-donate-buffer`. Les appels de production inspectés n'injectent que `vo=libmpv`. Ne pas exposer ces extras à une entrée distante/non fiable sans nouvelle revue.

## Invariants vérifiés

- `cache=yes`, `cache-secs=60`, `demuxer-readahead-secs=60`, `demuxer-max-bytes=128MiB`, `demuxer-max-back-bytes=64MiB`, `demuxer-donate-buffer=no` sont appliqués avant l'initialisation libmpv et testés sur les options effectives.
- Les limites en octets ne sont pas surchargeables via les options supplémentaires autorisées.
- `config=no`, `load-scripts=no`, `ytdl=no`, `access-references=no`, `autoload-files=no` et le schéma Rust lié au grant restent en place.
- Les tests réels HTTP/HLS confirment que les références non épinglées ne s'ouvrent pas.
- `apps/desktop/src/native_player.rs` est identique à la base (SHA-256 `219ab1432a7a3cc873b83b5f49fd9f15c0efea818b0ed66b64f2674a4b2ba240`) : l'échéance locale du bail, la pause à expiration et le remplacement/révocation du grant ne changent pas. Les octets déjà reçus ne prolongent donc pas l'autorisation.
- Aucun changement dans `crates/chill-client`, `crates/chill-server`, `Cargo.toml` ou `Cargo.lock`; aucune baisse de TLS, aucune nouvelle référence réseau, aucun cache disque demandé.
- La sonde ajoute seulement une mesure numérique/optionnelle `buffered_ahead_seconds`; elle n'imprime ni jeton, ni URL signée, ni secret.

## Preuves et commandes

```sh
git status --short
git rev-parse HEAD
git merge-base --is-ancestor c58da7e614b54890e0ef0207f4679d9b02cfca0e 27bd5b24b101ac7028bbd3ad8a2f81159bd40e03
git diff --check c58da7e614b54890e0ef0207f4679d9b02cfca0e..27bd5b24b101ac7028bbd3ad8a2f81159bd40e03
git diff --name-status c58da7e614b54890e0ef0207f4679d9b02cfca0e..27bd5b24b101ac7028bbd3ad8a2f81159bd40e03
git diff --exit-code c58da7e614b54890e0ef0207f4679d9b02cfca0e..27bd5b24b101ac7028bbd3ad8a2f81159bd40e03 -- apps/desktop/src/native_player.rs crates/chill-client crates/chill-server Cargo.toml Cargo.lock
shasum -a 256 apps/desktop/src/native_player.rs crates/chill-player/src/lib.rs docs/architecture/decisions/ADR-0010-client-playback-cache.md scripts/pi-probe/src/main.rs
CARGO_HOME=/private/tmp/video-catalog-cargo RUSTUP_HOME=/private/tmp/video-catalog-rustup CARGO_TARGET_DIR=/private/tmp/chillcenter-root-target PATH=/private/tmp/video-catalog-cargo/bin:/opt/homebrew/bin:/usr/bin:/bin cargo test --locked --offline -p chill-player
```

Résultats : identité/ascendance/diff-check PASS ; surfaces exclues sans diff ; `chill-player` **16/16 PASS** et doc-tests PASS lors du relancement autorisé hors bac à sable. Le premier lancement dans le bac à sable avait 14 PASS et 2 échecs `PermissionDenied` à la création des sockets loopback des fixtures HTTP/HLS ; le relancement avec accès loopback a confirmé les deux tests PASS. La suite globale de 46 tests PASS fournie avec le candidat est cohérente avec cette vérification ciblée, mais n'est pas revendiquée ici comme réexécutée par le reviewer.

SHA-256 observés :

- `crates/chill-player/src/lib.rs` : `a92528c7dba06efa2eebc62e5c58fdc71e4528c95e8285918bc7866ab25f328e`
- `ADR-0010-client-playback-cache.md` : `e20eab4affd917d736ba675c5e5cf22b91c5cce5048c3b4c4204f81a2b09746a`
- `scripts/pi-probe/src/main.rs` : `07819ba721e8a5e3248906af490ac91a50cafa97728eece80aefac33c21f145d`

## Hors périmètre / preuves restantes

- Aucun déploiement Pi ou test NAS effectué par cette revue.
- Aucun test de performance longue durée ni mesure RSS du bundle Mac effectué.
- La cible temporelle dépend du débit réel, du VBR, des pistes et du réseau ; la sonde doit publier la mesure, pas seulement la configuration.
