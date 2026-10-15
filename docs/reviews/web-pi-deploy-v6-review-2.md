# Revue indépendante n° 2 — déploiement web sur le Pi (script V6, diff Rust, artefacts, bancs d'essai)

- Date : 15 octobre 2026. Demandée par l'utilisateur (« …puis lance une nouvelle revue indépendante »).
- Reviewer : sous-agent `a3b654d20c6584123` (type `Explore`, outils d'édition désactivés, **nouvel agent**, distinct de l'auteur et du reviewer de la revue n° 1). Il pouvait rejouer les essais dans la VM Linux jetable locale ; il l'a fait.
- Auteur du code revu : session principale. L'auteur n'a pas modifié le texte ci-dessous ; seul cet en-tête est de l'auteur.
- Octets couverts : script V6 16 118 o `ba9777cdf3491d5093f4288e113d4efcb03ae136a17dca6c47c7d4e8b1713e95` ; diff Rust `0d0da0e..3a0d53a4` ; tar `486e8eb2…0756` ; binaire `24120568…ac29`.
- **Verdict global rendu : CHANGES_REQUESTED**, à cause du seul script (N1, MEDIUM, reproduit en VM). Rust APPROVED, artefacts APPROVED, bancs d'essai APPROVED_WITH_CONDITIONS.
- Suite : ce verdict n'autorise **aucune** exécution sur le Pi. La correction change le hash du script et exige une nouvelle revue de ces octets.

---

## Rapport du reviewer (verbatim)

# Revue indépendante n°2 : script V6, diff Rust, artefacts, bancs d'essai

Reviewer indépendant, non auteur. Aucun fichier du dépôt n'a été modifié, aucun accès au Pi. Les fichiers temporaires sont sous /tmp et dans la VM jetable, qui est arrêtée (`colima stop`).

## 1. Verdicts

| Objet | Octets / hashes couverts | Verdict |
|---|---|---|
| Script V6 `scripts/pi-update-web-operator.sh` | 16118 o, SHA-256 `ba9777cd…1713e95` (vérifié identique) | **CHANGES_REQUESTED** (1 constat MEDIUM, N1, reproduit en VM) |
| Diff Rust `0d0da0e..3a0d53a4` (`api.rs`, `web_static.rs`, `lib.rs`) | `crates/` à `3a0d53a` ; `git diff 3a0d53a..HEAD -- crates Cargo.toml Cargo.lock rust-toolchain.toml` vide | **APPROVED** (réserves LOW/INFO ci-dessous) |
| Artefacts | tar 1099264 o `486e8eb2…0756` ; binaire 14601992 o `24120568…ac29` | **APPROVED** (provenance du binaire non prouvée par reconstruction) |
| Bancs d'essai | `pi-operator-simulation.sh` (160 l.), `web-e2e-linux.sh` (42 l.) | **APPROVED_WITH_CONDITIONS** (N3, N4) |
| **Global** | | **CHANGES_REQUESTED**, à cause du seul script (N1) |

N1 se corrige en une ligne. Le script corrigé aura un nouveau hash et exigera une nouvelle revue de ces octets. Le Rust et les artefacts ne sont pas concernés.

## 2. Tableau des vérifications

| Vérification | Résultat | Type |
|---|---|---|
| `wc -c` et `shasum -a 256` du script | 16118 o, hash conforme | VÉRIFIÉ |
| `bash -n` et `--self-test` | OK, `parser_self_test=PASS` | VÉRIFIÉ |
| Lecture ligne à ligne des 206 lignes du script | voir §3 à §5 | LU |
| `git status --porcelain` | vide ; HEAD `148aecc` | VÉRIFIÉ |
| `git diff 3a0d53a..HEAD -- crates Cargo.toml Cargo.lock rust-toolchain.toml` | vide (seuls docs et 2 scripts changent après) | VÉRIFIÉ |
| `cargo test --locked --offline -p chill-server` | 102 passés, 0 échec ; 2 tests d'intégration passés | VÉRIFIÉ |
| Lecture intégrale du diff `crates/` (api.rs, web_static.rs, lib.rs, tests) et de `normalize_response` (api_version.rs:20-46) | voir §6 | LU |
| clippy `-D warnings`, build ARM64, `node --test` (49) | non rejoués (hors liste autorisée) | LU, déclaré par le doc |
| Binaire : taille, hash, `file` | 14601992 o, `24120568…ac29`, ELF aarch64 dynamique strippé | VÉRIFIÉ |
| Provenance du binaire | mtime 12:50:03 postérieur aux sources (`api.rs` et `web_static.rs` à 12:46:02) ; aucun fichier `crates/`, `Cargo.*` ou toolchain plus récent ; commit `3a0d53a` à 12:50:12. `strings` : CSP avec `form-action 'self'` présente, ancienne CSP absente | VÉRIFIÉ (indices, pas une reconstruction) |
| Tar : taille, hash | 1099264 o, `486e8eb2…0756` | VÉRIFIÉ |
| Tar : contenu | 54 fichiers `-` et 4 dossiers `d`, 0 lien, 0 périphérique ; 0 entrée hors `web/` ; 0 `LIBARCHIVE`/`com.apple`/PaxHeader/`._` dans les octets | VÉRIFIÉ |
| `SHA256SUMS` du tar extrait | tous OK ; 53 lignes = 53 fichiers hors `SHA256SUMS` (diff vide, `find` sans `./SHA256SUMS`) | VÉRIFIÉ |
| Wasm | `453fb7b1…bda53` ; `BUILD-INFO.json` (ffmpeg 9.0.2, wasm 652895 o, js `64d97f89…`) et `NOTICE-LGPL.md` présents | VÉRIFIÉ |
| `vm-e2e.sh` (`web-e2e-linux.sh`) en VM Ubuntu 24.04 aarch64 | **20 PASS, 0 FAIL** | VÉRIFIÉ (rejoué par moi) |
| `pi-operator-simulation.sh` en VM | **34 PASS, 0 FAIL** (A à H) ; script testé = copie uid 4997 (`b533a83f…`) | VÉRIFIÉ (rejoué par moi) |
| Mes scénarios adverses X0 à X7 | voir §7 | VÉRIFIÉ |
| Mutation du `--self-test` : `validate_archive_listing` remplacée par `return 0` | `--self-test` affiche quand même `PASS` | VÉRIFIÉ (N3) |
| Debian 13 réel, systemd réel, sudoers, RAM 905 Mio, `sudo` 1.9.16 | non testable | SUPPOSÉ |

## 3. Constats de la revue n°1

| Constat | Statut | Preuve |
|---|---|---|
| **S1** `curl \| grep -q` sous `pipefail` | **CORRIGÉ** | Lignes 196-197 : `page="$(curl_tls …)"` puis `[[ == *title* ]]`. Aucun autre pipeline à lecteur précoce : `grep` sur herestring (155), `sort\|tail\|cut`, `awk`, `df\|awk` lisent tout. Cas de 40 Ko dans le self-test. |
| **S2** récupération fragile | **PARTIEL** | Corrigé : `set +e` (96), `trap : HUP INT TERM` (97), `trap - EXIT` (98), étapes chaînées, redémarrage final. X2 : TERM, HUP et INT envoyés au bash pendant la récupération, la restauration va au bout (ancien binaire, `web/` retiré, service actif, rc=1). X3 : disque plein, message `could NOT be restored`, le handler continue et démarre le service. **Non couvert : SIGPIPE (N1).** |
| **S3** sonde TLS avant l'arrêt | **CORRIGÉ** | Ligne 87. Scénario B (mauvais port) : sortie 2, même PID, aucune sauvegarde, aucun `web/`. X6 (service arrêté) : sortie 2 avant tout. |
| **S4** rollback contrôlé trop tard | **CORRIGÉ** | Lignes 76-81 en préflight. E : copie identique réutilisée. F : copie douteuse, sortie 2, service intact. |
| **S5** `web_moved` | **CORRIGÉ** | Ligne 182 (drapeau posé juste après le `mv`), restauration aux lignes 115-118. D : l'ancien `web/` revient avec son marqueur. |
| **S6** TOCTOU de l'archive | **NON CORRIGÉ, risque accepté et documenté** (en-tête, lignes 12-13) | Le fichier entrant n'est ni copié ni contrôlé en propriétaire/mode. Voir N7. |
| **S7** `--strict` et exemption | **CORRIGÉ** | Ligne 145 (`--strict`), lignes 147-149 (seul `./SHA256SUMS` exempté, comparaison exacte). Cohérence vérifiée sur le tar : diff vide. |
| **S8** `ss` fail-open | **CORRIGÉ** | Ligne 67 (`ss` et `curl` requis), ligne 154 (échec de `ss` donne sortie 2 avant l'arrêt). |
| **R1** routes `/api/` inconnues | **CORRIGÉ** | `web_static::serve` renvoie un 404 nu pour `/api` et `/api/…` avant le test de méthode ; `normalize_response` produit `ROUTE_NOT_FOUND`. Le test passe pour GET, POST, PUT et DELETE (102 passés). |
| **R2** `form-action 'self'` | **CORRIGÉ** | Présent dans `PAGE_POLICY` et dans les chaînes du binaire. |
| **R3** plafond mémoire | **CORRIGÉ** | `MAX_FILE_BYTES = 8 * 1024 * 1024`. |
| **R6** tests | **CORRIGÉ** (réserve mineure) | Origin `null`, non UTF-8, étranger, casse ; `cross-site` sur `/` ; POST avec `none` ; HEAD sans corps ; POST statique en 405 ; lien symbolique de répertoire. Restent la racine de test en thread-local (fonctionne, `web_root()` est appelé avant `spawn_blocking`) et l'absence de test sur les en-têtes dupliqués. |

## 4. Nouveaux constats

### N1 (MEDIUM) : SIGPIPE tue la récupération avant la première restauration
- **Où** : script:99-100. Le premier `echo "RECOVERY: …" >&2` est exécuté avant toute action ; seuls HUP, INT et TERM sont interceptés.
- **Scénario** : stderr ou stdout deviennent des tubes fermés. Cause plausible : perte de la connexion SSH ou du master pendant le déploiement. Le premier `echo` de la récupération reçoit SIGPIPE et bash meurt avant d'avoir restauré quoi que ce soit.
- **Reproduit en VM (X1)**. Échec de vérification (archive sans wasm) avec lecteur de stderr fermé : `rc=141`, binaire **nouveau non vérifié laissé installé**, `web/` présent, aucune restauration.
- **Pire cas**, non exécuté mais direct : un échec entre l'arrêt (ligne 158) et le démarrage (ligne 187), par exemple la sauvegarde, avec stderr fermé. Le script meurt à ce premier `echo` et **le service reste arrêté**. Ce cas contredit la promesse du plan §7 (« tout échec après l'arrêt remet l'ancien binaire »).
- **Correctif proposé** : ajouter `trap '' PIPE` en tête du script, ou au minimum au début du handler, juste après `set +e`.
  - Idéalement, écrire aussi les messages `RECOVERY:` avec `|| true`, ou dans un journal root privé sous `/root/chillcenter-rollback`.
  - **Testé** sur une copie `/tmp` avec `set -euo pipefail; trap '' PIPE` : le même scénario X1 donne `rc=1`, ancien binaire restauré, `web/` absent, service actif.

### N2 (LOW) : `trap : …` au lieu de `trap '' …` est un choix dicté par le faux `systemctl`
- La justification du plan (« un signal ignoré serait hérité par `systemctl` ») vient du faux `systemctl` de la simulation, qui lance le service comme enfant du script. Avec le vrai systemd, le service n'est pas un enfant du script.
- Avec `:`, les enfants (`systemctl`, `install`, `mv`) repassent à l'action par défaut. Un signal adressé au groupe de processus entier tuerait `install` ou `mv` en pleine restauration ; ce serait le cas pour un SIGHUP sur le pty créé par `sudo` (`use_pty` est le défaut en 1.9.14 et plus, donc sur Debian 13).
- Je n'ai pas testé ce chemin (SUPPOSÉ). Avec `''`, les enfants ignoreraient aussi le signal. Voir aussi N1.

### N3 (MEDIUM pour la preuve, LOW pour la sécurité) : les assertions négatives du `--self-test` sont inertes
- **Où** : script:44, 47-54 (`! parse_backup_path …`, `! validate_archive_listing …`). Sous `set -e`, une commande précédée de `!` ne déclenche jamais l'arrêt.
- **Preuve (mutation)** : si `validate_archive_listing` accepte tout (`return 0`), `--self-test` affiche quand même `parser_self_test=PASS`.
- Seules les assertions positives (lignes 43 et 46) et la ligne 56 comptent réellement. Le plan §4 et §7 s'appuient sur cet auto-test.
- **Correctif proposé** : `if parse_backup_path …; then exit 1; fi`, ou `parse_backup_path … && { echo FAIL; exit 1; }`.
- Aucun impact à l'exécution : le validateur lui-même est correct à la lecture, et testé indirectement par le scénario H.

### N4 (LOW) : le même défaut `!` rend la ligne 159 inopérante
- `! systemctl is-active --quiet "$service"` ne peut jamais interrompre le script.
- Sans conséquence réelle : `systemctl stop` (158) échoue déjà seul si l'arrêt échoue.
- À remplacer par `if systemctl is-active --quiet …; then exit 1; fi`, ou à retirer.

### N5 (LOW) : le message d'échec ne dit pas quel contrôle a échoué
- Les échecs `set -e` silencieux (ligne 192, 197 et autres) ne laissent que `RECOVERY: restoring (exit status 1)` (voir X0).
- Proposition : `trap 'echo "ERR line $LINENO: $BASH_COMMAND" >&2' ERR`, ou des `|| { echo …; exit 1; }` sur les assertions de vérification.

### N6 (LOW) : la restauration est validée seulement par `is-active`
- Le script ne contrôle pas, après récupération, que le service répond en TLS (la simulation C le vérifie en dehors du script).
- Si l'ancien binaire ne peut pas être restauré (X3), le service tourne avec le nouveau binaire non vérifié. Le code de sortie est non nul et le message est explicite. C'est un choix de conception acceptable.
- Si `systemctl stop` échoue pendant la récupération, le message `service is active` peut induire en erreur.

### N7 (LOW) : durcissement d'entrée non fait
- Ni le propriétaire ni le mode de `/var/tmp/chill-server-web.new` et `chillcenter-web.tar` ne sont contrôlés. La copie `install` puis re-hachage couvre le binaire ; l'archive reste lue plusieurs fois (S6 accepté).
- `CHILLCENTER_BASE_URL` n'est pas rapproché d'un port réellement écouté par `chill-server`. Une URL vers une autre machine du LAN avec le même certificat passerait la sonde et la vérification finale. Un contrôle `ss -ltnp` suffirait.
- Prérequis `openssl`, `runuser` et `seq` non vérifiés en préflight. `openssl` manquant échoue sans conséquence avant l'arrêt ; `runuser` manquant échoue après l'arrêt mais est récupéré (X4).
- `curl` hérite de `~/.curlrc` et des variables proxy de l'environnement root. Utiliser `curl -q --noproxy '*'` écarterait ce cas.
- Le seuil de 64 Mio libres est une heuristique, non reliée à la taille réelle de la sauvegarde.

### N8 (LOW, bancs d'essai) : scénarios manquants dans la simulation
- Le banc ne couvre ni SIGPIPE (N1), ni signal pendant la récupération, ni restauration impossible, ni échec de sauvegarde après l'arrêt, ni service déjà arrêté, ni `sudo` avec `use_pty`, ni stdin par vrai tube. Je les ai ajoutés dans mes scénarios X (§7). Tous passent sauf X1.
- Le scénario C n'affirme que « sortie non nulle » ; il ne contrôle pas le code ni le contenu de la sortie. X0 confirme la bonne raison : `rc=1`, `RECOVERY: restoring (exit status 1)`.

### INFO
- Le Rust n'a pas de test sur les en-têtes dupliqués ; `headers().get` ne lit que le premier, sans effet pour un navigateur.
- Sous 905 Mio de RAM, la concurrence illimitée sur `spawn_blocking` avec lecture intégrale du fichier (≈ 650 Ko pour le wasm) reste une surface de saturation pour un client sans en-têtes sur le LAN. C'est la même exposition que l'API, sans authentification.
- Doc §2 : le SPKI `3018fb4d…` n'est pas comparé à une constante dans le script (seule l'égalité avant/après l'est). Cohérent avec l'annonce « SPKI contrôlée ».
- Le plan §7 (restauration manuelle) est cohérent avec le script.
- Le plan §5a vérifie `sudo -n env true` mais pas `openssl` ni `runuser`.

## 5. Points du script vérifiés sans défaut (lecture, confirmée par les scénarios)
- `set -euo pipefail` : les `[[ "$(…)" == … ]]` nus échouent proprement ; ceux gardés par `|| {…}` sortent en 2 avant l'arrêt.
- `local status=$?` est bien capturé en premier ; `trap - EXIT` évite la double exécution.
- Les chemins avant `stop_attempted=1` n'ont aucun effet autre que le nettoyage des fichiers temporaires, restreint par préfixe, fichier et non-lien (B, F, G, H, X6).
- **Ordre** : binaire (`mv -f` atomique dans le même répertoire), puis `web`, puis démarrage. Les restaurations suivent le même ordre ; le `web/` n'est retiré que si `web_swapped=1` et que le binaire est cohérent.
- **Écrasement** : la copie de rollback utilise `ln` sans `-f` et ne peut jamais écraser. Le seul écrasement est celui du binaire, volontaire, lors de la restauration.
- **Variables** : hashes `[0-9a-f]{64}`, URL IPv4:port, certificat absolu, fichier, non-lien. Les valeurs sont toujours citées ; aucun `eval` ni `sed` sur des données distantes ; aucun `-k` ; aucun secret imprimé.
- **stdin** : tous les enfants sensibles ont `</dev/null` (`curl_tls`, `ss`, `runuser`) ou lisent un fichier ou un tube interne. Le vrai tube `cat | bash -s` (X5) et `sudo` avec `use_pty` (X7) donnent un déploiement nominal (rc=0, nouveau binaire, `web/` présent).
- **Config, unité, TLS** : jamais écrits ; comparaison par hash avant et après (A et C).
- **Différence avec V4** (diff lu) : hashes paramétrés ; rollback préexistant (la création du dossier est retirée) ; récupération inconditionnelle après arrêt ; web, `df`, sonde et vérifications ajoutés. Aucune autre dérive.

## 6. Diff Rust
- **Garde** (`check_browser_origin_and_host`) :
  - le `match` ne laisse passer que `(None, None)`, `(same-origin, None)`, `(same-origin, Origin == https://<Host>)` (insensible à la casse) et `(none, None)` en GET hors `/api/`. Tout le reste est refusé : `cross-site`, `same-site`, `Origin: null`, valeur non UTF-8, Origin sans Sec-Fetch-Site.
  - Host non UTF-8 : repli sur l'authority de l'URI, sinon 400.
  - Le contrôle Host et `tls_names` reste exécuté ensuite : le DNS rebinding donne `HOST_REFUSED`.
  - Le timeout de 30 s est conservé ; la fonction synchrone corrige le futur non `Send`.
- **Service statique** :
  - `relative_name` refuse les segments vides ou commençant par `.`, ainsi que `\`, NUL, `%`, et les noms de plus de 200 octets.
  - `read_file` canonicalise la racine et le fichier et compare à `root.join(name)` : lien symbolique de fichier ou de répertoire refusé (test).
  - MIME fermé ; seuls GET et HEAD sont acceptés ; les fichiers spéciaux sont refusés par `is_file`.
  - `/api` et `/api/…` rendent un 404 nu (R1).
  - La CSP est posée seulement sur les pages HTML.
  - Aucune régression de contournement trouvée.
- **Rest** : 102 tests passent, dont les nouveaux.

## 7. Résultats de mes essais en VM (Ubuntu 24.04 aarch64, GNU tar 1.35, curl 8.5.0, sudo 1.9.15, root dans la VM)

| Essai | Résultat |
|---|---|
| e2e (20 contrôles) | 20 PASS |
| Simulation A à H | 34 PASS, 0 FAIL |
| X0 échec de vérification | rc=1, ancien binaire, `web/` absent, service actif |
| **X1 SIGPIPE** | **rc=141, nouveau binaire non vérifié laissé, `web/` présent, aucune restauration** (N1) |
| X2 TERM, HUP, INT pendant la récupération | restauration complète, rc=1 |
| X3 disque plein pendant la récupération (tmpfs 100 Mo) | rc=1, `could NOT be restored`, service actif sur le nouveau binaire, `web/` laissé, cohérent avec la conception |
| X4 sauvegarde en échec après l'arrêt (backups/ en 0755) | rc=1, ancien binaire restauré, service actif |
| X5 vrai tube | rc=0, nominal |
| X6 service déjà arrêté | rc=2 avant tout arrêt |
| X7 via `sudo` (`use_pty`) | rc=0, nominal |
| N1 corrigé (`trap '' PIPE`, copie /tmp) | X1 passe : rc=1, ancien binaire restauré, `web/` absent, service actif |

## 8. Non vérifié, risques résiduels
- Exécution réelle sur le Pi : Debian 13 (sudo 1.9.16, coreutils, curl 8.14, `ss`/`runuser`/`openssl` installés), systemd réel (le faux `systemctl` ne reproduit ni les jobs de stop ni les limites de redémarrage), sudoers et `env`.
- Comportement d'un SIGHUP sur le pty de `sudo` (N2) et d'une coupure SSH réelle.
- Provenance du binaire : pas de reconstruction ; je me fonde sur les mtime, le commit, les chaînes et l'absence de diff.
- clippy, build ARM64 et `node --test` non rejoués.
- Navigateur réel : validité de la CSP, Service Worker sur certificat auto-signé, performance du Pi 3B+.
- Durée d'interruption non mesurée.

## 9. Conditions pour lever CHANGES_REQUESTED
1. Corriger **N1** (au minimum `trap '' PIPE`), puis nouveau hash du script, nouvelle revue de ces octets, rejeu de la simulation avec un scénario SIGPIPE.
2. Recommandés : N3 et N4 (assertions `!` inertes), N5 (message d'erreur), N2 (`trap ''` pour HUP, INT, TERM).
3. Mettre à jour le plan (§2 hash et taille du script, §3, §4) en conséquence. Aucun changement requis côté Rust ni archive.
