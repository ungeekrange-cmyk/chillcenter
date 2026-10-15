# Revue indépendante n° 3 — déploiement web sur le Pi (script V7)

- Date : 15 octobre 2026. Demandée par l'utilisateur (« lance la revue indépendante n°3 … sur les octets exacts du script V7 b61b9fd6… (17609 o) … puis consigne son rapport verbatim »).
- Reviewer : sous-agent `ae25a68e5a663cac1` (type `Explore`, outils d'édition désactivés, **nouvel agent**, distinct de l'auteur et des reviewers des revues n° 1 et n° 2). Il pouvait rejouer les essais dans la VM Linux jetable locale ; il l'a fait.
- Auteur du code revu : session principale. L'auteur n'a pas modifié le texte ci-dessous ; seul cet en-tête est de l'auteur.
- Octets couverts : script V7 17 609 o `b61b9fd6274f19669dcddabcf180e443b865f8f3777529b63d57ebad07e64872` (218 lignes lues) ; Rust et artefacts inchangés (`3a0d53a`, tar `486e8eb2…0756`, binaire `24120568…ac29`).
- **Verdict global rendu : APPROVED_WITH_CONDITIONS** (aucun CRITICAL, HIGH ni MEDIUM ; 3 LOW et 3 INFO). **Toute modification du script, même d'un commentaire, annule cette revue.**
- Ce verdict **n'autorise pas à lui seul** l'exécution : elle exige en plus le feu vert explicite de l'utilisateur et une connexion SSH maître ouverte par lui.
- Conditions du reviewer : (1) corriger la ligne périmée « 34 PASS » du plan — fait en documentation, voir le plan ; (2) vérifier `openssl` et `runuser` à la main ou accepter un refus en sortie 2 avant tout arrêt — le préflight §5a les vérifie désormais ; (3) accepter LOW-1 (le contrôle d'URL ne vérifie que le port) — accepté et documenté dans le plan.

---

## Rapport du reviewer (verbatim)

# Revue indépendante n°3 : script opérateur V7 (Pi 3B+), Rust et artefacts, bancs d'essai, plan

Je n'ai modifié aucun fichier du dépôt et je n'ai touché ni git ni le Pi. Mes fichiers temporaires sont sous le scratchpad et dans la VM jetable. La VM est nettoyée et arrêtée (`colima stop`).

## 1. Verdicts

| Objet | Octets / hashes couverts | Verdict |
|---|---|---|
| Script V7 `scripts/pi-update-web-operator.sh` | 17609 o, SHA-256 `b61b9fd6274f19669dcddabcf180e443b865f8f3777529b63d57ebad07e64872` (vérifié identique). Les 218 lignes ont été lues. | **APPROVED_WITH_CONDITIONS** (aucun CRITICAL/HIGH/MEDIUM ; 3 LOW et 3 INFO, voir §5) |
| Rust et artefacts | `git diff 3a0d53a..HEAD -- crates Cargo.toml Cargo.lock rust-toolchain.toml` vide. Tar 1099264 o `486e8eb2…0756`. Binaire 14601992 o `24120568…ac29`. | **APPROVED** (inchangés) |
| Bancs d'essai | `pi-operator-simulation.sh`, `selftest-mutation.sh`, `web-e2e-linux.sh` | **APPROVED** (réserves LOW/INFO) |
| Plan `docs/deployment/WEB-CLIENT-PI.md` | §2 hash et taille V7 corrects | **APPROVED_WITH_CONDITIONS** (une ligne périmée, L1-doc) |
| **Global** | | **APPROVED_WITH_CONDITIONS** |

Conditions, toutes sans changement du script :
1. Corriger ou ignorer sciemment la ligne périmée « 34 PASS » du plan (ligne 115).
2. Contrôler `command -v openssl runuser` à la main avant l'exécution, ou accepter qu'un manque soit refusé en sortie 2 avant tout arrêt.
3. Accepter le risque résiduel LOW-1 : le contrôle d'URL ne vérifie que le port. Le Pi réel a un seul hôte cible, donc c'est acceptable.

Toute modification du script, même d'un commentaire, annule cette revue.

## 2. Tableau des vérifications

| Vérification | Résultat | Statut |
|---|---|---|
| `wc -c` et `shasum -a 256` du script | 17609 o, hash conforme | VÉRIFIÉ |
| `bash -n` et `--self-test` | `parser_self_test=PASS` | VÉRIFIÉ |
| `selftest-mutation.sh` | m1 (validateur qui accepte tout) rc=1 `SELF-TEST FAILED`. m2 (analyseur qui accepte tout) rc=1. Original rc=0. | VÉRIFIÉ |
| `git status --porcelain` | vide ; HEAD `a5d040e` | VÉRIFIÉ |
| `git diff 3a0d53a..HEAD -- crates Cargo.toml Cargo.lock rust-toolchain.toml` | vide | VÉRIFIÉ |
| Tar : taille, hash, contenu | 1099264 o, hash conforme, 54 `-` et 4 `d` | VÉRIFIÉ |
| Binaire : taille, hash | 14601992 o, hash conforme | VÉRIFIÉ |
| `cargo test --locked --offline -p chill-server` | 102 passés + 2 tests d'intégration, 0 échec | VÉRIFIÉ |
| `vm-e2e.sh` en VM | 20 PASS, 0 FAIL | VÉRIFIÉ |
| `pi-operator-simulation.sh` en VM | **45 PASS, 0 FAIL** (A à J et N) | VÉRIFIÉ |
| Copie testée (UID 997 → 4997) | `sed` refait localement : hash `748c908f…760f`, identique à celui affiché par la simulation. `diff` : exactement 2 lignes (76 et 176), seulement le nombre. | VÉRIFIÉ |
| Mes scénarios adverses X1 à X12 | 19 PASS, 0 FAIL, plus 1 constat informatif (X9). Voir §4. | VÉRIFIÉ |
| Lecture ligne à ligne : équilibre `if`/`else`/`fi` du handler (lignes 108-135), portées `set +e`, `trap - EXIT ERR` | cohérent, `bash -n` OK | LU |
| Debian 13 réel, systemd réel, `sudo` (`use_pty`, `env_reset`), RAM 905 Mio, durée d'interruption | non testable | SUPPOSÉ |

## 3. Constats de la revue n°2 (N1 à N8) et de la revue n°1 (S1 à S8)

| Constat | Statut | Preuve |
|---|---|---|
| **N1** SIGPIPE | **CORRIGÉ** | Ligne 16 `trap '' PIPE`. Simulation I : rc=1, pas 141. Mutation X10 : le script sans cette ligne donne rc=141 à ce même scénario. Le scénario détecte donc la régression pour la bonne raison. X1 (pire cas : sauvegarde impossible après l'arrêt, lecteur de sortie déjà parti) : rc=1, ancien binaire, `web/` absent, service actif. |
| **N2** `trap :` vs `trap ''` | **CORRIGÉ** | Ligne 106 `trap '' HUP INT TERM` dans le handler. X3 : HUP, INT et TERM envoyés au groupe entier (`setsid`, `kill -- -pgid`) pendant la récupération, alors que `systemctl start` dort 4 s. Récupération menée au bout : ancien binaire, `web/` retiré, service actif, TLS ok. X4 : TERM au groupe pendant `systemctl stop` du flux principal : rc=143, service actif sur l'ancien binaire. |
| **N3** assertions inertes | **CORRIGÉ** | `must_fail` (ligne 45). Preuve par mutation vérifiée par moi (m1 et m2 détectés). |
| **N4** `! systemctl is-active` | **CORRIGÉ** | Ligne 171, `if … then … exit 1`. Le chemin « stop impossible » (X8) passe par la ligne 170 : arrêt qui échoue, récupération, service intact. |
| **N5** diagnostic | **CORRIGÉ** | `set -E` et `trap … ERR` (ligne 18). Les sorties de X2, X3 et X8 nomment la ligne et la commande, par exemple `ERR line 199: systemctl start "$service"`. Aucune ligne `ERR` sur une exécution nominale (X6 : 0). |
| **N6** contrôle après récupération | **CORRIGÉ** | Sonde TLS lignes 131-134 : `sleep 2` plus `curl --max-time 10`, au plus 12 s, et un échec ne bloque pas. Message dédié si le service est actif mais ne répond pas. Message explicite ligne 111 si le service ne s'arrête pas (X8). |
| **N7** durcissement | **PARTIEL** | Corrigé : `curl -q --noproxy '*'` (X6 : proxy et `~/.curlrc` hostiles, déploiement rc=0). Outils requis vérifiés ligne 72 (X11 : `runuser` absent donne la sortie 2 avant tout). Cas `ss` : X7 refuse si `ss` ne montre pas le process, ou un autre port, ou la bonne ligne sans `chill-server`, et accepte le format IPv6 `[::]:18443`. Non corrigé : l'hôte de l'URL n'est pas comparé à l'adresse d'écoute, seulement le port (LOW-1). Le commentaire ligne 92 et le plan promettent plus (« pas un autre hôte du LAN avec le même certificat »). S6 et le seuil de 64 Mio restent acceptés. |
| **N8** scénarios | **CORRIGÉ** | I, J et N présents. J échoue bien pour la bonne raison : le dossier des sauvegardes est en 0555 root, et `ERR line` apparaît. N : le service est arrêté, sortie 2, le script ne le démarre pas. |
| **S1 à S5, S7, S8** | **CORRIGÉ** (inchangé, confirmé par A à H) | |
| **S6** | **NON CORRIGÉ, risque accepté et documenté** (lignes 12-13) | |

## 4. Résultats de mes scénarios adverses

| Scénario | Résultat |
|---|---|
| X10 mutation : V7 sans `trap '' PIPE` | Scénario I échoue (rc=141). Le banc détecte N1. |
| X1 sauvegarde impossible après l'arrêt, lecteur de sortie parti | rc=1, service revenu, ancien binaire. OK |
| X2 `systemctl start` échoue, y compris pendant la récupération | Ancien binaire restauré, `web/` retiré. Message `SERVICE IS NOT ACTIVE`. Pas de blocage, durée de 0 s. Le service reste arrêté puisque `start` échoue, mais c'est dit explicitement. |
| X3 HUP, INT et TERM au groupe pendant la récupération | Récupération complète. OK |
| X4 TERM au groupe pendant `systemctl stop` | Service actif, ancien binaire, rc=143. OK |
| X5 ancien `web/` non vide avec sous-dossiers en lecture seule, échec de vérification | Restauré à l'identique. OK |
| X6 proxy, `~/.curlrc` (`proxy`, `insecure`) et `BASH_ENV` hostiles | Déploiement rc=0. Le `BASH_ENV` exporté **est exécuté** par le script (INFO-2). |
| X7 `ss` sans process, autre port, autre ligne, IPv6 | Refus (2) avant l'arrêt, même PID. L'IPv6 est accepté. |
| X8 `systemctl stop` échoue, service reste actif | rc=1, rien touché, message `could not be stopped`. OK |
| X9 déploiement réussi, puis lecteur de sortie parti | **Déploiement complet (nouveau binaire, `web/` présent, service actif) mais rc=1** (LOW-2). |
| X11 outil requis absent (`runuser`) | Sortie 2, service intact. OK |
| X12 `web_dir` existant sous forme de simple fichier | Sortie 2, `unexpected web path`. OK |

## 5. Nouveaux constats

**LOW-1 : le contrôle « URL vs port écouté » ne vérifie que le port (lignes 92-94).**
- Une URL `https://<autre hôte LAN>:8443` passe le contrôle `ss` tant que `chill-server` écoute localement sur 8443. La sonde TLS stricte (SAN et certificat) la bloque ensuite sauf si l'autre hôte présente le même certificat.
- Scénario concret : cible mal saisie vers une machine clone, sonde et vérifications finales faites sur le mauvais hôte.
- Correctif proposé : extraire l'hôte de l'URL et vérifier qu'il figure dans l'adresse locale de la ligne `ss` (`*:PORT`, `0.0.0.0:PORT` ou `HOST:PORT`). Sinon corriger le commentaire et le plan.
- Impact réel sur le Pi (une seule IP cible) : faible.

**LOW-2 : code de sortie 1 après un déploiement réussi quand stdout est fermé (ligne 218, X9).**
- `trap '' PIPE` rend le `printf` final en EPIPE ; avec `set -e` (`completed=1` et `trap - EXIT` déjà posés), le script sort en 1 sans récupération. L'opérateur voit un échec alors que l'installation est complète.
- Correctif proposé : `printf … || true`, ou écrire le résumé dans un fichier root privé.

**LOW-3 : la restauration ignore un binaire dans un état tiers (lignes 112-127).**
- Si le hash courant n'est ni l'ancien ni le nouveau (cas théorique : corruption après le `mv`), `restored` reste à 1. Le `web/` est restauré, mais le binaire reste l'inconnu, sans message.
- Hérité de V6 ; improbable car le `mv -f` est atomique et suivi d'un contrôle de hash.
- Correctif proposé : message explicite et `restored=0` si le hash courant n'est ni l'ancien ni le nouveau.

**INFO-1 : `is-active` avant `systemctl stop` dans le handler (ligne 110).** Si le service est en `activating (auto-restart)`, `is-active` renvoie non nul, donc aucun `stop` n'est émis avant le remplacement du binaire. Aucun risque observé : l'écriture est atomique, et le `start` final remet l'ancien binaire. Un `systemctl stop` inconditionnel serait plus propre. Dépend de l'unité réelle (SUPPOSÉ).

**INFO-2 : `BASH_ENV` ou `SHELLOPTS` exportés sont honorés.** Seul l'opérateur root, via `sudo env`, peut les fournir ; `env_reset` les écarte par défaut. Aucune frontière franchie.

**INFO-3 : `cut`, `sort`, `tail`, `systemctl`, `chown`, `ln` ne sont pas dans la liste d'outils requis (ligne 72).** Ils sont dans coreutils, util-linux ou systemd, toujours présents. Sans effet. La liste utilisée (`seq`, `awk`, `install`, `runuser`, etc.) est présente sur Debian 13 minimale, sauf `openssl` et `curl`, qui sont refusés proprement en sortie 2 s'ils manquent. Le plan §5a ne les prévérifie que partiellement.

**Régressions V7 cherchées, non trouvées :**
- `set -E` et `trap ERR` n'ajoutent aucun bruit sur le chemin nominal (0 ligne `ERR`).
- Le handler retire `ERR` dès le début (`trap - EXIT ERR`), après `set +e`.
- Aucune interférence avec les `if`, `||` et `&&` : tous les contrôles gardés sortent en 2 sans ligne `ERR`.
- `trap '' PIPE` global : aucun pipeline du script n'a de lecteur qui quitte tôt (`find | sort | tail | cut`, `grep` sur herestring, `ss | grep`, `df | awk` lisent tout). Les enfants qui héritent de `SIG_IGN` (curl, `chill-server --backup` en Rust, `runuser`) s'en accommodent. Les 45 contrôles de la simulation et mes 19 scénarios le confirment.
- Nesting `if/else/fi` du handler : équilibré, tous les chemins vérifiés (service impossible à arrêter : X8 ; start impossible : X2 ; web_moved et web_swapped : C, D, X5).
- Aucun fichier de rollback n'est écrasé (`ln` sans `-f`).
- Config, unité et TLS : jamais écrits, contrôlés par hash avant et après.
- Aucun chemin n'a laissé le service arrêté sauf quand `start` échoue lui-même (X2), avec message explicite.

## 6. Cohérence du plan

- §2 : taille 17609 et hash `b61b9fd6…4872` corrects. Les hashes et tailles du tar et du binaire sont corrects.
- §5 : commandes conformes au script. L'URL passe en `CHILLCENTER_BASE_URL`, le script par stdin. Le préflight 5a ne vérifie que `curl ss`, pas `openssl` ni `runuser`, mais le script les contrôle lui-même avant tout arrêt.
- §7 : retour manuel cohérent avec le script (copie `chill-server-1eb24e97bcdf`, `web-<horodatage>`).
- **L1-doc** : la ligne 115 du plan affiche encore « 34 PASS, 0 FAIL (A à H) », alors que la simulation étendue est à 45 PASS (ligne 134). Vérifiée à 45 PASS.
- Le plan annonce lui-même que la revue n°3 est « À FAIRE ». Cette revue la couvre, à condition de mettre le plan à jour après coup.

## 7. Bancs d'essai : prouvent-ils ce qu'ils annoncent ?

- **I** échoue pour la bonne raison : mutation vérifiée, la version sans `trap '' PIPE` donne rc=141.
- **J** échoue pour la bonne raison : dossier des sauvegardes en lecture seule, et la sortie nomme la commande.
- **N** est correct : le service arrêté est refusé sans être démarré.
- **Réserve (LOW) :** le faux `systemctl` lance le service avec `env --default-signal=HUP,INT,TERM,PIPE`, ce qui reproduit l'indépendance du vrai systemd. Mais la simulation ne couvre ni `sudo` avec `use_pty`, ni le vrai `systemctl`, ni une coupure SSH réelle. Mes scénarios X3 et X4 comblent partiellement ce vide pour les signaux de groupe.
- La copie testée ne diffère que par l'UID (2 lignes). La simulation se tient sur Ubuntu 24.04, pas Debian 13.

## 8. Risques résiduels et non vérifié

- Debian 13 réel, systemd réel (`Restart=`, `TimeoutStopSec`), sudoers avec `env`, `use_pty` : SUPPOSÉ.
- RAM de 905 Mio et durée d'interruption sur le Pi 3B+ : non mesurées. Elle est d'environ 3 à 30 s dans la VM.
- Le format réel de `ss -Hltnp` sur iproute2 de Debian 13 : bien reconnu sur Ubuntu 24.04 (`0.0.0.0:PORT ` et `[::]:PORT `) ; un format `*:PORT` serait aussi accepté par `*":PORT "*`.
- Le Rust et les artefacts sont inchangés depuis la revue n°2. Je n'ai pas refait la revue du Rust, conformément à la consigne. La provenance du binaire n'est toujours pas prouvée par reconstruction.
- S6 (archive relue plusieurs fois dans `/var/tmp`) reste un risque accepté.

Aucun constat bloquant : le script V7 peut être autorisé à l'exécution sur ces octets exacts, sous les conditions du §1 et sur feu vert explicite de l'utilisateur.
