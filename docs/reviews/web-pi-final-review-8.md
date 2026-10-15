# Revue indépendante n° 8 — contrôle final des octets à déployer (15 octobre 2026)

Reviewer : agent distinct de l'auteur, lecture seule (aucune édition du dépôt, aucune connexion au Pi, injoignable à ce moment). Commit revu : `125ea2a`. Rapport reproduit tel que rendu ; seule la section « Traitement par l'auteur » finale est ajoutée.

---

REVUE N°8 (contrôle final). Tout est en français. Dépôt en lecture seule : `git status` est vide. J'ai tout exécuté dans la VM Colima (Ubuntu 24.04 aarch64, systemd 255, vrai binaire). Je n'ai pas touché au Pi. `colima stop` est fait.

## VERDICT : APPROVED
Le verdict porte sur ces octets exacts, dont j'ai vérifié chaque SHA-256 (tous identiques, HEAD = 125ea2a) :
- `scripts/pi-enable-http-operator.sh` 26a8df5832c83a2fbc659ddc8ab08ff41a55a59395e2e2fb43348ca82d938e57 : APPROVED.
- `scripts/pi-redirect-80-operator.sh` 9577ad88bdbe1c0b668db5d0344e19d91676a2de5a999f48bec410fb66f51dd7 : APPROVED.
- `scripts/pi-redirect-443-operator.sh` 170f41a787167c882973f45eef8d374fb530143b3aaa38cfb04041d1a585f821 : APPROVED. Le diff contre `905732d` (3e7f6d4f…) se limite à `failed() { set +e; trap - ERR; …`.
- `scripts/pi-update-web-operator.sh` dcf1866111ef756b53113adce323f90960f5edbf00835204da78097fd6528e29 : APPROVED, inchangé. Sa récupération survit à un stderr mort (finding 4).
- Binaire `chill-server` ffcc634923f637331cac25fec1e5283b062ef5e2943778bcab59460b6269b0f7, 14890920 octets : vérifié (hash et taille seulement).
- Archive `chillcenter-web.tar` 24839b3d81f21632818903c5c097471a98086748303ac304477f23e480d00df2 : vérifiée (hash seulement).

Aucun BLOCKER ni HIGH. Le défaut HIGH de la revue 7 est corrigé et prouvé. Rien de la même classe ne subsiste dans les chemins de récupération. Les réserves (MEDIUM/LOW) n'empêchent pas le déploiement. Elles ne dispensent pas du préflight en lecture seule ni du feu vert utilisateur : le Pi est injoignable depuis ~17 h.

## Findings

1. **[Résolu, preuve par mutants] `set +e` en tête de `revert_now` et de `failed()`.**
   - Enable : j'ai supprimé la ligne `set +e` (mutant `/tmp/r8repo-e`) et rejoué la simulation de l'auteur. Résultat : `47 PASS, 2 FAIL`, avec `FAIL E5d configuration d'origine restaurée malgré stderr cassé — attendu '3031…6c8a', obtenu '5441…0505'` et `FAIL E5d service — arrêté`. Sans le correctif, la config reste modifiée et le service arrêté. E5d détecte donc bien l'ancien bug.
   - Redirect-80 : mutant sans `set +e;` dans `failed()`. Résultat : `42 PASS, 3 FAIL` dont `FAIL R7b résidus — présents`. R8 et R9 échouent en cascade.
   - Redirect-443 : même mutant. Résultat : `43 PASS, 3 FAIL` dont `FAIL R7b résidus — présents` (R8/R9 en cascade).
   - Cela étend le contrôle de l'auteur, qui n'avait testé que le script enable.

2. **[Check 1] Scénarios que j'ai écrits moi-même sur le script enable, avec un candidat qui empêche le service de démarrer** (adresse 203.0.113.9, hors de l'hôte). Résultat : `25 PASS, 0 FAIL`.
   - (i) stderr ET stdout simultanément morts : `exec 2>/dev/full; exec >/dev/full` (i1) et `exec 2>&-; exec >&-` (i2). RC=1, config d'origine restaurée, service actif, HTTPS 200, aucun temporaire.
   - (ii) stdout et stderr en pipes dont le lecteur est mort, SIGPIPE ignoré (ii1), et stdout seul (ii2) : même résultat, RC=1 pour ii1.
   - (iii) HUP, INT et TERM envoyés avec `kill` depuis Python, avec SIGINT non ignoré (un `&` bash l'ignorerait). 12 cas : 3 signaux × {A : `sleep 4` avant le restart ; B : `ExecStartPre=sleep 5`, restart bloquant} × {stderr vivant, stderr `/dev/full`}. Tous : rc=1, config d'origine restaurée, service actif. Avec stderr vivant, "FAILURE" et "RECOVERY" s'affichent.
   - (iii-bis) Double HUP pendant la fenêtre risquée : rc=1, tout restauré. Les deux signaux sont fusionnés, car le piège attend la fin de `systemctl restart`.
   - (v) Fonction `systemctl` truquée qui fait échouer le 2e `restart`, pendant la récupération : config restaurée, RC=1 (`|| true` tient).
   - (iv) `--revert` avec stderr mort (iv1) : RC=0, config d'origine. Avec stdout+stderr morts (iv2) : RC=1 uniquement à cause du `printf` final sur stdout mort ; la config est bien restaurée. Avec fds fermés (iv3) : config restaurée.
   - Sans `set +e`, le seul risque de la classe était `echo … >&2` en tête de la fonction de récupération. Les autres commandes de récupération sont protégées (`|| true`, `rm -f`, `mv` sur un fichier préparé d'avance) ou ne dépendent pas de stdout/stderr. Après `set +e`, le `exit 1` final est toujours atteint. `[[ … ]] && echo …` retourne sans effet. Le `rm -f` du `cleanup` EXIT est inoffensif.
   - Disque plein : non simulé. La récupération est un `rename` atomique d'un fichier préparé avant (`.restore`, hash revérifié), qui n'a pas besoin d'espace. Les écritures qui pourraient échouer sont toutes avant le changement de config et laissent la config intacte. Conclusion par lecture du code, pas par expérience.

3. **[LOW] Un 2e signal pendant la récupération tue le script** (enable, `trap - ERR HUP INT TERM` en tête de `revert_now`). J'ai envoyé HUP à t=2,5 s, puis HUP à t=7,5 s pendant le restart de la récupération, avec ou sans stderr mort. Résultat : `script rc = -1` (mort par signal), message "RECOVERY" absent. La config est quand même restaurée (le `mv` atomique passe en premier) et le service est actif : `PASS iii-B-HUPx2-late` et `…-deadstderr`. Je n'ai pas vérifié l'état des temporaires `.final`/`.restore` après cette mort. Le script update ignore les signaux avec `trap '' HUP INT TERM`, ce qui est plus sûr. Recommandation facultative : remplacer par `trap '' HUP INT TERM` dans `revert_now`. Je ne bloque pas, car le risque résiduel est cosmétique.

4. **[Check update, sans modification] `recover_on_failure` du script update n'est pas concerné par le défaut.**
   - Lecture : elle commence par `local status=$?`, puis `set +e`, puis `trap '' HUP INT TERM`, puis `trap - EXIT ERR`. Aucun `echo >&2` ne précède `set +e`.
   - Test avec un harnais d'extraction (lignes 15–24 et 103–146 du script réel, `systemctl` simulé, binaire et rollback factices, échec forcé après `stop_attempted=1`) :

   | Cas | Script réel | Mutant sans `set +e` |
   | --- | --- | --- |
   | baseline | rc=1, binaire restauré (OLD), `systemctl start` appelé | rc=1, binaire restauré |
   | stderr `/dev/full` | idem | rc=1, binaire resté NEW, aucun appel `systemctl` |
   | stdout+stderr `/dev/full` | idem | idem (NEW, aucun appel) |
   | fds fermés | idem | idem (NEW, aucun appel) |
   | pipes sans lecteur | idem | idem (NEW, aucun appel) |
   | scénario I (lecteur parti après la 1re ligne) | restauré | restauré |

   Le harnais détecte donc le défaut. Le script réel y est insensible. Limite : ce n'est pas l'exécution de bout en bout (voir plus bas).

5. **[LOW] Enable : stdout mort après le point de non-retour.** Si stdout est mort après l'activation réussie, le `printf` final échoue. J'ai testé avec `exec >/dev/full` inséré après le piège : RC=1, config modifiée, service actif, HTTPS 200. L'état est correct mais l'opérateur voit un code d'échec sans le résumé. À vérifier à la main (`sha256sum` de la config, `ss`) avant de relancer. Sûr : l'`echo preconditions_ok` en amont avec stdout mort fait quitter avant tout changement.

6. **[LOW] Fenêtre pré-changement avec stderr mort.** Dans enable (backup ou `.final` installés avant le `trap revert_now`) et dans les deux redirects (entre l'installation des fichiers et `trap failed ERR`), le piège ERR initial fait `echo >&2` ; avec stderr mort le script sort sans nettoyer. Aucun changement de config ou de service n'a eu lieu. Il reste au pire un backup enable (le prochain run refuse avec "backup already exists") ou des fichiers de règles (`--remove` règle). Raisonné par lecture, non exécuté.

7. **[Check 2] Rejeu des simulations dans la VM (compteurs réels)** :
   - `pi-enable-http-simulation.sh` : **49 PASS, 0 FAIL**, rc=0.
   - `pi-redirect-http-simulation.sh` : **45 PASS, 0 FAIL**, rc=0.
   - `pi-redirect-simulation.sh` : **46 PASS, 0 FAIL**, rc=0.
   - `web-e2e-linux.sh` : **40 PASS, 0 FAIL**. Ce script s'exécute dans la VM. Mon premier essai sur le Mac a donné 0/40 (pas de `/etc/os-release`, binaire Linux inexécutable), invalide. J'ai relancé via `colima ssh` et obtenu le résultat ci-dessus. Les claims de l'auteur sont confirmés.

8. **[Check 4] Docs.**
   - PI-LIVE-STATE : la bannière indique que le Pi est injoignable depuis ~17 h le 15 octobre 2026 (ping 100 % de pertes, SSH tombé), qu'il ne faut rien déployer ni modifier avant un préflight en lecture seule réussi, et que l'état n'est pas vérifié depuis.
   - Runbook, tableau étapes 9–12 : les 5 hashes (binaire, archive, update, enable, redirect-80) correspondent aux miens, taille du binaire comprise. L'étape 8 pointe sur le hash 170f41a7… du redirect-443. L'ordre impératif est 9, 10, 11 (le redirect-80 exige l'écoute 8080). Les valeurs de l'étape 10 (`CONFIG_SHA256=ea24f2f0…` = celle de PI-LIVE-STATE, backup `server.json.ea24f2f0a98d`) et le retour arrière (`--revert` avec `PREVIOUS_CONFIG_SHA256`) sont cohérents.
   - [LOW] Les étapes 2, 4 et 5 contiennent toujours les anciens hashes (binaire `a7ab7b07…`, archive `894fbe95…`, `OLD=cef54cd0…`). L'étape 9 dit « étapes 2 à 6 avec… » et exige une substitution manuelle. Un copier-coller sans substitution échoue en sécurité : le hash du binaire installé ne correspond pas. Il serait plus net d'écrire les commandes `chk` et la commande complète de l'étape 5 avec les nouveaux hashes.
   - [LOW] Les étapes 10 et 11 n'ont pas de ligne `shasum … && echo conforme` ni de `--self-test` explicites comme l'étape 8. L'étape 11 n'a pas non plus le contrôle `ip -4 -o addr` sur l'adresse secondaire.
   - [LOW] L'étape 8 est présentée comme « à faire » alors que PI-LIVE-STATE la décrit comme déjà exécutée. Le hash 170f41… diffère de la version réellement déployée (3e7f6d4f…, revue 5).

## Ce que je n'ai pas pu vérifier
- Rien n'a été exécuté sur le Pi (Raspberry Pi OS réel, sudoers, versions de bash, systemd et ss différentes de la VM). Ni preflight réseau, ni ssh. Son état actuel est inconnu.
- Le script update n'a pas été exécuté de bout en bout avec dead-stderr. `pi-operator-simulation.sh` exige un vieux tarball de release absent ici. Je n'ai testé que `recover_on_failure` en harnais d'extraction, avec `systemctl` simulé.
- La simulation de disque plein et l'état exact des temporaires après la mort par 2e signal (finding 3) n'ont pas été testés.
- Les signaux ont été envoyés par `kill` depuis un processus Python local. Un vrai sshd qui perd la connexion, avec sudo, tty et groupe de processus, n'a pas été reproduit.
- Ce qui n'a pas été relu : le code Rust du serveur, le client web, et la revue 7 de l'auteur ligne par ligne (seul son tableau de traitement m'a servi de repère). Binaire et archive : hash seulement, contenu non inspecté.
- Les scripts s'appuient sur `systemctl`, `ss` et `nft` du Pi. Je n'ai testé que ceux d'Ubuntu 24.04, avec nft 1.0.x dans les simulations redirect, ce qui ne garantit rien pour le noyau du Pi.

---

## Traitement par l'auteur (après le rapport)

Verdict **APPROVED** sur les octets exacts listés ; aucun octet n'est modifié après ce rapport (les corrections ci-dessous ne touchent que le runbook, qui n'est pas un script exécuté).

| Constat | Traitement |
| --- | --- |
| 3 (2ᵉ signal pendant la récupération tue le script) | **accepté** : la configuration est déjà restaurée (le `mv` atomique passe en premier) et le service actif ; changer `trap - …` en `trap '' …` modifierait les octets approuvés pour un gain cosmétique |
| 5 (stdout mort après l'activation) | **documenté dans le runbook** : si le code de sortie est non nul sans résumé, contrôler le hash de la configuration et `ss` avant de relancer |
| 6 (stderr mort avant l'armement de la récupération) | **accepté** : aucun changement de configuration ni de service n'a eu lieu à ce stade ; `--remove` (redirections) ou suppression de la sauvegarde (activation) règlent le reliquat |
| 8 (runbook : anciens hashes aux étapes 2, 4, 5 ; étapes 10 et 11 sans contrôle de hash ni auto-test ; étape 8 « à faire ») | **corrigés** dans le runbook : hashes courants, ligne `shasum` et `--self-test` aux étapes 10 et 11, contrôle de l'adresse primaire à l'étape 11, mention que l'étape 8 a déjà été exécutée le 15 octobre avec les octets de la revue n° 5 |

Octets approuvés : `pi-enable-http-operator.sh` `26a8df58…8e57` ; `pi-redirect-80-operator.sh` `9577ad88…1dd7` ; `pi-redirect-443-operator.sh` `170f41a7…f821` ; `pi-update-web-operator.sh` `dcf18661…8e29` ; binaire `ffcc6349…b0f7` (14 890 920 o) ; archive `24839b3d…0df2`.
