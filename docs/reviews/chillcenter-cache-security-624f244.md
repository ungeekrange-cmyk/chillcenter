# Revue sécurité — cache client configurable `624f244`

Date : 2026-10-14  
Rôle : revue indépendante, candidat inspecté en lecture seule  
Base pré-cache : `c58da7e614b54890e0ef0207f4679d9b02cfca0e`  
Parent approuvé séparément : `27bd5b24b101ac7028bbd3ad8a2f81159bd40e03`  
Candidat : `624f2447b5c0ede6b4a053ea3fa7b82db85c0121`  
Répertoire : `/private/tmp/chillcenter-native-ui.KZRHlE/repo`

## Verdict

**APPROVED** pour le SHA exact `624f2447b5c0ede6b4a053ea3fa7b82db85c0121` et uniquement pour le réglage local client décrit par `ADR-0010 / CC-CACHE-002`.

Aucun finding HIGH/CRITICAL/MEDIUM. Aucun changement serveur, dépendance, lockfile, TLS, grant, bail, protocole ou accès NAS. L'ancienne approbation `27bd5b2` demeure conservée dans `/private/tmp/chillcenter-cache-security-27bd5b2.md` (SHA-256 inchangé `8ccfd486f7027d72bef65a6d334a4a66fa7f4bbf104592be7d3933f893d4bd62`) ; elle n'est pas réécrite et n'était pas considérée transitive.

Restrictions : un seul lecteur actif par processus ; mesure RSS réelle requise pour les valeurs par défaut et maximales avant toute affirmation de capacité ; les essais trois flux en cours sur `27bd5b2` ne prouvent pas l'UI/persistance/configuration de ce SHA. Toute modification des bornes, de la persistance, de `PlayerOptions`, du transport, des baux ou de TLS rend ce verdict périmé.

## Findings

### LOW — maximum local élevé, borné mais pas une limite RSS

Le maximum configurable représente 512 Mio avant + 256 Mio arrière = 768 Mio de paquets compressés par lecteur. Décodage, rendu, webview, transport et dépassements internes libmpv s'ajoutent. L'entrée est locale, explicite, bornée et ne permet pas à un serveur/NAS/profil distant d'imposer cette valeur ; ce n'est donc pas une primitive de DoS distant. Mesurer le RSS du bundle sur le Mac cible à 128/64 et 512/256, vérifier la réactivité sous pression mémoire, et conserver l'avertissement UI.

### INFO — application différée et cohérente

Une sauvegarde ne modifie pas la lecture active. `PlaybackConnection` prend un snapshot local ; `Session.cache` conserve celui-ci pour reprise/enchaînement. Une nouvelle lecture/reload applique le snapshot avant le nouveau `load`. En cas de réglage invalide, la validation échoue avant création du lecteur ou avant toute écriture de propriété ; le grant nouvellement acquis est relâché par le bridge si le chargement échoue.

### INFO — échec de durabilité déjà explicite

`client.json` reste publié atomiquement en fichier `0600` sous dossier `0700`, avec synchronisation de répertoire. Comme le contrat existant, un échec du fsync du dossier peut signifier « publié mais durabilité non confirmée » : l'erreur demande une réouverture. Aucun secret nouveau n'est stocké ; les deux entiers sont dans le même fichier privé que les réglages existants.

## Invariants vérifiés

- `PlaybackCache` est un objet fermé (`deny_unknown_fields`) avec deux `u16`; négatifs, flottants, champs inconnus et dépassements de type sont refusés.
- Bornes Rust : avant `16..=512`, arrière `0..=256`; défaut rétrocompatible `128/64`; `0` arrière est admis explicitement.
- Validation à trois frontières : ouverture de `client.json`, `SaveCacheSettings`, puis `Player::new`/`set_cache_limits`. Les valeurs ne peuvent devenir des noms/options mpv arbitraires : seules des chaînes décimales issues de `u16` suivies de `MiB` sont produites.
- `UiCommand` reste fermé (`tag`, `rename_all`, `deny_unknown_fields`). La commande locale n'accepte ni URL, ni chemin, ni grant, ni option libmpv libre. Le contrôleur DOM utilise `textContent`, bloque les doubles soumissions et Rust reste l'autorité de validation.
- Sauvegarde sans serveur ; réglages indépendants entre répertoires clients ; anciens JSON utilisent les défauts. Une erreur normale de sauvegarde restaure la valeur mémoire précédente.
- `cache=yes`, `cache-secs=60`, `demuxer-readahead-secs=60`, `demuxer-donate-buffer=no`, les interdictions de config/scripts/ytdl/références/autoload et les tests HTTP/HLS restent actifs.
- Le chemin reload met le lecteur en pause avant changement, valide avant propriétés et modifie les limites avant `clear_playback`/nouveau média. Les échéances monotone + murale, la pause à expiration et les contrôles de grant ne sont pas affaiblis.
- Aucun changement sous `crates/chill-server`, ni dans `Cargo.toml`/`Cargo.lock`; aucun secret, clé, bearer ou token ajouté par le delta.
- La persistance d'un budget local ne change ni préférence de profil, ni identité serveur, ni trust store logique, ni isolation entre profils.

## Commandes et résultats

```sh
git status --short
git rev-parse HEAD
git show -s --format='%H %P %s' HEAD
git merge-base --is-ancestor 27bd5b24b101ac7028bbd3ad8a2f81159bd40e03 HEAD
git diff --check 27bd5b24b101ac7028bbd3ad8a2f81159bd40e03..HEAD
git diff --name-status 27bd5b24b101ac7028bbd3ad8a2f81159bd40e03..HEAD
git diff --exit-code 27bd5b24b101ac7028bbd3ad8a2f81159bd40e03..HEAD -- crates/chill-server Cargo.toml Cargo.lock
git diff --check c58da7e614b54890e0ef0207f4679d9b02cfca0e..624f2447b5c0ede6b4a053ea3fa7b82db85c0121
shasum -a 256 crates/chill-client/src/playback_cache.rs crates/chill-client/src/ui.rs crates/chill-player/src/lib.rs apps/desktop/src/native_player.rs apps/desktop/src/player_bridge.rs ui/connected/cache.js ui/connected/cache.dom.test.js docs/architecture/decisions/ADR-0010-client-playback-cache.md
CARGO_HOME=/private/tmp/video-catalog-cargo RUSTUP_HOME=/private/tmp/video-catalog-rustup CARGO_TARGET_DIR=/private/tmp/chillcenter-root-target PATH=/private/tmp/video-catalog-cargo/bin:/opt/homebrew/bin:/usr/bin:/bin cargo test --locked --offline -p chill-client -p chill-player -p chillcenter-desktop
node --test ui/connected/cache.dom.test.js
node --test ui/connected/*.test.js
git diff 27bd5b24b101ac7028bbd3ad8a2f81159bd40e03..HEAD | rg -n -i 'password|passwd|secret|private.key|BEGIN [A-Z ]*PRIVATE|authorization|bearer|token'
```

Résultats autonomes : ascendance, arbre propre, diff-check et absence de delta serveur/lock PASS ; Rust **69/69 PASS** (client 32, player 17, desktop 20) ; doc-tests PASS ; contrôleur cache **3/3 PASS** ; glob DOM disponible **19/19 PASS**. La preuve fournie annonce 23 tests Node et clippy strict sur les trois paquets PASS ; ces deux éléments supplémentaires ne sont pas revendiqués comme rejoués intégralement par cette revue. La recherche sensible n'a trouvé aucune addition pertinente.

SHA-256 observés :

- `playback_cache.rs` : `0b655dd2f554ac152663aa586f48efe91ad4fbca4052ec570192f8dfc6b0b28e`
- `ui.rs` : `f162e5cc348af4fa62fc0019ef2ecc43f151a6452d843181a1a8a081cedecdfe`
- `chill-player/src/lib.rs` : `19f6c8bbdadfc470ec364434f9f847f44d29ddb3fed8751552b500272ba21530`
- `native_player.rs` : `161eb233c93e48c5cbc49bcda0df0c79b978a2aca8c949ff43666efd66a55cd1`
- `player_bridge.rs` : `ac81e55e39b97aeeac716474f68d0bc4bc28e9708eb11aa793cc9852b7f0591a`
- `cache.js` : `68b063376bbeb3f50623b2594f34791db4abee366aee72d474a2b102c047ad1b`
- `cache.dom.test.js` : `4d844d173146c6bcf9f5634f20da2eeff830e6c25bf4a5ac0d720b836b8624ad`
- `ADR-0010` : `72d37abd8a09f368f2af9d19d071d39da7d48fe73ca8b3fc254d750bbdf3c2e5`

## Preuves restantes

- Build/bundle Mac exact `624f244` et test humain de l'Administration.
- Réouverture réelle de l'application pour prouver la persistance du bundle.
- Mesures RSS et essais NAS aux valeurs défaut/max ; seek arrière sans refetch.
- Les essais trois flux déjà lancés sur `27bd5b2` restent utiles pour le transport/cache fixe mais ne couvrent pas ce nouveau contrat configurable.
