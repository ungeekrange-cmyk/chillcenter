# Revue indépendante n° 7 — contrôle des correctifs de la revue n° 6 (15 octobre 2026)

Reviewer : agent distinct de l'auteur, lecture seule (aucune édition du dépôt, aucune connexion au Pi, injoignable à ce moment). Commit revu : `2f26896`. Rapport reproduit tel que rendu ; seule la section « Traitement par l'auteur » finale est ajoutée.

---

REVUE N° 7 (contrôle de la revue 6) — commit 2f26896, worktree propre avant/après (git status vide).

VERDICT
- scripts/pi-enable-http-operator.sh (SHA-256 fb88ceffb9558a7b4988f0a929e3696760952b6102f5fb3e83decabb616c748b, 10488 o) : REFUSED. Un défaut HIGH reproduit dans la VM laisse une configuration cassée ET un service arrêté (voir constat 1).
- scripts/pi-redirect-80-operator.sh (07d49c26e46e307f0f06841c452a0a11c8609e540b110fb5c288e038da30f3a0) : APPROVED. Le diff contre 47d42ec n'a qu'une ligne de commentaire (« derived from pi-redirect-443-operator.sh » au lieu de « -80- »).
- Binaire target/aarch64-unknown-linux-gnu/release/chill-server (ffcc634923f637331cac25fec1e5283b062ef5e2943778bcab59460b6269b0f7, 14890920 o) : APPROVED pour le delta Rust (constat 3 et 4). Je ne l'ai pas recompilé.
- Archive artifacts/web-pi/chillcenter-web.tar (24839b3d81f21632818903c5c097471a98086748303ac304477f23e480d00df2) : APPROVED.
- pi-update-web-operator.sh (dcf1866111ef756b53113adce323f90960f5edbf00835204da78097fd6528e29) et pi-redirect-443-operator.sh (3e7f6d4f8b0bdfa33af09f7031f7862ddca992426507dbf0cd498a63fe1f4ef4) : hashes inchangés, vérifiés par shasum.
- Le déploiement ne doit pas se faire tant que l'étape 10 (enable) n'est pas corrigée et revue de nouveau.

FINDINGS

1. HIGH (bloque l'enable) : revert_now saute la restauration quand stderr est cassé.
Cause : à la ligne 100, `trap - ERR HUP INT TERM; echo 'FAILURE…' >&2` s'exécute sous `set -e`. Si l'écriture sur stderr échoue (ssh coupé : EPIPE, `trap '' PIPE` est actif ; pty fermé : EIO), `echo` retourne 1 et le shell sort avant le `mv -f .restore`. Le trap EXIT `cleanup` supprime ensuite `.restore` et `.final`. C'est exactement le cas « session SSH tombée » que le trap HUP devait couvrir.
Preuve 1, micro-test bash 5.2.21 dans la VM : avec un trap `rv` identique et stderr sur un pipe sans lecteur, puis avec `2>/dev/full`, le fichier témoin après l'echo n'est jamais créé (`ls: cannot access '/tmp/r7-restored'`, exit=1).
Preuve 2, sur les octets réels fb88ceff dans la simulation (systemd réel, mutant qui écrit une IP non assignable, `exec 2>/dev/full` inséré juste après le `trap revert_now`) :
`R7 RC=1 cfg_sha=8e0132e2…593a4 orig=303106828d58… service=failed https=000 tmp_left=0`
Le témoin sans stderr cassé donne `cfg_sha=orig service=active https=200`.
Résultat : configuration NOUVELLE conservée, service `failed`, HTTPS à 000, aucun temporaire restant.
Correction proposée : `set +e` en tête de revert_now, et chaque `echo … >&2` suivi de `|| true`. Mettre `mv -f` en premier est aussi possible. Ajouter un cas de simulation « stderr cassé + échec de démarrage ».
Le même motif existe peut-être ailleurs : j'ai relu seulement l'enable, pas les scripts inchangés.

2. LOW : trois mutants de l'enable ne sont pas détectés par pi-enable-http-simulation.sh (le script muté est copié dans un faux REPO dans la VM). Aucun n'est une régression, mais la simulation ne les protège pas.
- (a) `.restore` préparé après le `mv` : 46/46 PASS. Seule la fenêtre atomique change, sans test.
- (c) vérification du hash de la sauvegarde supprimée : 46/46 PASS. Défense en profondeur, redondante avec le contrôle du hash de la config à la ligne 58.
- (d) vérification du hash de `.restore` supprimée : 46/46 PASS. Même remarque.
- (b) HUP/INT/TERM retirés de la liste du trap : détecté (`FAIL E5c`, 45/1).
- (e) revert_now ne redémarre plus le service : détecté (`FAIL E4 HTTPS répond, HTTP coupé : attendu 200/000, obtenu 200/200`, 45/1).

3. Rust, FirstByteAcceptor et http1_only : correct, mesuré avec le vrai binaire dans la VM (ma propre séquence de test, hors dépôt).
- Lecture : `peek` plus `timeout` sont corrects. Au timeout, sur EOF ou sur erreur, le stream est abandonné, donc le permis revient (`LimitedStream` le porte). Le test unitaire `silent_connections_are_dropped_and_return_their_permit` couvre ce chemin.
- 16 connexions envoyant 1 octet puis se taisant : 16/16 fermées à 5,0 s, sonde 200.
- 16 connexions en trickle de 1 octet/s : 16/16 fermées à 6,0 s, sonde 200.
- 16 connexions avec en-têtes complets puis POST au corps jamais envoyé : 16/16 fermées à 30,0 s (middleware REQUEST_TIMEOUT), sonde 200. Un voisin LAN peut donc tenir les 16 permis ~30 s en boucle (DoS du seul listener HTTP, le HTTPS est séparé). C'est LOW/MEDIUM, déjà admis dans l'ADR, mais la valeur de 30 s n'y figure pas.
- main.rs : `*builder = Builder::new(..).http1_only()` sur un builder neuf, puis `.http1().timer().header_read_timeout().keep_alive(false)`. Aucune panique (l'assert de `version.is_none()` n'est pas atteint). Le binaire a démarré et servi 40 contrôles e2e.

4. Rust, https_only_path : correct. Le contrôle est exécuté dans `check_browser_origin_and_host`, donc AVANT tout contrôle d'Origin et avant le routage. Le layer s'applique au chemin complet.
Testé avec curl `--path-as-is` sur le listener HTTP du vrai binaire, avec Origin valide, en GET/HEAD/OPTIONS/PUT :
- 403 HTTPS_ONLY pour `/api/v1/admin/tmdb`, `/api/v1/admin/tmdb/` (slash final), `/api/V1/admin/tmdb`, `/api/2026-10-14/admin/imports`, `/api/2026-10-14/admin/imports/x/apply`, `/api/v1/admin/exports/x/status`, `/api/v1/admin/tmdb?x=1`. Les 403 sur HEAD sont visibles. Les sorties filtrées par mon grep n'ont pas été relues ligne à ligne, y compris pour GET/OPTIONS/PUT sur ces chemins.
- 404 ou 405, donc jamais le handler, pour `//api/…`, `/api//v1/…`, `/api/v1//admin//tmdb`, `%74mdb`, `tmdb%2fjobs`, `/TMDB`, `/x/../tmdb`, `/./exports`, `tmdb;x`.
- Les routes légitimes restent accessibles : `/admin/library` 200, `/admin/scan` 200.
- HTTPS inchangé : e2e « https : TMDB toujours disponible (200) ».
- Réserve LOW : restent atteignables en HTTP clair des routes d'écriture sous garde d'Origine, hors des trois préfixes : `/admin/profiles/{p}/media/{m}/tmdb*` (sélection, refresh, restauration d'affiche), `/admin/reassociations`, `PUT /admin/library`, `/admin/scan`. Elles ne portent pas la clé TMDB. L'ADR dit « administration de la bibliothèque », mais ne les liste pas.

5. Tests exécutés
- `cargo test --locked --offline -p chill-server --lib` : `test result: ok. 110 passed; 0 failed`. Le répertoire /tmp/review7-target est supprimé.
- `node --test web/test/*.test.js` : 51 pass, 0 fail.
- web-e2e-linux.sh : « résultat: 40 PASS, 0 FAIL », dont `idle_closed=16/16 probe_ok=True`.
- Simulations lancées en root dans la VM : enable 46 PASS / 0 FAIL, redirect-http 42/0, redirect-simulation 43/0.
- Artefacts : le tar contient 56 fichiers réguliers et 4 répertoires, tous sous `web/`, aucun lien. `diff -r -x SHA256SUMS` entre `node scripts/build-web.mjs` et le tar extrait : IDENTICAL.
- Le binaire contient la chaîne HTTPS_ONLY (`grep -a -c` = 1).

6. Analyse des traps, hors constat 1
- Avant la ligne 106, tout échec (y compris après la ligne 93) laisse la config intacte : ERR affiche puis sort, l'EXIT cleanup supprime les temporaires. La sauvegarde 93 reste et bloque un rejeu via la ligne 85 (« backup already exists ») ; c'est à nettoyer à la main.
- Fenêtre 106→117 : le `.restore` est préparé et son hash vérifié avant tout toucher à la config. Un signal pendant un `systemctl restart` ou un `curl` est différé par bash jusqu'à la fin de la commande, puis revert_now s'exécute. Je n'ai testé que TERM (E5c, simulation 46/46), pas HUP ni INT.
- Après la ligne 117, plus de récupération armée, mais la config est vérifiée par 5 contrôles. Seul le `printf` final peut échouer, sans état cassé.
- TERM reçu PENDANT revert_now : le handler est déjà retiré, l'action par défaut tue le shell. Si cela arrive avant le `mv`, la config NOUVELLE reste (EXIT supprime `.restore`) ; fenêtre de quelques ms, non mesurée.
- Les modes `--revert` et `--self-test` passent en simulation (E6 et self-test). Je n'ai pas relu `--revert` pour le motif du constat 1.

7. Docs
- Les hashes des étapes 9 à 12 du runbook sont cohérents avec les octets ci-dessus (binaire, tar, scripts, 14890920 o).
- Étape 10 : la sauvegarde `server.json.ea24f2f0a98d` correspond bien à `${previous_sha:0:12}`.
- LOW : l'introduction des étapes 9 à 12 exige « la revue indépendante n° 6 des octets exacts » alors que c'est maintenant la n° 7.
- LOW : WEB-CLIENT-PI.md garde des lignes périmées (« 108 tests », « 36 PASS », « 39 PASS » pour l'enable, alors que la ligne suivante donne 110/40/46).
- ADR-0018 : l'honnêteté sur la surface exposée (même API, secrets/archives refusés) et sur la disponibilité est bonne. Mais la décision 2 dit encore « HTTP/2 borné », or le listener est maintenant `http1_only`. La valeur 30 s du constat 3 n'y est pas non plus.
- PI-LIVE-STATE : l'injoignabilité du Pi depuis ~17 h n'est dite qu'en italique au milieu de la section 3. Un agent qui lit la section 1 « exécuté et observé » peut l'ignorer. Je recommande un bandeau en tête de fichier : « Pi injoignable depuis ~17 h le 15/10/2026, état NON vérifié, ne rien déployer avant un préflight lecture seule ». La section 1 contient pourtant déjà la consigne de refaire un préflight.

CE QUE JE N'AI PAS PU VÉRIFIER
- Le binaire n'a pas été recompilé : sa correspondance exacte avec la source de 2f26896 repose sur les hashes et sur son comportement (HTTPS_ONLY présent, e2e verts). Sa date de fichier (20:04:59) est postérieure à celle des sources (20:03:40).
- Aucun accès au Pi (comme prescrit) : les exécutions réelles de sudo avec ou sans pty, et le comportement réel d'une chute SSH (SIGHUP ou EPIPE), sont déduits, pas observés. Le constat 1 est prouvé sur le bash 5.2.21 d'Ubuntu 24.04 de la VM, pas sur le bash de la Pi OS.
- Les signaux HUP et INT dans la fenêtre, et un TERM pendant revert_now, ne sont pas testés.
- Je n'ai pas lancé de mutants Rust. Par raisonnement : retirer FirstByteAcceptor est détecté par le test unitaire `silent_connections_are_dropped…` (qui appelle directement le type) et par le contrôle e2e « 16 connexions muettes ». Retirer le contrôle `https_only_path` est détecté par le test unitaire `secrets_and_archives_are_https_only` et par 3 contrôles e2e (« secret TMDB réservé au HTTPS », « exports réservés », « TMDB toujours disponible »). Aucun test ne couvre `http1_only` seul. Le trickle et le corps bloqué ne sont pas non plus dans la suite livrée ; je les ai mesurés hors dépôt.
- La performance sous charge, la lecture média en HTTP clair et les navigateurs mobiles ne sont pas couverts.

Nettoyage fait : scratch du Mac et de la VM (/tmp/review7-*, mutants, scripts temporaires dans ~) supprimés ; `colima stop` exécuté ; aucun fichier du dépôt modifié.

---

## Traitement par l'auteur (après le rapport)

Les octets revus (`pi-enable-http-operator.sh` `fb88ceff…`) sont **remplacés** ; les autres objets approuvés (binaire `ffcc6349…`, archive `24839b3d…`) ne changent pas. **Une revue de contrôle n° 8 sur les octets finaux reste requise.**

| Constat | Traitement |
| --- | --- |
| 1 (HIGH, stderr cassé : restauration sautée) | **corrigé** : `set +e` en tête de `revert_now` ; scénario **E5d** ajouté à la simulation d'activation (stderr cassé + échec de démarrage : configuration d'origine restaurée, service actif) ; vérifié que E5d **détecte** le défaut sur un mutant sans `set +e` (2 FAIL). Le même motif existait dans `failed()` des **deux** scripts de redirection : corrigé dans `pi-redirect-80-operator.sh` et `pi-redirect-443-operator.sh` (nouveau hash `170f41a7…` ; l'exécution du 15 octobre avait utilisé les octets de la revue n° 5, sans échec) ; scénario R7b ajouté aux deux simulations |
| 2 (mutants a, c, d non détectés) | **accepté** : (c) et (d) sont des défenses en profondeur redondantes avec le hash de la configuration ; (a) ne change que la fenêtre atomique. (b) et (e) sont détectés |
| 3 (DoS à 30 s par corps jamais envoyé, trickle) | **accepté et documenté** dans l'ADR-0018 (valeur de 30 s) ; le HTTPS n'est pas touché |
| 4 (routes d'écriture non listées) | **documenté** dans l'ADR-0018 |
| 6 (TERM pendant `revert_now`, HUP/INT non testés) | **accepté**, fenêtre de quelques millisecondes ; HUP et INT partagent la liste de signaux de TERM |
| 7 (documents) | corrigés : introduction des étapes 9 à 12 (revue des octets exacts), chiffres périmés du plan (110 tests, 40/40, 49/49), ADR (HTTP/1 seulement, 30 s), **bandeau « Pi injoignable » en tête de PI-LIVE-STATE** |

Nouveaux octets : `pi-enable-http-operator.sh` `26a8df5832c83a2fbc659ddc8ab08ff41a55a59395e2e2fb43348ca82d938e57`, `pi-redirect-80-operator.sh` `9577ad88bdbe1c0b668db5d0344e19d91676a2de5a999f48bec410fb66f51dd7`, `pi-redirect-443-operator.sh` `170f41a787167c882973f45eef8d374fb530143b3aaa38cfb04041d1a585f821`. Résultats : simulation d'activation 49/49, redirection 80 45/45, redirection 443 46/46.
