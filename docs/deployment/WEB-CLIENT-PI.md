# Déploiement du client web sur le Pi — artefacts construits et testés, déploiement NON exécuté

Révision `CC-WEB-DEPLOY-001` r2, 15 octobre 2026, branche `codex/web-pwa`, source `05e637dec0b721300925be14e46c94c26d331789`. **Rien n'a été exécuté sur le Pi ni copié vers lui.** Ce document est une proposition soumise à revue indépendante puis à ton feu vert explicite (AGENTS : pas de déploiement sur un matériel ou un réseau supposé ; un seul auteur par chemin, le reviewer n'édite pas).

## 1. Ce qui change sur le Pi, et ce qui ne change pas

| Change | Ne change pas |
| --- | --- |
| Binaire `/usr/lib/chillcenter/chill-server` (remplacé) | `/etc/chillcenter/server.json` (hash contrôlé avant/après) |
| Nouveau dossier **`/usr/lib/chillcenter/web/`** (fichiers statiques, root, lecture seule pour le service) | Unité systemd `chillcenter.service` (hash contrôlé) |
| Une sauvegarde privée supplémentaire sous `/var/lib/chillcenter/data/backups/<uuid>` (0700, `chillcenter`) | Montage NAS, vidéos, base SQLite, profils, identité, clés et certificat TLS (SPKI contrôlée) |
| Une copie de l'ancien binaire dans `/root/chillcenter-rollback/` | Exposition réseau, ports, pare-feu, utilisateurs, groupes |
| Deux fichiers temporaires dans `/var/tmp/` (supprimés ensuite) | Aucun transcodage, aucun nouveau processus résident |

Pourquoi `web/` à côté du binaire et non dans `data_dir` : la sauvegarde/restauration ne traite qu'une liste fermée (base, affiches, TLS, config) ; un dossier web dans les données serait **silencieusement perdu à une restauration**. À côté du binaire, il est remplacé avec lui, hors de portée d'écriture du service (`ProtectSystem=strict`).

Interruption du service : arrêt, sauvegarde hors ligne, copies et redémarrage ≈ quelques dizaines de secondes (à mesurer ; la V4 avait la même séquence). Le script **refuse d'arrêter** s'il existe une connexion cliente établie.

## 2. Entrées figées et hashes

| Entrée | Valeur |
| --- | --- |
| Binaire actuellement installé (V4, 14 oct. 19:12 UTC) | `1eb24e97bcdf8f66440c1681af92d6202c151ebe336c2890ea42a7f8179ac181` |
| Ancien rollback R7 (déjà sur le Pi) | `/root/chillcenter-rollback/chill-server-r7-9a17ffe55e3d`, `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6` |
| Certificat public du Pi (lu le 15 oct.) | SPKI SHA-256 `3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df`, valide du 14/10/2026 au 14/04/2028 ; `server_id` `885f7713-b8c6-4891-a8d1-c1410d5f1537` |
| Script opérateur candidat V5 `scripts/pi-update-web-operator.sh` | 13 078 octets, SHA-256 `8d47e9df8feba5c234552ff61517fd04e29a8612c2a3e14cabfc3b420fbb9d14` — **toute modification, même un commentaire, annule la revue** |
| **Archive web finale** (`scripts/package-web-pi.sh`, sans métadonnées macOS) | `artifacts/web-pi/chillcenter-web.tar`, **1 099 264 octets**, 54 fichiers, SHA-256 `1aaf79a6c476904b191a0e3fe02a64c5535df82b0753448d5f9884a18f6cc669`. Le tar n'est pas reproductible octet pour octet (horodatages) : **ce fichier exact** est celui à copier ; le contenu (build web `387b98d9b0ad`) est reproductible |
| Décodeur MPEG-4 embarqué | `mpeg4-decoder.wasm` SHA-256 `453fb7b171fde50d1306a57793ebac571459a7c324d28bd8b4f6c3db6b7bda53` (652 895 o), `mpeg4-decoder.js` `64d97f891594c386652cf3f783dfbdaf65d358ddb0bf1be96f1fbe3b2d3be827` ; FFmpeg 9.0.2 `8c385028…002e`, emsdk 6.0.12 `sha256:a7883da4…8c98` |
| **Nouveau binaire serveur** (`cargo zigbuild --release --locked -p chill-server --target aarch64-unknown-linux-gnu.2.28`) | `target/aarch64-unknown-linux-gnu/release/chill-server`, **14 601 224 octets**, ELF 64-bit ARM aarch64 dynamique, strippé, SHA-256 **`c3681a47f7dc7b39730e3e11024306ab754b4142aef44eb026edb7e0372d2c47`** → `NEW_BINARY_SHA256` |
| Source | commit `05e637dec0b721300925be14e46c94c26d331789` (les commits suivants ne touchent que la documentation et le paquetage) |

## 3. Compilation du serveur : état réel et téléchargements à approuver

**Constat initial (avant exécution).** La modification Rust (garde d'origine + service des fichiers statiques + leurs tests) n'avait jamais été compilée : `cargo` n'était pas sur le PATH de la session. Il existait (`~/.cargo/bin`), mais :

1. **Effet de bord déjà subi, à signaler** : lancer `cargo` dans le dépôt a déclenché `rust-toolchain.toml` et `rustup` a **installé automatiquement la chaîne 1.99.0 (≈ 470 Mo sur disque)** sans que je l'aie demandé ni annoncé. C'est la chaîne épinglée par le projet ; à garder (nécessaire au build) ou à retirer (`rustup toolchain uninstall 1.99.0`) selon ton choix.
2. Le cache des crates est **incomplet** (index et 123 crates manquants pour le serveur seul) : `--offline` échoue.
3. Les outils de compilation croisée utilisés pour la livraison précédente (**Zig 0.17.0**, **cargo-zigbuild 0.23.4**, cible Rust `aarch64-unknown-linux-gnu`) ne sont plus installés.

**Téléchargements nécessaires pour compiler et tester (tailles exactes, métadonnées lues sans téléchargement) — *approuvés par l'utilisateur le 15 octobre 2026, exécutés* :**

| Élément | Taille exacte | Vérification |
| --- | ---: | --- |
| Crates du serveur absentes du cache (≤ 123, borne haute calculée depuis `Cargo.lock`) | ≤ 33 769 949 octets | `cargo` contrôle les sommes `checksum` de `Cargo.lock` |
| `rust-std` 1.99.0 pour `aarch64-unknown-linux-gnu` (xz) | 31 757 040 octets | `rustup` contrôle `xz_hash` `1cf7e2ef58ed1cfa6f1adea18e155cbf254c1951f39e5f5e3376879f55fd64a4` (manifeste officiel) |
| Zig 0.17.0 `aarch64-macos` | 53 985 220 octets | SHA-256 `b607e9b9234790a008116ae5bdb71c6243b84b9fb42a53a9e70fde41c06c536a` (index officiel ziglang.org) |
| cargo-zigbuild 0.23.4 `aarch64-apple-darwin` | 1 035 284 octets | SHA-256 `ca2369ae1a9995542206a6c18dd372acff94dddeb9d7c07e09bfb7594197cea3` (fichier `.sha256` de la release) |
| **Total** | **≤ 120 547 493 octets (≈ 115 Mio)** | |

**Commandes (à lancer après accord ; aucune ne touche le Pi) :**

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cd <worktree codex/web-pwa>
# 1. Tests Rust du delta (garde d'origine, service statique) et de tout le serveur
cargo test --locked -p chill-server
cargo clippy --locked -p chill-server -- -D warnings
# 2. Binaire Linux ARM64 (glibc 2.28, même méthode que REL-PI-001 / REL-UX-API-001)
cargo zigbuild --release --locked -p chill-server --target aarch64-unknown-linux-gnu.2.28
shasum -a 256 target/aarch64-unknown-linux-gnu/release/chill-server     # → NEW_BINARY_SHA256
file target/aarch64-unknown-linux-gnu/release/chill-server              # ELF 64-bit ARM aarch64
# 3. Client web (inclut le décodeur WASM figé)
node --test web/test/*.test.js
./scripts/package-web-pi.sh                                              # → WEB_ARCHIVE_SHA256
```

Le **diff Rust** à revoir est petit : `crates/chill-server/src/web_static.rs` (nouveau), la garde `same_origin` dans `api.rs`, `lib.rs`, et leurs tests.

### Exécution du §3 (15 octobre 2026, après ton approbation)

| Étape | Résultat |
| --- | --- |
| Zig 0.17.0 | 53 985 220 octets, SHA-256 **identique** à l'index officiel (`b607e9b9…536a`) ; `zig version` = 0.17.0 |
| cargo-zigbuild 0.23.4 | 1 035 284 octets, SHA-256 **identique** (`ca2369ae…cea3`) |
| `rust-std` aarch64-linux | installé par `rustup` (hash du manifeste officiel vérifié par `rustup`) ; 31 757 040 octets annoncés |
| Crates du serveur | **109 crates, ≈ 24,2 Mo** mesurés par la croissance du cache (24 816 → 48 416 Kio, arrondi par blocs) ; `Cargo.lock` imposé par `--locked` (sommes `checksum` vérifiées par cargo) |
| **Total réellement téléchargé** | **≈ 110,9 Mo**, sous le plafond annoncé de 120 547 493 octets |
| Premier `cargo test` | **ÉCHEC de compilation** : le futur du middleware `same_origin` n'était plus `Send` (emprunts de la requête vivants à travers l'`await`) et une fermeture de test sans annotation de type. Corrigé en sortant les contrôles dans une fonction synchrone (`check_browser_origin_and_host`), commit `05e637d` |
| `cargo test --locked -p chill-server` | **101 passés, 0 échec** (dont `same_origin_browser_is_accepted_and_cross_site_is_refused`, les 3 tests de `web_static`, et l'ancien test de refus d'origine) ; 2 tests d'intégration passés |
| `cargo clippy --locked -p chill-server --all-targets -- -D warnings` | propre |
| Build ARM64 release | OK en 2 min 19 s ; un avertissement de l'éditeur de liens (déjà présent lors des livraisons précédentes) |
| **Essai de bout en bout du binaire livrable** : vrai binaire ARM64 + vraie archive web, lancés dans une VM Linux aarch64 (Ubuntu 24.04, glibc 2.39, boucle locale, données temporaires), mêmes vérifications que le script du Pi | **20 sur 20** : API sans en-tête 200 ; `same-origin` 200 ; `cross-site`, `same-site`, `Origin` étranger, `none` sur l'API : 403 ; page d'accueil 200 avec `<title>` et CSP ; `.wasm` en `application/wasm` (652 895 octets) ; `sw.js` en `no-cache` ; `.env`, `../`, `%2e%2e`, `SHA256SUMS` : 404 ; POST statique 405 ; hôte inattendu 403 |

Ce que cet essai **ne** prouve **pas** : l'exécution sur le vrai Pi (Debian 13, glibc 2.41, 905 Mio de RAM), les chemins `/usr/lib/chillcenter`, systemd, la durée d'interruption, ni la revue indépendante.

## 4. Revue indépendante (obligatoire avant exécution)

Un reviewer distinct de l'auteur (et sans droit d'édition sur le contenu revu) approuve **les octets exacts** de : `pi-update-web-operator.sh` (hash §2), le diff Rust, l'archive web finale et le hash du binaire. Points à contrôler : arrêt refusé si connexion établie ; sauvegarde hors ligne sous l'UID 997 ; copie de rollback sans écrasement ; remplacement atomique du binaire ; refus des liens symboliques/membres spéciaux dans l'archive ; `SHA256SUMS` couvrant tout l'arbre ; vérifications en TLS strict (jamais `-k`) ; **restauration automatique** (ancien binaire + ancien dossier web) à tout échec avant `completed=1` ; invariants config/unité/SPKI. Auto-test local déjà passé : `bash scripts/pi-update-web-operator.sh --self-test` → `parser_self_test=PASS`, `bash -n` OK ; `shellcheck` non disponible.

Enseignement conservé de l'échec V1–V3 (BUG-0051) : le script est envoyé par **stdin** (`bash -s`), sans citation imbriquée ni `eval`/`sed` sur des données distantes.

## 5. Exécution (seulement après revue + ton feu vert explicite)

Prérequis : une **connexion maître SSH stricte/batch** ouverte par toi (les anciens sockets `/private/tmp/chillcenter-ssh.*` ne sont plus utilisables comme preuve d'une session valide ; je ne contourne aucun échec d'agent).

```bash
CP=<chemin du ControlPath que tu as ouvert>;  HOST=<hôte Pi>
# a) Préflight en LECTURE SEULE (aucune écriture) : service actif, hash binaire installé, absence de connexions établies,
#    espace disque, curl présent, chemin du certificat public
ssh -S "$CP" "$HOST" 'sudo -n sha256sum /usr/lib/chillcenter/chill-server; systemctl is-active chillcenter; df -h /usr /var/tmp | tail -2; command -v curl; ls -l /var/lib/chillcenter/data/tls/certificate.pem'
# b) Copie des deux fichiers vers /var/tmp du Pi (écriture déclarée : 2 fichiers, ≈ 8 Mio)
scp -o ControlPath="$CP" target/aarch64-unknown-linux-gnu/release/chill-server "$HOST":/var/tmp/chill-server-web.new
scp -o ControlPath="$CP" artifacts/web-pi/chillcenter-web.tar "$HOST":/var/tmp/chillcenter-web.tar
# c) Exécution du script revu, par stdin
ssh -S "$CP" "$HOST" "sudo -n env NEW_BINARY_SHA256=<hash> WEB_ARCHIVE_SHA256=<hash> \
  CHILLCENTER_TLS_CERT_PATH=/var/lib/chillcenter/data/tls/certificate.pem CHILLCENTER_BASE_URL=https://192.168.1.42:8443 \
  /bin/bash -s --" < scripts/pi-update-web-operator.sh
# d) Nettoyage des deux fichiers /var/tmp (après vérification)
```

Le script imprime seulement : date UTC, PID, hashes, chemin de sauvegarde, dossier web précédent. Il ne lit, ne hache ni n'affiche aucune clé privée.

## 6. Vérification après déploiement (lecture seule)

Automatique dans le script (TLS strict, depuis le Pi) : `/api/2026-10-14/server` → 200 sans en-tête (clients natifs), 200 avec `Sec-Fetch-Site: same-origin`, **403** avec `cross-site` et avec un `Origin` étranger ; `/` contient `<title>Chill Center</title>` ; `vendor/mpeg4/mpeg4-decoder.wasm` servi en `application/wasm` ; `/.env` → 404 ; traversée `/../etc/passwd` ≠ 200.

Par moi ensuite (Mac, lecture seule, bail volatil libéré) : sonde Node déjà éprouvée (certificat, `/server`, catalogue, Range MKV/MP4/AVI, démuxeurs, décodeur WASM sur de vrais fichiers) — et, pour **voir l'interface dans un navigateur**, il faut que le navigateur accepte le certificat auto-signé : soit une exception pour `192.168.1.42:8443`, soit l'ajout du certificat public du foyer au trousseau macOS (modification de la confiance système : **geste à faire par toi**). Sans confiance système, Service Worker et installation PWA seront refusés par plusieurs navigateurs ; la page elle-même fonctionnera avec l'exception.

## 7. Retour arrière

**Automatique** : tout échec du script après l'arrêt du service remet l'ancien binaire (copie vérifiée par hash), supprime le nouveau `web/` et restaure l'ancien s'il existait, puis redémarre le service.

**Manuel** (service arrêté, sous root, base et config intactes) :

```bash
systemctl stop chillcenter
install -o root -g root -m 0755 /root/chillcenter-rollback/chill-server-1eb24e97bcdf /usr/lib/chillcenter/chill-server
sha256sum /usr/lib/chillcenter/chill-server        # doit afficher 1eb24e97…c181
rm -rf --one-file-system /usr/lib/chillcenter/web   # ou : mv -T /root/chillcenter-rollback/web-<horodatage> /usr/lib/chillcenter/web
systemctl start chillcenter
```

Aucune migration SQLite n'est introduite par ce lot : l'ancien binaire reste compatible avec les données (à confirmer par la revue du diff). La sauvegarde privée fraîche n'est nécessaire qu'en cas d'altération des données, ce qui n'est pas prévu. **Déclencheurs** : hash inattendu, sauvegarde en échec, service non démarré ou API critique en échec, toute vérification du §6 en échec, régression de confinement.

## 8. Ce qui n'est pas prouvé

Exécution du script opérateur sur un vrai Pi (`NOT_RUN`), durée réelle d'interruption, revue indépendante du script et du diff (`NOT_DONE`), lecture réelle depuis un navigateur sur le Pi, performances sur appareils faibles, Service Worker sur certificat auto-signé. Compilation, tests Rust, build ARM64 et essai de bout en bout Linux : faits (§3).

Rejouer l'essai de bout en bout : `colima start` puis `colima ssh -- <repo>/scripts/web-e2e-linux.sh <repo>` (le binaire et `artifacts/web-pi/chillcenter-web.tar` doivent exister), puis `colima stop`.

## Revue indépendante n° 1 (15 octobre 2026)

Verdict **CHANGES_REQUESTED** (script V5, trois défauts MEDIUM S1–S3), Rust APPROVED_WITH_CONDITIONS, artefacts APPROVED : [rapport verbatim](../reviews/web-pi-deploy-v5-review-1.md). Le script `8d47e9df…` ne doit donc **pas** être exécuté ; sa correction produira un nouveau hash et une nouvelle revue.
