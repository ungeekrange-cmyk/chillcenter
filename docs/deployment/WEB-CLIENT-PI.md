# Déploiement du client web sur le Pi — déployé le 15 octobre 2026 (voir « Exécution du 15 octobre 2026 »)

Révision `CC-WEB-DEPLOY-001` r4, 15 octobre 2026, branche `codex/web-pwa`, source du binaire : merge `7204e19` puis `1a4019f` (condition `Host` du port 443 ; ancienne source `3a0d53a4`, périmée) (le script et le banc d'essai sont au commit `e2e1c8bdd07d12102184e727790a67e241e2ef90`, qui ne modifie pas `crates/`). **Rien n'a été exécuté sur le Pi ni copié vers lui.** Ce document est une proposition soumise à revue indépendante puis à ton feu vert explicite (AGENTS : pas de déploiement sur un matériel ou un réseau supposé ; un seul auteur par chemin, le reviewer n'édite pas).

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
| Binaire actuellement installé sur le Pi (build `c610ea9`, constaté au préflight du 15 oct.) | `cef54cd0e8baf45f70ee0f57584001521c7dcd705d4f8ab351eb8074f524b3c3` → `OLD_BINARY_SHA256` (le V4 `1eb24e97…` des revues 1 à 3 n'est plus celui du Pi) |
| Ancien rollback R7 (déjà sur le Pi) | `/root/chillcenter-rollback/chill-server-r7-9a17ffe55e3d`, `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6` |
| Certificat public du Pi (lu le 15 oct.) | SPKI SHA-256 `3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df`, valide du 14/10/2026 au 14/04/2028 ; `server_id` `885f7713-b8c6-4891-a8d1-c1410d5f1537` |
| Script opérateur candidat **V8** `scripts/pi-update-web-operator.sh` (= V7 `b61b9fd6…` approuvé par la revue n° 3, dont le hash de l'ancien binaire, auparavant codé en dur, devient le paramètre obligatoire `OLD_BINARY_SHA256` ; **V8 n'a pas encore été relu** : revue n° 4 requise) | 17 916 octets, SHA-256 **`dcf1866111ef756b53113adce323f90960f5edbf00835204da78097fd6528e29`** — **toute modification, même un commentaire, annule la revue** |
| **Archive web finale** (`scripts/package-web-pi.sh`, sans métadonnées macOS) | `artifacts/web-pi/chillcenter-web.tar`, **1 125 376 octets**, 60 membres : 56 fichiers dont `SHA256SUMS`, 4 répertoires, SHA-256 **`894fbe952a58bdae01527e8d8fa646e87977b9f1aea5bd996d770bacb71f7118`**. Le tar n'est pas reproductible octet pour octet (horodatages) : **ce fichier exact** est celui à copier ; contenu : build web `1556174bd809` |
| Décodeur MPEG-4 embarqué | `mpeg4-decoder.wasm` SHA-256 `453fb7b171fde50d1306a57793ebac571459a7c324d28bd8b4f6c3db6b7bda53` (652 895 o), `mpeg4-decoder.js` `64d97f891594c386652cf3f783dfbdaf65d358ddb0bf1be96f1fbe3b2d3be827` ; FFmpeg 9.0.2 `8c385028…002e`, emsdk 6.0.12 `sha256:a7883da4…8c98` |
| **Nouveau binaire serveur** (`cargo zigbuild --release --locked --offline -p chill-server --target aarch64-unknown-linux-gnu.2.28`) | `target/aarch64-unknown-linux-gnu/release/chill-server`, **14 765 736 octets**, ELF 64-bit ARM aarch64 dynamique, strippé, SHA-256 **`a7ab7b07633c916c358f1656aa8f154b207185d5ac306c374d2a7a52b0c7d527`** → `NEW_BINARY_SHA256` |
| Source | binaire construit à partir du merge `7204e19` (codex/player-streaming `a59f5e6` dans codex/web-pwa) ; le code Rust ne change plus après ce commit |

## Reprise après intégration de `codex/player-streaming` (15 octobre 2026)

**Pourquoi.** Le préflight du 15 octobre a montré que le Pi exécute le build `c610ea9` (`cef54cd0e8baf45f70ee0f57584001521c7dcd705d4f8ab351eb8074f524b3c3`) et non le `1eb24e97…` attendu par le script V7. Décision de l'utilisateur : intégrer la branche Mac `codex/player-streaming` (jusqu'à `a59f5e6`) dans `codex/web-pwa`, tout retester, rendre le hash de l'ancien binaire paramétrable. Rien n'a été exécuté sur le Pi depuis.

| Étape | Résultat réellement obtenu |
| --- | --- |
| Merge `a59f5e6` | commit `7204e19`, sans conflit textuel (`api.rs` compris) ; ADR web renumérotés 0014/0015/0016 car 0012/0013 appartiennent désormais au client Mac (voir `docs/architecture/decisions/INDEX.md`) |
| Pont web (`web/src/bridge.js`) | ajout de `playlist_artwork`, `saga_artwork`, `save_player_settings` et `player_presentation` (plein écran au lancement, défaut `true`, stocké localement) ; la demande de plein écran refusée par le navigateur n'est plus une erreur de lecture. **Écart assumé** : le client Mac sert désormais le catalogue depuis un miroir SQLite local (ADR-0013) ; le client web continue de lire l'API serveur, inchangée (ajout seul), et n'a pas de miroir |
| `node --test web/test/*.test.js` | 50 tests, 0 échec |
| `cargo test --locked --offline -p chill-server` | 104 + 2 tests, 0 échec |
| `cargo clippy --locked --offline -p chill-server --all-targets -- -D warnings` | sans avertissement |
| Build ARM64 | Zig 0.17.0 et cargo-zigbuild 0.23.4 déjà présents dans `/private/tmp/chillcenter-linux-cross.1M5r5u/venv` (aucun nouveau téléchargement) ; binaire et archive : voir §2 |
| Simulation sur VM jetable (Colima Ubuntu 24.04 aarch64, ancien binaire réel `c610ea9`) | **52 PASS, 0 FAIL** (scénarios A–J, N, P) ; P = paramètre `OLD_BINARY_SHA256` vide ou erroné refusé avant tout arrêt. Première passe : 1 FAIL, mon attente de code de sortie (2) était fausse, `${VAR:?}` sort en 1 comme pour les autres variables obligatoires ; attente corrigée, script inchangé |
| Essai Linux aarch64 (`scripts/web-e2e-linux.sh`) | 20 PASS, 0 FAIL |
| Preuve de mutation de l'auto-test | les deux mutants sont détectés, l'original passe |

**Changements du script V8 par rapport à V7** : suppression de `old_expected='1eb24e97…'` et des noms `chill-server-1eb24e97bcdf` codés en dur ; `OLD_BINARY_SHA256` est obligatoire (64 hex, différent de `NEW_BINARY_SHA256`), le nom du rollback et le préfixe des temporaires en sont dérivés (12 premiers hex). Aucune autre ligne modifiée. Le préflight lecture seule affiche désormais l'attendu `cef54cd0…`.

**Statut.** La revue indépendante n° 4 (octets V8 `dcf18661…`) conclut **APPROVED_WITH_CONDITIONS** : [rapport verbatim](../reviews/web-pi-deploy-v8-review-4.md). Conditions documentaires traitées : MEDIUM-1 (hashes périmés du runbook, étapes 5 et 6), LOW-1 (retour arrière du plan §7), LOW-2 (mentions V7 du runbook), LOW-3 (décompte du tar). **Non traités, acceptés** : MEDIUM-2 (la simulation ne détecte pas la casse du nettoyage du temporaire de rollback, impact faible), LOW-4 (cache ≤ 1 h des fichiers statiques non `no-cache`, lecture intégrale en mémoire) et les résiduels hérités LOW-2/LOW-3 de la revue 3. Aucune exécution sur le Pi sans nouveau feu vert explicite et connexion maître.

## Ajout du port 443 (15 octobre 2026) : ADR-0017

Demande de l'utilisateur : joindre le client web sur le port normal. Décision : redirection réseau 443 → 8443 sur le Pi, le serveur acceptant le port par défaut dans `Host` ([ADR-0017](../architecture/decisions/ADR-0017-web-client-standard-https-port.md)). Le service ne gagne ni port ni capacité ; sa configuration et son unité ne changent pas.

| Élément | Résultat réellement obtenu |
| --- | --- |
| Rust | `check_browser_origin_and_host` accepte le port de `listen` **ou** 443 ; nom toujours exigé dans `tls_names` ; nouveau test (105 tests de la bibliothèque, plus 2 d'intégration ; 0 échec), clippy `-D warnings` propre |
| Nouveau binaire ARM64 | **14 765 736 octets**, SHA-256 `a7ab7b07633c916c358f1656aa8f154b207185d5ac306c374d2a7a52b0c7d527` : voir §2 (valeur de référence) ; **remplace** `b80408da…` relu par la revue n° 4 |
| Essai Linux aarch64 | 23 PASS, 0 FAIL (3 contrôles ajoutés : `Host` sans port, `:443`, autre port refusé) |
| Simulation V8 | 52 PASS, 0 FAIL avec le nouveau binaire ; script V8 inchangé (`dcf18661…`) |
| Relevé du pare-feu du Pi (lecture seule, exécuté le 15 oct. 15:22 UTC via ta connexion maître) | `scripts/pi-firewall-probe-readonly.sh` (19 lignes, SHA-256 `7116d03bab4b6117c9202ca784eb0560566678e08680ecb56cabcfa5acb2f2ce`). Constat : Debian 13, noyau 6.18.50+rpt-rpi-v8, `nft` 1.1.3 présent, `nftables.service` **désactivé**, jeu de règles **vide**, ni iptables ni ufw, rien n'écoute sur 80/443, modules `nf_tables`/`nft_nat`/`nft_redir` disponibles (non chargés), `/etc/nftables.conf` par défaut (`flush ruleset`, donc **ni édité ni activé**). Préflight du même jour : tout conforme (binaire `cef54cd0…`, service actif, certificat `3018fb4d…`, `web_dir` absent, aucun fichier entrant) |
| Script d'installation de la règle 443 → 8443 | `scripts/pi-redirect-443-operator.sh`, SHA-256 `3e7f6d4f8b0bdfa33af09f7031f7862ddca992426507dbf0cd498a63fe1f4ef4` : table nftables dédiée `ip chillcenter_web` chargée par sa propre unité `chillcenter-redirect.service` ; installation, `--remove`, `--self-test`. **Simulation** `scripts/pi-redirect-simulation.sh` (VM Ubuntu 24.04 aarch64, systemd et nftables réels, client dans un espace réseau séparé, vrai binaire et vraie archive) : **43 PASS, 0 FAIL** (40 d'abord ; R3b « autre adresse locale non redirigée » et R9 « service arrêté : refus » ajoutés après la revue n° 5, car les mutants correspondants n'étaient pas détectés ; vérifié : R3b détecte le mutant sans `ip daddr`). Elle a trouvé un défaut réel du script (temporaires laissés si l'essai à blanc `nft -c` échoue), corrigé par un nettoyage à la sortie |
| Revue n° 5 (15 octobre 2026) | **APPROVED_WITH_CONDITIONS** pour le script de redirection `3e7f6d4f…` ; **APPROVED** pour le delta Rust et le binaire `a7ab7b07…` : [rapport verbatim](../reviews/web-pi-redirect-443-review-5.md). Conditions C1 (contrôle de l'adresse primaire + test de bout en bout obligatoire), C2 (hash périmé du runbook) et C3 (risques résiduels) traitées dans le runbook, étape 8 ; octets du script inchangés. La revue n° 4 reste valable pour le script V8 |

## Exécution du 15 octobre 2026 (feu vert de l'utilisateur pour les deux actions)

Tout ce qui suit a réellement été exécuté, via la connexion maître de l'utilisateur (`/private/tmp/chillcenter-ssh.pkLA1S/control`), après contrôle local des hashes (tous conformes) et un préflight frais (conforme, aucune connexion cliente établie).

| Étape | Résultat observé |
| --- | --- |
| Copie dans `/var/tmp` | binaire `a7ab7b07…` et archive `894fbe95…` : hashes relus sur le Pi, identiques |
| Script V8 (`dcf18661…`, par stdin, `OLD_BINARY_SHA256=cef54cd0…`) | **code de sortie 0** à 15:44:49 UTC ; `installed_hash=a7ab7b07…`, `rollback_hash=cef54cd0…`, `web_archive_hash=894fbe95…`, `web_previous=none`, sauvegarde privée `/var/lib/chillcenter/data/backups/93f3dee3-6297-4319-a455-a0ece8a98437` ; copie de rollback `chill-server-cef54cd0e8ba` créée |
| Préflight après déploiement (lecture seule) | binaire `a7ab7b07…`, service actif (PID 9457, `NRestarts=0`), `web_dir` présent, configuration `ea24f2f0…`, unité `76f7d349…` et certificat `3018fb4d…` **inchangés**, 6 sauvegardes |
| Nettoyage | `/var/tmp/chill-server-web.new` et `/var/tmp/chillcenter-web.tar` supprimés |
| Vérification depuis le Mac, TLS strict (certificat du Pi comme seule ancre, empreinte `3018fb4d…` vérifiée) | `/` 200 `text/html` 19 604 o ; `/api/2026-10-14/server` 200 ; cross-site 403 ; décodeur `.wasm` 200 `application/wasm` 652 895 o ; `sw.js` 200 ; profils lus (`server_id` `885f7713…`) |
| Redirection 443 (`pi-redirect-443-operator.sh` `3e7f6d4f…`) | contrôles préalables : `wlan0` n'a que `192.168.1.42/24` (primaire), `/etc/chillcenter` `750 root:chillcenter` ; `--self-test` PASS sur le Pi ; installation **code 0** à 15:45:51 UTC, `unit_state=enabled/active`, `rules_sha256=d3da7f4f…`, `unit_sha256=c3bf9e7d…`, service toujours actif |
| Vérification de bout en bout sur 443 depuis le Mac | certificat servi : empreinte `3018fb4d…` ; `https://192.168.1.42/` 200 ; API same-origin 200 (profils lus) ; cross-site 403 ; `:8443` répond toujours 200 |

**Non vérifié** : ouverture dans un vrai navigateur (le navigateur intégré à l'application a refusé la navigation vers `https://192.168.1.42/` ; installation en PWA et Service Worker sur certificat auto-signé non testés) ; lecture réelle d'un média depuis un navigateur ; comportement de la règle après un **redémarrage réel** du Pi (l'unité est activée ; la persistance n'a été prouvée qu'en VM) ; test de bascule/retour arrière sur le Pi (non nécessaire, non exécuté).

## Écoute HTTP non chiffrée (15 octobre 2026) : ADR-0018

Décision de l'utilisateur après refus de l'autorité privée et sans domaine : « fais du HTTP non chiffré sinon ». Détails, risques acceptés et rollback : [ADR-0018](../architecture/decisions/ADR-0018-web-client-plain-http-listener.md). **Rien de ceci n'est déployé** ; le Pi était d'ailleurs injoignable (ping 100 % de pertes, connexion maître SSH tombée) à partir d'environ 17 h le 15 octobre, sans action de ma part sur lui depuis la vérification de la redirection 443.

| Élément | Résultat réellement obtenu |
| --- | --- |
| Rust | `http_listen` optionnel, second listener, garde d'origine selon le schéma ; **110 tests** de la bibliothèque, clippy `-D warnings` propre |
| Web | repli `getRandomValues` à la place de `crypto.randomUUID` (absent hors contexte sécurisé) ; **51 tests Node** |
| Nouveau binaire ARM64 / archive web | `ffcc634923f637331cac25fec1e5283b062ef5e2943778bcab59460b6269b0f7` (14 890 920 o) / `24839b3d81f21632818903c5c097471a98086748303ac304477f23e480d00df2` : **à remplacer** par ceux du §2 si ce chantier est déployé |
| Essai Linux aarch64 (`web-e2e-linux.sh`) | **40 PASS, 0 FAIL** (contrôles HTTP, routes sensibles refusées en HTTP clair, 16 connexions muettes) |
| Test en navigateur réel, **contexte non sécurisé** (navigateur intégré, `http://192.168.1.14:18080`, vrai serveur sur le Mac) | `isSecureContext` faux, pas de Service Worker, `crypto.randomUUID` absent : création de profil (POST 201), réglage de bibliothèque (PUT 200), scan, catalogue ; **lecture MP4** (image à 5,8 s) et **AVI** (décodeur WebAssembly, image à 4,8 s) ; bail acquis, flux d'événements, heartbeat et progression en 200 |
| `pi-enable-http-operator.sh` (`26a8df5832c83a2fbc659ddc8ab08ff41a55a59395e2e2fb43348ca82d938e57`) | simulation VM (systemd réel) **49 PASS, 0 FAIL** ; la simulation a révélé 2 défauts réels corrigés avant revue : contrôle des connexions établies sans `-p`, mode `--revert` sans hash distinct de la sauvegarde |
| `pi-redirect-80-operator.sh` (`9577ad88bdbe1c0b668db5d0344e19d91676a2de5a999f48bec410fb66f51dd7`) | dérivé mécaniquement du script du 443 relu par la revue n° 5 ; simulation VM **42 PASS, 0 FAIL** |
| Revue indépendante n° 6 (commit `47d42ec`) | **APPROVED_WITH_CONDITIONS** : [rapport verbatim](../reviews/web-pi-http-listener-review-6.md). Constats traités par un nouveau lot de changements (binaire et scripts **remplacés**, hashes ci-dessus et §2) : saturation par 16 connexions muettes (mesurée) → `FirstByteAcceptor` (délai de 5 s sur le premier octet ; hyper ne démarre son délai d'en-têtes qu'après un octet) + `http1_only` ; restauration automatique rendue atomique, déclenchée aussi par HUP/INT/TERM, hash de la sauvegarde vérifié ; routes à secrets ou archives (`/admin/tmdb*`, `/admin/exports*`, `/admin/imports*`) refusées en HTTP clair (`HTTPS_ONLY`) ; commentaire du script de redirection 80 corrigé |
| Revue indépendante n° 7 (contrôle, commit `2f26896`) | **REFUSED** pour `pi-enable-http-operator.sh` `fb88ceff…` : [rapport verbatim](../reviews/web-pi-http-listener-review-7.md). Défaut HIGH reproduit : dans `revert_now`, un `echo >&2` vers un stderr cassé (session SSH tombée) fait sortir le script sous `set -e` avant la restauration, ce qui laisse la configuration neuve et le service arrêté. Corrigé (`set +e` en tête de la récupération) ; le même motif existait dans `failed()` du script de redirection 80, corrigé aussi ; scénarios E5d et R7b ajoutés, et vérifié que E5d détecte le défaut sur un mutant (2 FAIL). **Le script du port 443, déjà exécuté une fois le 15 octobre, avait le même motif** dans `failed()` : corrigé aussi (`170f41a7…`, simulation 46/46 avec le nouveau scénario R7b) ; l'exécution du 15 octobre a utilisé les octets de la revue n° 5 (`3e7f6d4f…`), qui n'ont pas échoué. APPROVED : binaire `ffcc6349…`, archive, `pi-redirect-80` (diff de commentaire seul) |
| Après la revue n° 6 | 110 tests Rust, clippy propre ; essai Linux **40/40** (dont « 16 connexions muettes fermées, service disponible », TMDB/exports refusés en HTTP) ; simulation d'activation **46/46** (récupération : propriétaire/droits restaurés, signal TERM dans la fenêtre, insertion doublée refusée) ; simulations de redirection 42/42 (port 80) et 43/43 (port 443). **Une revue de contrôle (n° 7) sur ces octets reste requise** avant déploiement |

## Exécution du 15 octobre 2026, soir : binaire à écoute HTTP, activation, redirection du port 80

Après les revues n° 6 à 8 (n° 8 : APPROVED sur les octets déployés) et le retour du Pi (injoignable d'environ 16 h à 20 h UTC, redémarré). Connexion maître `/private/tmp/chillcenter-ssh.bETCib/control`. Observé :

| Étape | Résultat |
| --- | --- |
| Préflight lecture seule (20:18 UTC) | binaire `a7ab7b07…`, service actif (redémarré à 21:03 IST), configuration `ea24f2f0…`, unité `76f7d349…`, certificat `3018fb4d…` inchangés, aucune connexion cliente ; la redirection 443 avait survécu au redémarrage (vérifié depuis le Mac) |
| Copie + V8 (`dcf18661…`) | hashes identiques sur le Pi ; **code 0** à 20:21:47 UTC ; `installed_hash=ffcc6349…`, `rollback_hash=a7ab7b07…`, `web_previous=/root/chillcenter-rollback/web-20261015T202141Z`, sauvegarde `/var/lib/chillcenter/data/backups/4a43cede-1c75-4125-b406-6385b87c8e40` ; fichiers entrants supprimés |
| Activation (`pi-enable-http-operator.sh` `26a8df58…`, auto-test PASS sur le Pi) | **code 0** à 20:22:11 UTC ; configuration `d6167cb1…`, sauvegarde `server.json.ea24f2f0a98d`, service actif |
| Redirection 80 (`pi-redirect-80-operator.sh` `9577ad88…`, auto-test PASS, `wlan0` n'a que `192.168.1.42/24`) | **code 0** à 20:22:34 UTC ; `unit_state=enabled/active` |
| Vérifications depuis le Mac | `http://192.168.1.42/` 200 (19 604 o), API 200, `:8080` 200, `Origin` propre 200 / étrangère 403, `/admin/tmdb` et `/admin/exports/x` **403 en HTTP** et TMDB **200 en HTTPS**, `.wasm` 200 `application/wasm` (652 895 o), `https://192.168.1.42` 200, `:8443` 200 |
| Navigateur réel (intégré à l'application), `http://192.168.1.42/` | page chargée, `isSecureContext` faux, pas de Service Worker, `crypto.randomUUID` absent (repli utilisé), profil réel lu par l'API ; **aucune lecture lancée** dans l'interface (elle écrirait la progression du vrai profil) |
| Sonde de lecture réelle, port 80 puis 8443 (bail volatil libéré sans progression) | **21 fichiers réels ouverts, 0 échec** sur les deux ports (MKV 12, MP4 4, AVI 5 ; débit identique) |

**Non vérifié** : lecture complète d'un film dans un navigateur sur le Pi (PWA, téléphones, autres navigateurs) ; comportement après un redémarrage **provoqué** par nous (celui-ci était subi : la redirection 443 a survécu, la redirection 80 et `http_listen` n'ont pas été éprouvées au redémarrage) ; la cause de la coupure de 4 heures (aucune action de notre part sur le Pi à ce moment).

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
| **Simulation du déploiement** (`scripts/pi-operator-simulation.sh`, VM Ubuntu 24.04 aarch64, GNU tar 1.35, coreutils 9.4, curl 8.5.0, root, vrai ancien binaire `1eb24e97…` comme service via un `systemctl` factice, script envoyé par stdin comme en production) | **34 PASS, 0 FAIL** (état de V6, dépassé : V7 passe **45 PASS**, voir « Reprise après la revue n° 2 ») : A déploiement nominal (binaire, web, rollback, sauvegarde, invariants, aucun résidu) ; B mauvaise URL refusée **avant** l'arrêt ; C échec de vérification ⇒ ancien binaire restauré, `web/` retiré, service actif ; D idem avec un ancien `web/` qui revient ; E redéploiement avec copie de rollback réutilisée et ancien web conservé ; F copie de rollback douteuse refusée ; G connexion cliente établie ⇒ refus (42) ; H archive avec lien symbolique refusée |

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

## 4. Revue indépendante (obligatoire avant exécution) — la revue n° 3 a porté sur les octets V7 ci-dessus (voir « Revue indépendante n° 3 »)

Un reviewer distinct de l'auteur (et sans droit d'édition sur le contenu revu) approuve **les octets exacts** de : `pi-update-web-operator.sh` (hash §2), le diff Rust, l'archive web finale et le hash du binaire. Points à contrôler : arrêt refusé si connexion établie ; sauvegarde hors ligne sous l'UID 997 ; copie de rollback sans écrasement ; remplacement atomique du binaire ; refus des liens symboliques/membres spéciaux dans l'archive ; `SHA256SUMS` couvrant tout l'arbre ; vérifications en TLS strict (jamais `-k`) ; **restauration automatique** (ancien binaire + ancien dossier web) à tout échec avant `completed=1` ; invariants config/unité/SPKI. Auto-test local : `bash scripts/pi-update-web-operator.sh --self-test` → `parser_self_test=PASS`, `bash -n` OK ; `shellcheck` non disponible. Preuve d'exécution réelle sous Linux : simulation 45/45 (§3).

Enseignement conservé de l'échec V1–V3 (BUG-0051) : le script est envoyé par **stdin** (`bash -s`), sans citation imbriquée ni `eval`/`sed` sur des données distantes.

## 5. Exécution (seulement après revue + ton feu vert explicite)

> **Le [runbook](WEB-CLIENT-PI-RUNBOOK.md) fait foi** pour l'exécution (conventions SSH, préflight en lecture seule par stdin, hashes V8 exacts, tableau des codes de sortie, retour arrière). Les commandes ci-dessous sont l'esquisse initiale, conservée pour traçabilité ; en cas de différence, c'est le runbook qui s'applique.

Prérequis : une **connexion maître SSH stricte/batch** ouverte par toi (les anciens sockets `/private/tmp/chillcenter-ssh.*` ne sont plus utilisables comme preuve d'une session valide ; je ne contourne aucun échec d'agent).

```bash
CP=<chemin du ControlPath que tu as ouvert>;  HOST=<hôte Pi>
# a) Préflight en LECTURE SEULE (aucune écriture) : sudoers autorise `env`, service actif, hash binaire installé,
#    espace disque, curl et ss présents, chemin du certificat public
ssh -S "$CP" "$HOST" 'sudo -n env true && echo sudo_env_ok; sudo -n sha256sum /usr/lib/chillcenter/chill-server; systemctl is-active chillcenter; df -h /usr /var/tmp | tail -2; command -v curl ss openssl runuser seq; ls -l /var/lib/chillcenter/data/tls/certificate.pem'
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
install -o root -g root -m 0755 /root/chillcenter-rollback/chill-server-cef54cd0e8ba /usr/lib/chillcenter/chill-server
sha256sum /usr/lib/chillcenter/chill-server        # doit afficher cef54cd0…b3c3
rm -rf --one-file-system /usr/lib/chillcenter/web   # ou : mv -T /root/chillcenter-rollback/web-<horodatage> /usr/lib/chillcenter/web
systemctl start chillcenter
```

Aucune migration SQLite n'est introduite par ce lot : l'ancien binaire reste compatible avec les données (à confirmer par la revue du diff). La sauvegarde privée fraîche n'est nécessaire qu'en cas d'altération des données, ce qui n'est pas prévu. **Déclencheurs** : hash inattendu, sauvegarde en échec, service non démarré ou API critique en échec, toute vérification du §6 en échec, régression de confinement.

## 8. Ce qui n'est pas prouvé

Exécution du script opérateur sur le vrai Pi (`NOT_RUN` ; seule la simulation Linux/VM a eu lieu), durée réelle d'interruption, revue indépendante n° 3 des octets V7 (faite : APPROVED_WITH_CONDITIONS ; elle ne remplace pas ton feu vert), lecture réelle depuis un navigateur sur le Pi, performances sur appareils faibles, Service Worker sur certificat auto-signé. Compilation, tests Rust, build ARM64 et essai de bout en bout Linux : faits (§3).

Rejouer l'essai de bout en bout : `colima start` puis `colima ssh -- <repo>/scripts/web-e2e-linux.sh <repo>` (le binaire et `artifacts/web-pi/chillcenter-web.tar` doivent exister), puis `colima stop`.

## Tentative d'exécution du 15 octobre 2026 : arrêtée au préflight, rien déployé

À 13:49 UTC, après ton « exécute toi » et l'ouverture de la connexion maître (socket privé `0700`/`0600`, master actif), le préflight en **lecture seule** (étape 3 du runbook) a montré que **le binaire installé sur le Pi n'est plus `1eb24e97…`** : il vaut **`cef54cd0e8baf45f70ee0f57584001521c7dcd705d4f8ab351eb8074f524b3c3`**, identique au binaire de l'archive locale `chillcenter-server-c610ea9-pi64.tar.gz` (livraison de la branche `codex/player-streaming`, service redémarré à 13:43 IST, soit environ 6 minutes avant le préflight). Le répertoire de rollback contient aussi des copies des livraisons `07358f7`, `9277ac4` et `cfc87de`. Une autre session a donc mis à jour le serveur pendant ce travail (cohérent avec l'avertissement « opération en cours sur le serveur »).

Conséquences, constatées et non contournées :

- Conformément au runbook (« hash inattendu = stop »), **aucun fichier n'a été copié, aucun script exécuté** ; le seul accès au Pi a été `sudo -n env true` et le préflight, tous deux en lecture seule. Le script V7 aurait de toute façon refusé (sortie 2 avant tout arrêt : binaire installé ≠ `1eb24e97…`).
- **Déployer mon binaire aurait été une régression** : ma branche part de `0d0da0e`, et la branche Mac a depuis reçu 13 commits dont **2 396 lignes de modifications dans `crates/`** (dont `chill-server` : `api.rs`, `catalog_api.rs`, `store.rs`, `database.rs`, scanner) que mon binaire n'a pas. Le remplacer par le mien retirerait ces correctifs de la production et, comme `store.rs`/`database.rs` ont changé, un retour à un binaire plus ancien n'est sûr que si le schéma SQLite reste compatible (non établi).
- Avant tout déploiement : **intégrer** les commits de `codex/player-streaming` dans ma branche (conflits probables dans `api.rs`), recompiler et retester, reconstruire le binaire ARM64, repaqueter l'archive, mettre à jour les hashes, et faire porter une **nouvelle revue indépendante** sur le script (son hash change : l'ancien hash attendu `1eb24e97…` est écrit en dur dans le script V7). État du Pi à ce moment : service actif (`NRestarts=0`), 5 sauvegardes privées présentes, `web/` absent, configuration `ea24f2f0…`, unité `76f7d349…`, empreinte TLS inchangée `3018fb4d…`.

## Revue indépendante n° 3 (15 octobre 2026)

Verdict **APPROVED_WITH_CONDITIONS** sur le script **V7** (`b61b9fd6…`, 17 609 octets, 218 lignes lues) : aucun défaut critique, majeur ni moyen ; Rust et artefacts inchangés APPROVED ; bancs d'essai APPROVED ; rapport verbatim : [web-pi-deploy-v7-review-3.md](../reviews/web-pi-deploy-v7-review-3.md). Le reviewer (nouvel agent) a rejoué la simulation (45/45), l'essai de bout en bout (20/20), la preuve par mutation, `cargo test` (102), et 19 scénarios adverses de son cru (récupération interrompue par des signaux de groupe, `systemctl start` ou `stop` qui échouent, `ss` trompeur, proxy et `~/.curlrc` hostiles, ancien `web/` non vide, outil absent…). **Toute modification du script annule cette revue.**

Conditions du reviewer et décisions :

| Condition / constat | Décision |
| --- | --- |
| Ligne périmée « 34 PASS » du plan | corrigée dans ce document |
| `openssl` et `runuser` à vérifier avant exécution | ajoutés au préflight manuel §5a ; le script les exige de toute façon avant tout arrêt (sortie 2) |
| **LOW-1** le contrôle d'URL ne compare que le **port**, pas l'hôte : une URL mal saisie vers un autre hôte du LAN présentant le même certificat passerait la sonde | **risque accepté** (un seul hôte cible, le Pi ; la sonde TLS stricte vérifie déjà le certificat). Ne pas se fier à l'URL seule : relire `CHILLCENTER_BASE_URL` avant l'exécution (`https://192.168.1.42:8443`) |
| **LOW-2** code de sortie 1 après un déploiement **réussi** si stdout est fermé (le `printf` final échoue) | **risque accepté, à connaître** : un code 1 avec « RECOVERY » absent de la sortie et un service actif sur le nouveau hash signifie « installé, résumé non imprimé » ; vérifier l'état par la sonde du §6, ne pas relancer à l'aveugle |
| **LOW-3** binaire dans un état tiers (ni ancien ni nouveau) non signalé pendant la récupération | accepté (hérité de V6 ; improbable : `mv -f` atomique suivi d'un contrôle de hash) |
| INFO-1 `is-active` avant `stop` dans le gestionnaire (unité en `activating (auto-restart)`) ; INFO-2 `BASH_ENV` honoré (seul root via `sudo env` peut le fournir) ; INFO-3 quelques outils coreutils non listés | acceptés |

Non vérifié par aucune revue : exécution sur **Debian 13 réel avec systemd réel**, sudoers/`use_pty`, mémoire de 905 Mio, durée réelle d'interruption (≈ 3 à 30 s dans la VM). Ces points ne se lèvent que par l'exécution réelle, qui exige ton feu vert explicite et la connexion SSH maître.

## Revue indépendante n° 2 (15 octobre 2026)

Verdict **CHANGES_REQUESTED** sur le script V6 (N1 SIGPIPE, reproduit par le reviewer), Rust APPROVED, artefacts APPROVED : [rapport verbatim](../reviews/web-pi-deploy-v6-review-2.md). Corrigé dans V7 (voir « Reprise après la revue n° 2 »). Le script V7 `b61b9fd6…` n'a **pas encore de revue** : il ne doit pas être exécuté.

## Revue indépendante n° 1 (15 octobre 2026)

Verdict **CHANGES_REQUESTED** (script V5, trois défauts MEDIUM S1–S3), Rust APPROVED_WITH_CONDITIONS, artefacts APPROVED : [rapport verbatim](../reviews/web-pi-deploy-v5-review-1.md). Le script `8d47e9df…` ne doit donc **pas** être exécuté ; sa correction produira un nouveau hash et une nouvelle revue.
