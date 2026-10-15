# Déploiement du client web sur le Pi — artefacts construits et testés, déploiement NON exécuté

Révision `CC-WEB-DEPLOY-001` r4, 15 octobre 2026, branche `codex/web-pwa`, source du binaire `3a0d53a4d88c1ae903cc83194e4430af56a246a3` (le script et le banc d'essai sont au commit `e2e1c8bdd07d12102184e727790a67e241e2ef90`, qui ne modifie pas `crates/`). **Rien n'a été exécuté sur le Pi ni copié vers lui.** Ce document est une proposition soumise à revue indépendante puis à ton feu vert explicite (AGENTS : pas de déploiement sur un matériel ou un réseau supposé ; un seul auteur par chemin, le reviewer n'édite pas).

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
| Script opérateur candidat **V7** `scripts/pi-update-web-operator.sh` (remplace V6 `ba9777cd…` refusé par la revue n° 2 et V5 `8d47e9df…` refusé par la revue n° 1) | 17 609 octets, SHA-256 **`b61b9fd6274f19669dcddabcf180e443b865f8f3777529b63d57ebad07e64872`** — **toute modification, même un commentaire, annule la revue** |
| **Archive web finale** (`scripts/package-web-pi.sh`, sans métadonnées macOS) | `artifacts/web-pi/chillcenter-web.tar`, **1 099 264 octets**, 54 fichiers, SHA-256 **`486e8eb248ab74169af953ef5e6a125d26adde7b157e67c600c93713513b0756`**. Le tar n'est pas reproductible octet pour octet (horodatages) : **ce fichier exact** est celui à copier ; le contenu (build web `387b98d9b0ad`) est reproductible |
| Décodeur MPEG-4 embarqué | `mpeg4-decoder.wasm` SHA-256 `453fb7b171fde50d1306a57793ebac571459a7c324d28bd8b4f6c3db6b7bda53` (652 895 o), `mpeg4-decoder.js` `64d97f891594c386652cf3f783dfbdaf65d358ddb0bf1be96f1fbe3b2d3be827` ; FFmpeg 9.0.2 `8c385028…002e`, emsdk 6.0.12 `sha256:a7883da4…8c98` |
| **Nouveau binaire serveur** (`cargo zigbuild --release --locked -p chill-server --target aarch64-unknown-linux-gnu.2.28`) | `target/aarch64-unknown-linux-gnu/release/chill-server`, **14 601 992 octets**, ELF 64-bit ARM aarch64 dynamique, strippé, SHA-256 **`241205689e656b949d0a9a6058d74408f1a6b70ac2446f0daf3c014b1b30ac29`** → `NEW_BINARY_SHA256` |
| Source | binaire construit à partir de `3a0d53a4d88c1ae903cc83194e4430af56a246a3` ; `git diff 3a0d53a HEAD -- crates` est vide |

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

### Reprise après la revue n° 1 (15 octobre 2026)

La [revue n° 1](../reviews/web-pi-deploy-v5-review-1.md) a rendu CHANGES_REQUESTED sur le script V5. Traitement, finding par finding :

| Finding | Traitement dans V6 / Rust |
| --- | --- |
| S1 `curl \| grep -q` sous `pipefail` | page capturée dans une variable puis comparée ; cas ajouté à l'auto-test (page de 40 Ko) |
| S2 récupération fragile | `set +e`, signaux **interceptés** (`trap : …`, pas ignorés : un signal ignoré serait hérité par `systemctl`, défaut trouvé par la simulation), chaque étape protégée, messages `RECOVERY:`, ancien binaire non restaurable ⇒ le dossier web n'est pas retiré (état cohérent) et le service est redémarré |
| S3 pas de sonde avant l'arrêt | sonde TLS stricte du service actuel **avant** tout arrêt : un SAN, un port ou un CA erroné échoue sans interruption (scénario B) |
| S4 contrôle de rollback tardif | vérifié en préflight ; une copie existante est réutilisée **seulement** si elle est identique (scénarios E et F) |
| S5 `web_swapped` tardif | drapeau `web_moved` posé juste après le déplacement de l'ancien dossier (scénario D) |
| S6 TOCTOU de l'archive | **risque accepté, documenté** dans l'en-tête du script (le seul écrivain possible est déjà équivalent à root) |
| S7 `--strict`, exemption exacte | `sha256sum -c --quiet --strict` ; seul `./SHA256SUMS` de la racine est exempté |
| S8 `ss` fail-open | `ss` requis en préflight ; un échec de `ss` interdit l'arrêt |
| S9 espace disque, `sudo env` | contrôle de 64 Mio libres sur trois emplacements ; `sudo -n env true` ajouté au préflight §5a |
| R1 routes API inconnues | `404` nu conservé pour tout chemin `/api/…`, donc `ROUTE_NOT_FOUND` JSON inchangé pour toute méthode (test ajouté) |
| R2 CSP | `form-action 'self'` ajouté |
| R3 plafond mémoire | fichiers statiques limités à 8 Mio |
| R6 tests | Origin `null`, non UTF-8, étranger, casse ; `cross-site` et `none`+POST sur `/` ; HEAD sans corps ; POST statique 405 ; lien symbolique de **répertoire** ; routes API inconnues pour 4 méthodes |

Résultats après corrections :

| Vérification | Résultat |
| --- | --- |
| `cargo test --locked -p chill-server` | **102 passés**, 0 échec ; 2 tests d'intégration passés |
| `cargo clippy --locked -p chill-server --all-targets -- -D warnings` | propre |
| Build ARM64 (`cargo zigbuild … .2.28`) | OK en 1 min 9 s ; `file` : ELF aarch64 strippé |
| `node --test web/test/*.test.js` | 49 passés |
| Essai binaire + archive sous Linux aarch64 (`scripts/web-e2e-linux.sh`) | **20 sur 20** |
| **Simulation du déploiement** (`scripts/pi-operator-simulation.sh`, VM Ubuntu 24.04 aarch64, GNU tar 1.35, coreutils 9.4, curl 8.5.0, root, vrai ancien binaire `1eb24e97…` comme service via un `systemctl` factice, script envoyé par stdin comme en production) | **34 PASS, 0 FAIL** : A déploiement nominal (binaire, web, rollback, sauvegarde, invariants, aucun résidu) ; B mauvaise URL refusée **avant** l'arrêt ; C échec de vérification ⇒ ancien binaire restauré, `web/` retiré, service actif ; D idem avec un ancien `web/` qui revient ; E redéploiement avec copie de rollback réutilisée et ancien web conservé ; F copie de rollback douteuse refusée ; G connexion cliente établie ⇒ refus (42) ; H archive avec lien symbolique refusée |

Limites honnêtes de cette simulation : l'UID de service 997 était pris dans la VM ; la copie testée du script ne diffère de l'original **que** par ce nombre (2 lignes, substitution mécanique `sed`). Le « service » est piloté par un `systemctl` factice, pas par systemd ; l'environnement est Ubuntu 24.04, pas Debian 13 ; mémoire et CPU du Pi 3B+ non représentatifs. Un premier essai de simulation a échoué sur le défaut de signaux ci-dessus et sur des artefacts du banc (arrêt factice trop court, processus d'un essai interrompu) : corrigés, rien n'a été caché.

### Reprise après la revue n° 2 (15 octobre 2026)

La [revue n° 2](../reviews/web-pi-deploy-v6-review-2.md) (nouvel agent, qui a rejoué lui-même les essais en VM) a rendu CHANGES_REQUESTED sur le seul script V6 : Rust et artefacts APPROVED, bancs d'essai APPROVED_WITH_CONDITIONS. Son constat N1 a été reproduit par lui : SIGPIPE (sortie fermée, par exemple session SSH coupée) tuait la récupération avant la première restauration, avec `rc=141`, nouveau binaire non vérifié laissé installé, et, dans le pire cas, service laissé arrêté.

| Finding | Traitement dans V7 |
| --- | --- |
| **N1 (MEDIUM)** SIGPIPE tue la récupération | `trap '' PIPE` dès le début du script. Scénario **I** ajouté à la simulation (sortie fermée pendant un échec de vérification) : code 1 (pas 141), ancien binaire restauré, `web/` retiré, service actif et répondant |
| N2 `trap :` vs `trap ''` | retour à `trap '' HUP INT TERM` dans la récupération (ni elle ni les `install`/`mv` qu'elle lance ne peuvent être interrompus, y compris par un SIGHUP du pty de `sudo`). Le défaut d'héritage rencontré venait du **banc** (faux `systemctl` qui lance le service comme enfant) : corrigé là, avec `env --default-signal=HUP,INT,TERM,PIPE` |
| N3 assertions négatives inertes | helper `must_fail` ; **preuve par mutation** (`scripts/selftest-mutation.sh`) : un validateur d'archive qui accepte tout et un analyseur de sauvegarde qui accepte tout font maintenant **échouer** l'auto-test (rc=1, message `SELF-TEST FAILED`) |
| N4 `! systemctl is-active` inopérant | remplacé par un `if … then exit 1` explicite |
| N5 échec muet | `set -E` et `trap … ERR` : la sortie nomme la ligne et la commande (`ERR line N: …`), testé par le scénario J |
| N6 état après récupération | après restauration, sonde TLS stricte du service et message dédié ; message explicite si le service n'a pas pu être arrêté |
| N7 durcissement | `curl -q --noproxy '*'` ; outils requis vérifiés en préflight (`openssl runuser seq install mktemp sha256sum tar find du stat df awk`) ; **l'URL doit désigner un port sur lequel `chill-server` écoute réellement** (contrôle `ss -ltnp`) ; S6 (archive relue) et le seuil de 64 Mio restent des risques acceptés/heuristiques, documentés |
| N8 scénarios manquants | ajoutés : **I** SIGPIPE, **J** sauvegarde impossible après l'arrêt (le service doit revenir, la sortie nomme la commande), **N** service déjà arrêté (refus avant tout, le script ne le démarre pas) |

Résultat de la simulation étendue (VM Ubuntu 24.04 aarch64, GNU tar 1.35, coreutils 9.4, curl 8.5.0, vrai ancien binaire, script par stdin) : **45 PASS, 0 FAIL** (scénarios A à J et N). Un premier passage a signalé un échec **de mon scénario J** (j'avais rendu le dossier de données illisible pour le service, qui ne pouvait plus redémarrer ; le script l'avait dit honnêtement, `SERVICE IS NOT ACTIVE`) : le scénario ne sabote désormais que le dossier des sauvegardes. Le Rust (`3a0d53a`), le binaire et l'archive web **ne changent pas** : hashes inchangés.

## 4. Revue indépendante (obligatoire avant exécution) — la la revue n° 3 doit porter sur les octets V7 ci-dessus

Un reviewer distinct de l'auteur (et sans droit d'édition sur le contenu revu) approuve **les octets exacts** de : `pi-update-web-operator.sh` (hash §2), le diff Rust, l'archive web finale et le hash du binaire. Points à contrôler : arrêt refusé si connexion établie ; sauvegarde hors ligne sous l'UID 997 ; copie de rollback sans écrasement ; remplacement atomique du binaire ; refus des liens symboliques/membres spéciaux dans l'archive ; `SHA256SUMS` couvrant tout l'arbre ; vérifications en TLS strict (jamais `-k`) ; **restauration automatique** (ancien binaire + ancien dossier web) à tout échec avant `completed=1` ; invariants config/unité/SPKI. Auto-test local : `bash scripts/pi-update-web-operator.sh --self-test` → `parser_self_test=PASS`, `bash -n` OK ; `shellcheck` non disponible. Preuve d'exécution réelle sous Linux : simulation 45/45 (§3).

Enseignement conservé de l'échec V1–V3 (BUG-0051) : le script est envoyé par **stdin** (`bash -s`), sans citation imbriquée ni `eval`/`sed` sur des données distantes.

## 5. Exécution (seulement après revue + ton feu vert explicite)

Prérequis : une **connexion maître SSH stricte/batch** ouverte par toi (les anciens sockets `/private/tmp/chillcenter-ssh.*` ne sont plus utilisables comme preuve d'une session valide ; je ne contourne aucun échec d'agent).

```bash
CP=<chemin du ControlPath que tu as ouvert>;  HOST=<hôte Pi>
# a) Préflight en LECTURE SEULE (aucune écriture) : sudoers autorise `env`, service actif, hash binaire installé,
#    espace disque, curl et ss présents, chemin du certificat public
ssh -S "$CP" "$HOST" 'sudo -n env true && echo sudo_env_ok; sudo -n sha256sum /usr/lib/chillcenter/chill-server; systemctl is-active chillcenter; df -h /usr /var/tmp | tail -2; command -v curl ss; ls -l /var/lib/chillcenter/data/tls/certificate.pem'
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

Exécution du script opérateur sur le vrai Pi (`NOT_RUN` ; seule la simulation Linux/VM a eu lieu), durée réelle d'interruption, revue indépendante n° 3 des octets V7 (`À FAIRE`), lecture réelle depuis un navigateur sur le Pi, performances sur appareils faibles, Service Worker sur certificat auto-signé. Compilation, tests Rust, build ARM64 et essai de bout en bout Linux : faits (§3).

Rejouer l'essai de bout en bout : `colima start` puis `colima ssh -- <repo>/scripts/web-e2e-linux.sh <repo>` (le binaire et `artifacts/web-pi/chillcenter-web.tar` doivent exister), puis `colima stop`.

## Revue indépendante n° 2 (15 octobre 2026)

Verdict **CHANGES_REQUESTED** sur le script V6 (N1 SIGPIPE, reproduit par le reviewer), Rust APPROVED, artefacts APPROVED : [rapport verbatim](../reviews/web-pi-deploy-v6-review-2.md). Corrigé dans V7 (voir « Reprise après la revue n° 2 »). Le script V7 `b61b9fd6…` n'a **pas encore de revue** : il ne doit pas être exécuté.

## Revue indépendante n° 1 (15 octobre 2026)

Verdict **CHANGES_REQUESTED** (script V5, trois défauts MEDIUM S1–S3), Rust APPROVED_WITH_CONDITIONS, artefacts APPROVED : [rapport verbatim](../reviews/web-pi-deploy-v5-review-1.md). Le script `8d47e9df…` ne doit donc **pas** être exécuté ; sa correction produira un nouveau hash et une nouvelle revue.
