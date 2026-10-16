# Revue indépendante n° 1 — déploiement web sur le Pi (script V5, diff Rust, artefacts)

- Date : 15 octobre 2026. Demandée par l'utilisateur (« Oui : lance une revue indépendante… Rapporte son verdict tel quel dans docs/reviews/ »).
- Reviewer : sous-agent `a13028dbf0d12ff2c` (type `Explore`, outils d'édition désactivés, instruit de ne toucher ni au dépôt ni au Pi). **Distinct de l'auteur** (`/root`, la session principale). Il n'a reçu ni les conclusions ni l'avis de l'auteur sur la sûreté.
- Auteur du code revu : session principale. L'auteur n'a pas modifié le texte ci-dessous ; seul cet en-tête est de l'auteur.
- Objets et octets couverts : script `scripts/pi-update-web-operator.sh` (13 078 o, SHA-256 `8d47e9df8feba5c234552ff61517fd04e29a8612c2a3e14cabfc3b420fbb9d14`), diff Rust `0d0da0e..05e637dec0b721300925be14e46c94c26d331789`, archive web `1aaf79a6…cc669` et binaire ARM64 `c3681a47…2d2c47`.
- **Verdict global rendu : CHANGES_REQUESTED** (uniquement à cause du script). Rust : APPROVED_WITH_CONDITIONS. Artefacts : APPROVED pour ce qui est vérifiable localement.
- Suite : ce verdict n'autorise **aucune** exécution sur le Pi. Les corrections éventuelles du script changent son hash et exigent une nouvelle revue de ces octets.
- Limite connue de cette revue, signalée par le reviewer lui-même : l'agent n'a pas pu rejouer clippy, le build ARM64 ni l'essai VM (colima interdit), il les a seulement lus.

---

## Rapport du reviewer (verbatim)

# Revue indépendante : script V5, diff Rust, artefacts (lecture seule)

## Verdicts

| Objet | Octets / hash couverts | Verdict |
|---|---|---|
| Script `scripts/pi-update-web-operator.sh` | 13078 o, SHA-256 `8d47e9df8feba5c234552ff61517fd04e29a8612c2a3e14cabfc3b420fbb9d14` (vérifié identique) | **CHANGES_REQUESTED** (aucun CRITICAL ni HIGH, mais 3 défauts MEDIUM à corriger) |
| Diff Rust `0d0da0e..05e637dec0b7…` : `api.rs`, `web_static.rs`, `lib.rs` (+271/−11) | tel que commité | **APPROVED_WITH_CONDITIONS** |
| Artefacts : tar web et binaire ARM64 | tar 1099264 o, `1aaf79a6…cc669` ; binaire 14601224 o, `c3681a47…2d2c47` | **APPROVED** pour tout ce qui est vérifiable localement |
| **Global** | | **CHANGES_REQUESTED**, uniquement à cause du script |

Les verdicts sur le Rust et les artefacts restent valables si seul le script change. Toute modification du script change son hash et impose une nouvelle revue de ces octets.

## Vérifications effectuées

Légende : V = exécuté ou mesuré ici ; L = lu ; S = supposé ou non vérifiable.

| Vérification | Résultat | Type |
|---|---|---|
| `shasum -a 256` et `wc -c` du script | hash et 13078 o conformes | V |
| `bash -n` du script | OK | V |
| `bash scripts/pi-update-web-operator.sh --self-test` | `parser_self_test=PASS` | V |
| Lecture intégrale du script, lignes 1 à 168, et diff complet contre V4 | voir constats | L |
| V4 `/private/tmp/chillcenter-pi-update-operator-v4.sh` | 6933 o, `55c6be48…9587`, identique au BUG-0051 | V |
| `git rev-parse HEAD` | `cfcddbb` | V |
| `git status --porcelain` | vide | V |
| `git diff --stat 05e637d HEAD` | uniquement 2 docs et 2 scripts (`package-web-pi.sh`, `web-e2e-linux.sh`) ; `crates/` vide | V |
| Lecture intégrale du diff `crates/` (3 fichiers) | voir constats | L |
| `cargo test --locked --offline -p chill-server` | 101 passés, 0 échec ; 2 tests d'intégration passés ; les 4 nouveaux tests (`same_origin_browser…`, 3 de `web_static`) sont OK | V |
| `cargo clippy` et build ARM64 | non rejoués ; l'affirmation du doc est seulement lue | L |
| Archive `shasum` et taille | 1099264 o, `1aaf79a6…cc669` | V |
| Archive `tar -tvf` | 58 entrées : 54 fichiers (`-`) et 4 répertoires (`d`) ; 0 lien, 0 périphérique ; 0 entrée hors `web/` ; pas d'en-têtes PAX ni `._`/xattrs (format ustar) | V |
| Extraction de l'archive dans le répertoire scratchpad | `shasum -c SHA256SUMS` OK ; couverture exacte (53 lignes = 53 fichiers hors SHA256SUMS) ; 0 fichier spécial ; `du` ≈ 1,1 Mo, bien sous 8 Mio | V |
| Hashes `vendor/mpeg4/` | wasm `453fb7b1…bda53` (652895 o) et `mpeg4-decoder.js` `64d97f89…e827` conformes au doc et à `BUILD-INFO.json` ; `NOTICE-LGPL.md` cohérent (FFmpeg 9.0.2, `8c385028…002e`) | V |
| Binaire : `file` et hash | ELF 64-bit aarch64, dynamique, strippé ; 14601224 o ; `c3681a47…2d2c47` | V |
| `strings` du binaire | contient la CSP exacte, `chillcenter-web`, `BROWSER_ORIGIN_REFUSED` | V |
| Provenance du binaire | mtime du binaire 12:32:59 ; `api.rs` modifié à 12:29:48, `web_static.rs` à 12:23:20 ; aucun fichier `crates/`, `Cargo.*` ou `rust-toolchain.toml` plus récent que le binaire ; commit 05e637d à 12:33:15 | V (indices mtime, pas une preuve cryptographique) |
| Pas de migration SQLite ni de dépendance dans le diff Rust | `Cargo.*` inchangés, 3 fichiers seulement | V |
| Base de la branche | `git diff 9277ac4 0d0da0e -- crates` est vide, donc le delta entre V4 et le nouveau binaire est bien ce diff | V |
| `grep innerHTML/eval/document.write/new Function` sur `*.js` du client | aucune occurrence (hors `vendor/`) ; pas de script inline dans `index.html` ; pas de `<script>` dans les SVG `default.svg` (seul contrôlé) | V |
| Essai VM Linux « 20/20 », durée d'interruption, comportement du Pi (GNU tar, coreutils, curl, ss, sudoers) | colima interdit, rien de tout cela n'a été exécuté | S |

## Constats : script

**S1 (MEDIUM) : ligne 159, `curl_tls … | grep -q` sous `pipefail` + `errexit` : faux échec possible, donc rollback automatique inutile.**
- `index.html` fait 17 589 o, plus qu'un enregistrement TLS de 16 Kio, donc `curl` écrit en au moins deux fois.
- `grep -q` quitte dès le premier match. Si `curl` écrit après la fermeture du tube, il sort en code 23 (ou SIGPIPE).
- `pipefail` fait alors échouer le pipeline et `set -e` termine le script. Le trap arrête le service, restaure l'ancien binaire et supprime `web/`, alors que le déploiement était sain.
- Une simulation locale montre `rc=141` avec `grep -q` après `head -c 17000`, même sans délai.
- `scripts/web-e2e-linux.sh` n'est pas équivalent : il n'a pas `-e` et un `&& echo yes || echo no`.
- Correctif :
  - `page="$(curl_tls "$CHILLCENTER_BASE_URL/")"; [[ "$page" == *'<title>Chill Center</title>'* ]]`
  - même précaution si d'autres `| grep -q` sont ajoutés.

**S2 (MEDIUM) : le handler `recover_on_failure` s'exécute avec `errexit` actif et réinitialise les traps.**
- Dans le corps du `if` (lignes 78 à 81), `mktemp`, `install`, le `[[ hash ]]` nu et `mv -f` ne sont pas gardés. Si l'un échoue (disque plein sur `/usr/lib/chillcenter`, par exemple), le shell sort immédiatement.
- Les lignes 84 à 90 ne sont alors pas exécutées : pas de restauration de `web/`, pas de `systemctl start`. Le service reste arrêté.
- La ligne 72 fait `trap - EXIT HUP INT TERM` au début du handler. Un SIGHUP ou SIGTERM pendant la restauration (coupure SSH) tue donc le handler au milieu.
- V4 avait la même structure. V5 ajoute la restauration web derrière ces mêmes commandes, ce qui accroît l'enjeu.
- Correctif :
  - `set +e` en début de handler, et `|| true` ou un journal sur chaque étape ;
  - `trap '' HUP INT TERM` plutôt que `trap -`.

**S3 (MEDIUM) : aucune sonde TLS ou URL avant l'arrêt du service.**
- Si le SAN du certificat ne contient pas l'IP de `CHILLCENTER_BASE_URL`, si `--cacert` d'un certificat auto-signé n'est pas accepté, si le port est faux ou si le Host est refusé, l'échec n'apparaît qu'à la ligne 154, après arrêt, sauvegarde, remplacement et redémarrage.
- Le résultat est une interruption complète, une sauvegarde inutile et un rollback.
- Correctif : avant la ligne 120, faire `curl_tls … /api/2026-10-14/server` contre le service V4 encore actif et exiger 200. Le script V4 sait déjà répondre 200 à cette route (BUG-0051).

**S4 (LOW) : le contrôle « rollback déjà existant » (ligne 136) arrive après l'arrêt et la sauvegarde.**
- Si un run précédent a échoué après la ligne 140 (le fichier `chill-server-1eb24e97bcdf` existe alors), le run suivant arrête le service, crée une sauvegarde, puis sort en code 2. Le trap redémarre.
- Correctif : déplacer le contrôle en préflight, et accepter la réutilisation si le hash du fichier existant vaut `old_expected`.

**S5 (LOW, redéploiement uniquement) : `web_swapped=1` n'est posé qu'à la ligne 148.**
- Un échec entre la ligne 145 (`mv -T web_dir web_prev`) et la ligne 148 laisse l'ancien `web/` dans `/root/chillcenter-rollback/web-*` sans le remettre. La promesse « restaure l'ancien s'il existait » du doc §7 est alors fausse.
- Sans impact pour ce run, car `web/` n'existe pas sur le Pi (`web_prev` est vide).
- Correctif : poser un drapeau `web_moved` juste après le `mv` vers `web_prev`.

**S6 (LOW) : TOCTOU sur l'archive web dans `/var/tmp`.**
- L'archive est lue trois fois : hash (ligne 60), listing (64), extraction (107). Seul le binaire est copié puis re-haché.
- L'attaquant potentiel serait le propriétaire du fichier, qui a déjà `sudo -n`, donc pas de frontière de privilège.
- Correctif : `install` du tar vers un répertoire root privé, puis hash, listing et extraction depuis cette copie.

**S7 (LOW) : `sha256sum -c --quiet` sans `--strict`.**
- Les lignes malformées sont ignorées par `sha256sum -c`, mais comptées par `awk '{print $2}'` dans la comparaison de couverture.
- `find ! -name SHA256SUMS` exclut aussi un éventuel `SHA256SUMS` imbriqué dans un sous-répertoire.
- L'archive étant épinglée par hash, l'impact est faible.
- Correctif : ajouter `--strict`, et n'exclure que `./SHA256SUMS`.

**S8 (LOW) : le contrôle des connexions établies échoue ouvert.**
- La ligne 121 fait `ss … 2>/dev/null | grep || true`. Si `ss` est absent ou échoue, la liste est vide et le script arrête le service malgré tout.
- Correctif : `command -v ss` dans le préflight.

**S9 (INFO)**
- Aucune vérification d'espace disque dans le script. Seul le préflight manuel du plan §5a (`df`) la couvre.
- La barrière `--self-test` ne teste que 5 cas de `validate_archive_listing`. Par lecture, elle refuse correctement : `..`, chemin absolu, liens `l` et `h`, périphériques, nom hors `web/` (y compris un fichier nommé `web`), détail vide, et un nom contenant un retour à la ligne (fail-closed, car la ligne suivante doit commencer par `web/` et par `-` ou `d`). Je n'ai pas pu exécuter de cas supplémentaires : l'outil a refusé la construction de la commande.
- L'en-tête du script cite `sudo -n /bin/bash -s --`, mais le plan §5c utilise `sudo -n env VAR=… /bin/bash -s --`. Il faut que sudoers autorise `env` (non vérifié ; le préflight 5a peut le tester).

**Points vérifiés et corrects (par lecture)**
- `set -euo pipefail` : toutes les substitutions dans `[[ ]]` échouent de façon sûre.
- `$?` est correctement capturé en tête du handler.
- Avant l'installation du trap (lignes 1 à 67) et avec `stop_attempted=0`, un échec ne touche à rien. Le nettoyage des fichiers temporaires est limité par préfixe, fichier et non-lien.
- L'ordre est correct : binaire (rename atomique dans le même répertoire), puis `mv -T` du web, puis `start`. Le service est arrêté pendant toute la fenêtre.
- Les chemins d'échec après `stop_attempted=1` redémarrent le service (backup, rollback, `mv`, curl, invariants).
- Les chemins sont robustes à un EOF de stdin, car `bash -s` lit le script incrémentalement et l'EXIT trap se déclenche alors.
- Le rollback `mktemp` + `install` + hash + `ln` (sans `-f`) + `rm` ne peut jamais écraser une cible existante.
- Les liens symboliques sont refusés sur `/usr`, `/usr/lib`, `/usr/lib/chillcenter`, le binaire, `web_dir`, `/etc`, `/etc/chillcenter`, la config, `/root`, les deux fichiers entrants et `/var/tmp`.
- Les variables d'environnement sont validées par regex ancrées : hash en minuscules et différent de `old_expected`, URL de la forme IP:port. Aucun secret n'est manipulé, et `curl` n'est jamais appelé avec `-k`.
- Aucun `eval` ni `sed` sur des données distantes ; le script est transmis par stdin.
- Les différences avec V4 sont toutes justifiées :
  - l'arrêt est désormais inconditionnel dans le handler (nécessaire pour retirer `web/` service arrêté) ;
  - le répertoire de rollback est supposé exister ;
  - les contrôles web et curl sont ajoutés ;
  - les hashes sont paramétrés.

**Écarts plan et script (§5 et §7)**
- Les noms `/var/tmp/chill-server-web.new` et `/var/tmp/chillcenter-web.tar` concordent. Le nettoyage de `/var/tmp` est à la charge de l'opérateur, comme annoncé.
- La sortie du script (date, PID, hashes, `web_previous`, `backup_path`) concorde.
- Le rollback manuel correspond au nom `chill-server-1eb24e97bcdf`.
- Le seul écart réel est S5.

## Constats : diff Rust

Lu intégralement. `web_static.rs` : 182 lignes ; garde dans `api.rs` ; `mod web_static;` dans `lib.rs`.

**R1 (LOW) : changement de comportement pour les routes API inconnues.**
- Le `.fallback(web_static::serve)` global renvoie `NOT_FOUND` en JSON, au lieu de `ROUTE_NOT_FOUND` produit par `normalize_response`. Cette branche 404 devient morte.
- Un POST ou PUT sur un chemin inconnu renvoie désormais 405 (sans en-tête `Allow`) au lieu de 404.
- Rien dans `crates/`, `docs/` ou `ui/` ne référence `ROUTE_NOT_FOUND` hors `api_version.rs`.
- Correctif : retourner 404 pour tout chemin commençant par `/api/`, avant le test de méthode.

**R2 (LOW) : CSP.**
- `script-src 'self' 'wasm-unsafe-eval'` est correct : pas d'inline, pas de `eval`, workers et wasm couverts.
- `connect-src 'self'` couvre l'EventSource vers `/api/…/playback/events`. Aucune URL externe n'est présente dans le JS/CSS/HTML.
- Aucun trou majeur trouvé.
- Manques :
  - pas de `form-action 'self'`, que `default-src` ne couvre pas ;
  - `style-src 'unsafe-inline'` ;
  - `worker-src blob:` ;
  - la CSP n'est posée que sur les réponses `text/html`, donc pas sur un SVG consulté en navigation directe (les SVG actuels sont sains) ;
  - aucune validation avec un vrai navigateur.
- Correctif : ajouter `form-action 'self'` ; tester dans un navigateur après déploiement.

**R3 (LOW) : lecture intégrale en mémoire, plafond de 32 Mio.**
- Un tel plafond est sans objet avec une archive limitée à 8 Mio par le script, mais risqué sur 905 Mio de RAM avec des requêtes concurrentes (LAN, sans authentification).
- Correctif : ramener `MAX_FILE_BYTES` à 8 Mio ou moins.

**R4 (INFO) : cache.**
- Les JS et CSS ont `max-age=3600` ; `index.html`, `sw.js` et le manifeste sont en `no-cache`.
- Lors d'une mise à jour future, `cache.addAll` du service worker peut précacher d'anciens fichiers depuis le cache HTTP pendant une heure (utiliser `{cache:'reload'}`). Sans effet pour ce premier déploiement.

**R5 (INFO) : ordre des contrôles.**
- Le Host est désormais exigé avant l'Origin. Une requête sans Host mais avec Origin donne 400 (au lieu de 403).
- Le timeout de 30 s couvre la lecture du fichier statique dans `spawn_blocking`, sans conséquence pour des fichiers de moins de 1 Mio. Il ne couvre pas le streaming du corps (inchangé).
- Si `current_exe()` échoue, la racine devient `/nonexistent/chillcenter-web` et tout répond 404 : sûr. Sous Linux, le suffixe « (deleted) » ne gêne pas, car seul le parent est utilisé.

**Garde `check_browser_origin_and_host` : analyse**
- Par lecture, je n'ai trouvé aucun contournement par un site tiers.
- `Sec-Fetch-*` est un en-tête interdit côté script. Un site tiers envoie `cross-site` ou `same-site`, donc 403.
- Une autre origine sur le même hôte avec un autre port est `same-site` : 403.
- Cas refusés, tous en fail-closed :
  - `Origin: null` ;
  - valeur non UTF-8 ;
  - `Origin` seul sans `Sec-Fetch-Site` (anciens navigateurs : le client web ne fonctionnera pas, c'est le comportement avant le changement) ;
  - `none` sur `/api/` ;
  - `none` avec une méthode autre que GET ;
  - `Origin` différent de `https://<Host>` (comparaison insensible à la casse, port par défaut omis des deux côtés : cohérent avec les navigateurs).
- En-têtes dupliqués : `headers().get` ne lit que le premier, mais un navigateur ne peut pas en forger. Un client non navigateur n'a de toute façon aucune barrière (API volontairement sans authentification).
- DNS rebinding : le Host doit figurer dans `tls_names` avec le port d'écoute, sinon 403 `HOST_REFUSED`. Cela s'applique aussi aux fichiers statiques et au fallback.
- Clients natifs sans en-têtes : `(None, None)` est accepté, donc pas de régression.
- Surface d'attaque par rapport à l'original : l'autorisation de `same-origin` n'ouvre rien à un site tiers. Le risque ajouté est qu'une XSS dans le client web ait un accès total à l'API (sans authentification). Il est borné par la CSP et par l'absence de `innerHTML`/`eval` (vérifié par grep) ; les données NAS et TMDb restent à échapper (non revu).
- Un site tiers peut toujours émettre des GET sans en-têtes depuis de très vieux navigateurs, comme avant le changement. Il n'en lit pas le résultat (pas de CORS).

**Service statique** (lu, plus tests unitaires exécutés)
- `relative_name` refuse tout segment vide ou commençant par `.`, ainsi que `\`, NUL et `%`, donc pas de traversée possible. `SHA256SUMS` et les extensions inconnues donnent 404 (MIME fermé).
- La canonicalisation de la racine puis du fichier, avec comparaison à `root.join(name)`, refuse les liens symboliques à n'importe quel niveau.
- Méthodes : GET et HEAD uniquement. HEAD renvoie des en-têtes complets avec un corps vide, y compris le `content-length`.
- Tests ajoutés : pertinents, mais incomplets (voir R6).

**R6 (LOW) : tests manquants** (non bloquant)
- Origin `null`, casse, valeur non UTF-8.
- `cross-site` sur `/` (la statique doit être refusée aussi).
- HEAD et POST sur un fichier statique (couverts seulement par l'essai VM, non rejoué ici).
- `none` avec POST.
- Lien symbolique sur un répertoire (seul un lien sur un fichier est testé).
- La racine de test passe par une variable thread-local, fragile avec des runtimes multi-thread.

## Constats : artefacts et document

- Hashes et tailles : tous conformes (tar, binaire, wasm, js). `build.json` annonce `files: 52`, soit 54 moins `build.json` et `SHA256SUMS`, ce qui est cohérent (INFO).
- Le dépôt n'a aucune modification Rust non commitée ; `05e637d..HEAD` ne touche pas `crates/`. L'affirmation « les commits suivants ne touchent que la documentation et le paquetage » est exacte (docs, `package-web-pi.sh`, `web-e2e-linux.sh`).
- Chiffres de tests du doc (101 passés, 2 d'intégration, 0 échec) : confirmés par mon exécution.
- Essai de bout en bout « 20/20 » en VM Ubuntu, clippy et build ARM64 : seulement lus.
- Le doc §8 reconnaît à juste titre que l'exécution sur le Pi, la durée d'interruption et le comportement dans un navigateur ne sont pas prouvés.
- Divergence plan/script : le §7 « restaure l'ancien web s'il existait » est incomplet pour S5.

## Risques résiduels et non vérifié

- Comportement réel sur le Pi (Debian 13) : GNU `tar -tvf` et `--no-same-permissions`, `mv -T`, `find -printf`, `du -sb`, `stat -c`, `ss -Htnp`, versions de `curl` et d'`openssl`, sudoers pour `env`.
- Validité TLS par `--cacert` sur le certificat auto-signé réel et présence de `chillcenter.local` dans les SAN et dans `tls_names` ; le flux `/api/2026-10-14/server` n'a pas été testé depuis le Pi.
- Compatibilité de la CSP avec la lecture réelle (MSE, wasm, EventSource) dans un navigateur.
- `ProtectSystem=strict` n'empêche pas la lecture de `/usr/lib/chillcenter/web` (logique, non testé).
- La provenance du binaire repose sur les mtime et les chaînes, pas sur une reproduction du build.

## Conditions pour lever CHANGES_REQUESTED

1. Corriger S1, S2 et S3 (S4 et S5 recommandés), produire un nouveau script avec nouveau hash et le re-soumettre à revue.
2. Rust et artefacts : aucune modification n'est exigée ; R1 et R2 sont recommandés mais acceptables en l'état.
3. Après déploiement : tester l'interface dans un navigateur pour valider la CSP.
