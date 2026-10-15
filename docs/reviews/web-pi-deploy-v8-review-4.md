# Revue indépendante n° 4 — script opérateur V8 (15 octobre 2026)

Reviewer : agent distinct de l'auteur, lecture seule (aucune édition du dépôt). Commit revu : `623a768`. Rapport reproduit tel que rendu ; seul l'en-tête ci-dessus et la section finale « Traitement » sont ajoutés par l'auteur.

---

# Revue indépendante n°4 : script opérateur V8, préflight, Rust web, artefacts, runbook

Je n'ai modifié aucun fichier du dépôt (`git status --short` vide à la fin). Mes fichiers temporaires étaient sous /tmp/review4-* et sont supprimés. La VM Colima est arrêtée (`colima stop`) et /opt/review4 est effacé dans la VM. Je n'ai touché ni le Pi ni aucun hôte réseau.

## 1. Verdict

**APPROVED_WITH_CONDITIONS** pour le script `scripts/pi-update-web-operator.sh`, 17916 octets, SHA-256 `dcf1866111ef756b53113adce323f90960f5edbf00835204da78097fd6528e29` (hash et taille vérifiés identiques). Cette approbation ne vaut que pour ces octets : toute modification, même d'un commentaire, l'annule.

| Objet | Verdict |
|---|---|
| Script V8 (octets ci-dessus) | APPROVED_WITH_CONDITIONS |
| Préflight `pi-preflight-readonly.sh` (hash `2ee4a5a9…0d50d44` vérifié) | APPROVED |
| Rust web : `web_static.rs`, garde d'origine dans `api.rs`, `lib.rs` | APPROVED (aucune régression de sécurité trouvée) |
| Binaire (14765736 o, `b80408da…c89f`) et tar (1125376 o, `894fbe95…7118`) | APPROVED sur le contenu. La provenance du binaire n'est pas prouvée (voir §5). |
| Banc de simulation | APPROVED avec réserves (MEDIUM-2 et LOW-3) |
| Runbook et plan | CORRECTIONS REQUISES (MEDIUM-1, LOW-1, LOW-2), sans changer le script |

Conditions, toutes sans modification du script :
1. Corriger les références périmées du runbook : MEDIUM-1 (`24120568` / `2412…` aux lignes 124 et 140). Sinon l'opérateur lira « mismatch » après un déploiement réussi.
2. Corriger ou barrer le retour arrière manuel du plan §7 (LOW-1), qui nomme encore `chill-server-1eb24e97bcdf` (fichier inexistant sur le Pi). Le runbook étape 7 est correct et prime.
3. Accepter les risques résiduels listés au §5.
4. Garder en plus le feu vert explicite de l'utilisateur et sa connexion SSH maître. Ce verdict ne l'autorise pas à lui seul.

## 2. Diff V7 → V8 (commandes exécutées)

- `git show f2a765f:scripts/pi-update-web-operator.sh` : SHA-256 `b61b9fd6274f19669dcddabcf180e443b865f8f3777529b63d57ebad07e64872`, identique à V7.
- `diff V7 V8` : 8 hunks, tous liés à l'ancien hash.
  - L2-3 et L7 : commentaires seulement (V8, `OLD_BINARY_SHA256` ajouté dans l'exemple de commande).
  - L19 supprimée : `old_expected='1eb24e97…'`.
  - L22 : le nom `rollback=…/chill-server-1eb24e97bcdf` est retiré de la ligne `rollback_dir`.
  - L65-66 : `${OLD_BINARY_SHA256:?…}` ajouté, et le test `[[ "$OLD_BINARY_SHA256" =~ ^[0-9a-f]{64}$ … && "$NEW_BINARY_SHA256" != "$OLD_BINARY_SHA256" ]]`.
  - Nouvelles L69-71 :
    - `old_expected="$OLD_BINARY_SHA256"`
    - `rollback="$rollback_dir/chill-server-${old_expected:0:12}"`
    - `rollback_tmp_prefix="$rollback_dir/.chill-server-${old_expected:0:12}."`
  - L137 → L140 : `cleanup_owned_file "$rollback_tmp" "$rollback_tmp_prefix"`.
  - L183 → L186 : `mktemp "${rollback_tmp_prefix}XXXXXX"`.
- Aucune autre ligne ne diffère. Le diff ne contient rien d'autre que le changement annoncé.
- `grep` de `1eb24e97|cef54cd0` dans V8 : aucune occurrence en dehors des commentaires (le hash cef n'apparaît nulle part dans le script).

## 3. Analyse du script V8 (lecture ligne à ligne, 221 lignes)

| Point | Résultat | Preuve |
|---|---|---|
| Validation du paramètre | OK | L65 `:?` (vide ou absent : sortie 1, comme les autres variables obligatoires). L66 : 64 hex minuscules, et différent de `NEW_BINARY_SHA256`, sinon sortie 2 « invalid expected hashes ». Les majuscules sont refusées. |
| Ordre d'affectation | OK | `old_expected`, `rollback` et `rollback_tmp_prefix` sont fixés aux L69-71. Première utilisation : L76 (`new_expected`), L78 (`old_expected`), L84-87 (`rollback`). Le handler `recover_on_failure` est défini L105 et le trap EXIT posé L145, donc après les affectations. Aucune sortie avant L145 ne passe par le handler. Le handler lit `$rollback` (L118), `$old_expected` (L118, L121) et `$rollback_tmp_prefix` (L140) : tous définis. Les chemins du trap ERR n'affichent que `$LINENO` et `$BASH_COMMAND`. |
| Sûreté sous `set -u` | OK | Aucune variable nouvelle n'est lue avant affectation. `${old_expected:0:12}` est sûr car la valeur est validée comme hex de 64 caractères. `set -u` reste actif dans le handler (seul `-e` est retiré) et tout y est défini. |
| Noms dérivés | OK | Le nom ne contient que `[0-9a-f]`, 12 caractères, sous `/root/chillcenter-rollback` : aucun `/`, aucun `..`, aucun préfixe `-`. Le répertoire est vérifié `700:0:0` et non symlink (L84). L'existant est accepté seulement s'il est un fichier régulier non symlink, de hash complet égal à l'ancien (L86-88). Une collision de 12 hex avec un autre binaire est refusée par la comparaison du hash complet. |
| Cleanup | OK | `cleanup_owned_file` compare avec `"$prefix"*` entre guillemets (littéral), exige un fichier régulier non symlink, et le préfixe est non vide par construction. |
| N1 à N8 (revue 2) et S1 à S8 (revue 1) | Toujours en place | Les lignes concernées sont inchangées à l'octet près (diff ci-dessus) : `trap '' PIPE` (L17), `trap '' HUP INT TERM` dans le handler (L109), `must_fail` (L45), `if … exit 1` après stop (L174), `set -E` + trap ERR (L15, 19), sonde post-récupération (L136), `curl -q --noproxy` et outils requis (L94, 75), refus fermé si `ss` échoue (L169), check de couverture de SHA256SUMS (L162-164), copie de rollback créée par `ln` sans écrasement (L189). |
| Conditions de la revue 3 | Honorées | (1) ligne « 34 PASS » : à vérifier dans le plan (mentionnée comme dépassée en L135, texte « V7 passe 45 PASS » encore présent, pas bloquant). (2) `openssl` et `runuser` vérifiés par le script en L75 avant tout arrêt. (3) LOW-1 de la revue 3 (contrôle d'URL = port seulement) : toujours vrai et documenté dans le runbook. |
| LOW-2 et LOW-3 de la revue 3 | Non corrigés (héritage V7) | Le `printf` final (L221) sans `|| true` peut donner un code 1 après un déploiement réussi si stdout est fermé. La restauration L116 ne traite pas un hash tiers. La revue 3 les avait acceptés, le runbook les mentionne. Pas de régression V8. |

Le `--self-test` n'a pas régressé (voir §4).

## 4. Commandes exécutées et résultats

- `wc -c` / `shasum -a 256` : script 17916 o `dcf18661…8e29` ; préflight `2ee4a5a9…0d50d44` ; binaire 14765736 o `b80408da…c89f` ; tar 1125376 o `894fbe95…7118`. Tous conformes.
- `bash -n scripts/pi-update-web-operator.sh` : `syntax_ok`.
- `bash scripts/selftest-mutation.sh scripts/pi-update-web-operator.sh` :
  - `mutant m1 : rc=1 (détecté ✓)`
  - `mutant m2 : rc=1 (détecté ✓)`
  - `parser_self_test=PASS` puis `original : rc=0`
  - Le lancement direct de `--self-test` a été refusé par le sandbox de la session (hors de mon contrôle) ; l'auto-test de l'original a donc été exécuté via ce script (même sortie `parser_self_test=PASS`, rc=0).
- Simulation en VM, `colima ssh -- sudo -n …/pi-operator-simulation.sh <worktree> …/chillcenter-server-c610ea9-pi64.tar.gz` :
  - Bandeau : ancien binaire `cef54cd0…b3c3`, nouveau `b80408da…c89f`, archive `894fbe95…7118`.
  - Script testé : `45d48cf2…4424` (copie avec uid 4997, car 997 est pris sur la VM ; « seule différence : 2 lignes »), original `dcf18661…8e29`.
  - Résultat : **52 PASS, 0 FAIL** (A–J, N, P), conforme à l'attendu.
- Mutants de la simulation (copiés dans la VM, exécutés l'un après l'autre) :
  - **m1** : `${old_expected:0:12}` → `:0:11` dans le nom du rollback. Simulation : **49 PASS, 3 FAIL**. Détectés : A (« copie de rollback = ancien binaire »), F (refus attendu 2, obtenu 0) et F (service non intact). Une dérivation cassée du nom est donc bien détectée.
  - **m2** : préfixe de nettoyage du fichier temporaire de rollback remplacé par `/nonexistent/`. Simulation : **52 PASS, 0 FAIL**. **Non détecté.**
- `cargo test --locked --offline -p chill-server` (avec `CARGO_TARGET_DIR=/tmp/review4-target`, supprimé ensuite) : 104 passés, 0 échec ; tests d'intégration `legacy_cli` : 2 passés. Les autres cibles du lot (`src/main.rs`, doc-tests) : 0 test, rien à signaler.
- `git diff --stat 7204e19 623a768 -- crates Cargo.toml Cargo.lock rust-toolchain.toml` : vide. Le Rust est donc inchangé depuis le merge sur lequel le binaire a été construit.

## 5. Constats

**MEDIUM-1 — Runbook : hash de l'ancien « nouveau binaire » encore présent (étapes 5 et 6).**
- Le tableau des codes de sortie (ligne 124) dit : « si `installed_binary_sha256=2412…` et service actif, c'est déployé ». La ligne 140 attend « hash binaire `24120568…` ».
- Or le nouveau hash est `b80408da…c89f` (le `24120568` était celui de l'ancien binaire de la revue 3).
- Conséquence : après un déploiement réussi, l'opérateur verra un hash qui ne correspond pas à la consigne. Dans le cas « LOW-2, déployé mais résumé non imprimé », il conclura à tort à un échec et pourrait faire un retour arrière inutile. Le script lui-même n'est pas concerné.
- Preuve : `grep -n "2412\|24120568" docs/deployment/WEB-CLIENT-PI-RUNBOOK.md` donne les lignes 124 et 140.
- Correctif : remplacer par `b80408da…c89f`.

**MEDIUM-2 — Le banc de simulation ne prouve pas toute la dérivation V8.**
- Le nettoyage du temporaire de rollback (`rollback_tmp_prefix`, L140) n'est pas testé : le mutant m2 passe 52/52.
- Aucun scénario n'échoue entre `mktemp` (L186) et `rm -f` (L190). Le contrôle de résidus du scénario A ne regarde pas `/root/chillcenter-rollback/.chill-server-*`.
- Impact réel : faible. Ce chemin de nettoyage est protégé par `|| true` et sans effet sur la sécurité, mais un fichier de 14 Mio pourrait traîner dans le rollback dir en cas d'échec à cet endroit.
- Le banc ne peut pas non plus détecter une réintroduction d'un hash codé en dur : il utilise toujours le vrai ancien `cef54cd0…`, et le scénario P n'essaie qu'un mauvais hash `1eb24e97…`, qui est refusé avant l'arrêt. Je compense par `grep`, qui ne trouve aucun hash codé en dur dans V8.
- Correctif proposé : un scénario où le déploiement échoue juste après la création du temporaire (ex. `ln` impossible), plus un contrôle des résidus dans le répertoire de rollback. Optionnel pour l'exécution.

**LOW-1 — Plan §7 (`WEB-CLIENT-PI.md` lignes 199-200) : le retour arrière manuel cite encore `chill-server-1eb24e97bcdf` et « doit afficher 1eb24e97…c181 ».**
- Sur le Pi, la copie s'appelle `chill-server-cef54cd0e8ba`. La commande échouerait (« No such file »), ce qui est inoffensif mais trompeur en situation d'urgence.
- Le runbook (étape 7) est correct (`chill-server-cef54cd0e8ba`, `cef54cd0…b3c3`) et le plan lui-même dit (ligne 164) que le runbook fait foi.
- Aussi périmées mais sans danger : plan l. 135 « vrai ancien binaire `1eb24e97…` », l. 208 (§8, « revue n° 3 … faite »).

**LOW-2 — Runbook : mentions « V7 » résiduelles.**
- Lignes 16 (« Exécution du script V7 »), 20 (« Feu vert déploiement V7 »), 25 (« contrairement au script V7 »), 26 (« revues 1 à 3 »).
- Le runbook cite aussi « Étape 5 — script V8 » à la ligne ~92, d'où une incohérence interne. L'opérateur ne sera pas trompé sur les hashes, mais la phrase « feu vert » devrait nommer V8 et mentionner la revue n°4 (ce rapport).
- Le §0 du runbook affirme « 28 lignes » pour le préflight : exact (`wc -l` = 28).

**LOW-3 — Plan : description du tar inexacte.**
- Le plan dit « 56 entrées (54 fichiers) ». Le tar réel compte **60 membres : 56 fichiers (55 + `SHA256SUMS`) et 4 répertoires**.
- Preuve : `tar -tvf` donne 56 lignes `-` et 4 lignes `d`. Le build web reste `1556174bd809` (`build.json`).
- Sans conséquence sur la sécurité : `validate_archive_listing` accepte tous les types `-` et `d`.

**LOW-4 — Rust, `web_static.rs` : cache et mémoire (non bloquant).**
- Les fichiers non volatils sont servis `public, max-age=3600`. Seuls `index.html`, `sw.js` et `manifest.webmanifest` sont `no-cache`. Après une mise à jour du `web/`, un navigateur peut mélanger des modules JS de deux versions pendant ≤ 1 h. À vérifier côté `sw.js` ; pas un défaut de sécurité.
- Chaque requête lit tout le fichier en mémoire (jusqu'à 8 Mio) sans limite de concurrence : sur un Pi de 1 Gio, le décodeur WASM de 653 Ko est le plus gros fichier réel. Risque de déni de service local seulement.
- Les fichiers statiques sont servis sans authentification (c'est le but : l'API reste derrière sa propre garde).

**INFO — Préflight (`pi-preflight-readonly.sh`) : lecture seule, sans secret.**
- Lecture des 28 lignes : aucune redirection `>` ni écriture, `set -u` sans `-e` (une commande qui échoue n'interrompt pas la lecture).
- Il imprime uniquement : l'heure, `id -u`, os-release, versions d'outils, hashes (binaire, config, unité), état systemd, uid, compte de sockets, adresses d'écoute, **empreinte SPKI du certificat public** (dérivée du certificat, la clé privée n'est jamais lue), dates de validité, contenu du répertoire de rollback, espace libre, mémoire, nombre de sauvegardes. Aucune clé, aucun contenu de configuration.
- Il source `/etc/os-release` (fichier root, sans effet de bord).
- La ligne `installed_binary_sha256=` affiche l'attendu `cef54cd0…b3c3` dans le texte (cohérent avec le runbook).
- Réserve : il lit `ls -1 /root/chillcenter-rollback`, c'est-à-dire des noms de fichiers seulement.

## 6. Rust : diff du merge côté web (`git diff a59f5e6 623a768 -- crates/chill-server`, 417 lignes, 3 fichiers)

Fichiers : `api.rs` (+166/−11), `lib.rs` (+1), `web_static.rs` (+194, nouveau).

**Garde d'origine (`same_origin` → `check_browser_origin_and_host`)**
- Sans `Origin` ni `Sec-Fetch-Site` (clients natifs) : accepté.
- `Sec-Fetch-Site: same-origin` seul : accepté ; avec `Origin` : accepté seulement s'il égale `https://<Host>` (comparaison insensible à la casse).
- `Sec-Fetch-Site: none` : accepté uniquement pour un GET hors `/api/` (donc jamais pour l'API, ni pour HEAD ni POST).
- Tout le reste est refusé en 403 : `cross-site`, `same-site`, `Origin` seul sans `Sec-Fetch-Site`, `Origin` `null`, valeurs non UTF-8 (le motif `Some(None)` tombe dans `_ => false`).
- La vérification du Host (nom TLS et port) vient après et reste inchangée. Le garde-fou DNS-rebinding est donc conservé.
- L'ordre est correct : la garde passe avant le routage (couche `from_fn_with_state`), donc elle s'applique aussi au fallback statique.
- Pas de contournement trouvé : `Sec-Fetch-Site` est un en-tête interdit côté JavaScript (le navigateur le pose), et un client non navigateur peut le forger mais n'a de toute façon pas besoin de la garde.

**Fallback statique (`web_static::serve`)**
- `/api` et `/api/…` : 404 nu (devenu `ROUTE_NOT_FOUND` en JSON par `normalize_response`) pour toute méthode. Les routes API ne sont donc pas servies comme statiques.
- `relative_name` refuse : chemin vide, `..`, parties commençant par `.` (donc `.env`, `.git`), `//`, `\`, `\0`, `%` (pas de décodage, donc pas de `%2e%2e`), longueur > 200. Seuls GET et HEAD sont acceptés (405 sinon).
- `read_file` : `canonicalize` de la racine et du fichier, et refus si le chemin canonique diffère (un symlink n'importe où dans le chemin est refusé), fichier régulier, ≤ 8 Mio. Faille TOCTOU irréaliste : `web/` est root, en lecture seule pour le service.
- Table MIME fermée : une extension inconnue ou absente donne 404. Pas de listage de répertoire.
- Aucun panic repéré : pas de `unwrap` en dehors des tests, `Response::builder` mappe l'erreur.
- En-têtes : `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, CSP sur HTML (`default-src 'self'`, `script-src 'self' 'wasm-unsafe-eval'`, `object-src 'none'`, `frame-ancestors 'none'`).
- Tests : les 104 tests passent, dont ceux du garde et du statique (`same_origin_browser…`, `static_files_obey…`, `only_plain_relative_names…`, `symbolic_links_and_missing_roots…`).
- Comparaison avec la version revue en revue 3 : `web_static.rs` ne diffère de `3a0d53a` que par une ligne de commentaire.

## 7. Artefacts

- Hashes et tailles : conformes (voir §4). Binaire : `file` = ELF 64-bit ARM aarch64, dynamique (`/lib/ld-linux-aarch64.so.1`), strippé. `strings` y trouve la CSP du fallback statique (`wasm-unsafe-eval`) et des chemins du registre cargo local (`/Users/ghillaumesoubigou/.cargo/registry/…`), cosmétiques, sans secret.
- Tar : 60 membres, 56 fichiers réguliers (`-`) et 4 répertoires (`d`), 0 autre type. `tar -tf | grep -v '^web/'` : vide ; aucun `..`, aucun chemin absolu, aucun `._*` ni `.DS_Store`. Extraction dans un dossier neuf : `sha256sum -c --quiet --strict SHA256SUMS` OK ; 0 fichier spécial.
- Contenu comparé aux sources : `node scripts/build-web.mjs /tmp/review4-tar/rebuilt/web` → « 54 fichiers, build 1556174bd809 ». `diff -r` contre le tar extrait : seul `SHA256SUMS` diffère (ajouté par le script de packaging). Le contenu du tar est donc bien celui des sources actuelles.
- Décodeur : `mpeg4-decoder.wasm` `453fb7b171fde50d1306a57793ebac571459a7c324d28bd8b4f6c3db6b7bda53` (652895 o) et `mpeg4-decoder.js` `64d97f891594c386652cf3f783dfbdaf65d358ddb0bf1be96f1fbe3b2d3be827` : identiques à ceux du plan.

## 8. Runbook : cohérence

- Étape 2 : les 4 hashes (`dcf18661…`, `b80408da…`, `894fbe95…`, `2ee4a5a9…`) et la taille 17916 sont corrects. Étape 4 : copie vers `/var/tmp`, hashes attendus abrégés corrects.
- Étape 5 : `sudo -n env OLD_BINARY_SHA256=cef54cd0e8baf45f70ee0f57584001521c7dcd705d4f8ab351eb8074f524b3c3 NEW_BINARY_SHA256=b80408da… WEB_ARCHIVE_SHA256=894fbe95… CHILLCENTER_TLS_CERT_PATH=… CHILLCENTER_BASE_URL=https://192.168.1.42:8443 /bin/bash -s --` : les trois hashes sont corrects et complets, aucun `$` ni guillemet imbriqué ; le script est envoyé par stdin. La sortie listée (7 lignes) correspond au `printf` final.
- Étape 7 : retour arrière correct, avec le nom dérivé `chill-server-cef54cd0e8ba`, mais voir MEDIUM-1 pour l'étape 6.
- Le tableau préflight attend `cef54cd0…b3c3` : cohérent avec le préflight et la simulation.
- Résiduels : MEDIUM-1, LOW-2 ci-dessus. Plus `1eb24e97` uniquement dans le plan (LOW-1) et dans `pi-operator-simulation.sh` L187, où il sert volontairement de « mauvais hash » du scénario P.

## 9. Ce que je n'ai pas pu vérifier

- Exécution sur le Pi réel : Debian 13, systemd réel (`Restart=`, `TimeoutStopSec`), sudoers avec `env` et `use_pty`, RAM de 905 Mio, durée d'interruption, format réel de `ss -Hltnp` sur iproute2 de Debian : SUPPOSÉ (la simulation tourne sur Ubuntu 24.04 aarch64 avec un `systemctl` factice, GNU tar 1.35).
- Le binaire déployé lui-même n'a pas été reconstruit : je n'ai pas refait le build zigbuild ARM64. La provenance (`b80408da…`) repose sur l'affirmation du plan et sur le fait que le Rust n'a pas changé depuis `7204e19` ; le binaire embarque bien la CSP du fallback statique. L'essai bout en bout `web-e2e-linux.sh` (20 PASS annoncés dans le plan) n'a pas été rejoué.
- Le Pi a été supposé tel que décrit par le préflight du 15 octobre dans le plan (binaire `cef54cd0…`, build `c610ea9`, `/root/chillcenter-rollback/chill-server-r7-9a17ffe55e3d`) : je n'ai aucun moyen de le confirmer sans accès au Pi.
- Pas de test dans un vrai navigateur (Service Worker sur certificat auto-signé, CSP, mise en cache) : non testé.
- Le lancement direct de `bash scripts/pi-update-web-operator.sh --self-test` a été bloqué par le sandbox de la session ; je me suis appuyé sur la sortie de l'original dans `selftest-mutation.sh` (`parser_self_test=PASS`, rc=0).
- `cargo clippy`, `node --test web/test/*.test.js`, `vm-e2e.sh` : non rejoués.
- La revue du préflight se fonde sur la lecture du fichier ; je ne l'ai pas exécuté (il lit des fichiers du Pi).

---

## Traitement par l'auteur (après le rapport)

| Constat | Traitement |
| --- | --- |
| MEDIUM-1 (hashes `2412…`/`24120568…` du runbook) | corrigé : `b804…` / `b80408da…` |
| LOW-1 (retour arrière du plan §7) | corrigé : `chill-server-cef54cd0e8ba`, `cef54cd0…b3c3` |
| LOW-2 (mentions V7 du runbook) | corrigé aux étapes 5 et « Feu vert déploiement V8 » ; les mentions de l'historique (revues 1 à 3) restent volontairement |
| LOW-3 (décompte du tar) | corrigé : 60 membres, 56 fichiers dont `SHA256SUMS`, 4 répertoires |
| MEDIUM-2 (nettoyage du temporaire de rollback non couvert) | **accepté, non corrigé** : impact faible ; un scénario dédié reste à écrire si le script est rouvert |
| LOW-4 (cache ≤ 1 h, lecture intégrale en mémoire) | **accepté, non corrigé**, à surveiller lors du test réel |
| LOW-2/LOW-3 de la revue 3 | hérités, acceptés comme dans la revue 3 |

Le script V8 (`dcf18661…`, 17 916 octets) n'a pas été modifié après la revue. L'essai direct de `--self-test` bloqué pour le reviewer a, lui, été exécuté par l'auteur (`parser_self_test=PASS`).
