# Revue sécurité indépendante — BUG0128

- **Mode :** `DEV_STANDARD`, frontière opératoire root-only existante.
- **Auteur :** `/root`.
- **Reviewer :** `/root/bug0128_security_review`, distinct de l'auteur ; aucun
  octet candidat écrit ou corrigé par cette instance.
- **Base :** `0122c22d5b00030fc0cdcd08763b133985371be0`.
- **Candidat exact :** `a6f0fc0c0c98ee1d4a4ecfb2236177fa157a2415`, branche
  `codex/bug0128-durable-operator`.
- **Revue UTC :** `2026-10-21T13:34:49Z`.
- **Verdict :** **APPROVED**.

Cette approbation porte uniquement sur le diff exact base/candidat et sur les
octets testés ci-dessous. Elle n'autorise ni ne prétend une connexion Pi/NAS,
une publication, une mutation GitHub/main/tag/release, un déploiement serveur,
web ou Mac, ni une modification de service, donnée ou réglage du foyer.

## Frontière et scénarios adversariaux

| Contrôle | Conclusion et preuve dans le candidat |
| :--- | :--- |
| Autorité / isolation | L'autorité reste le même rôle administrateur Linux root : le lanceur refuse hors Linux/root (`pi-operator-job.py:226-229`) et l'opérateur exige root (`pi-v07-legacy-server-web-operator.sh:41-46`). Aucun nouveau compte, helper, timer, IAM, secret, endpoint ou rôle. Le produit demeure LAN familial sans tenant ni authentification, conformément au périmètre approuvé ; ce delta ne crée aucune route, session, cookie, CSRF, callback ou redirection. |
| Propriété, chemins et liens | Racines absolues canoniques, parents root non inscriptibles, répertoires privés `0700`, fichiers root `0600/0700`, fichiers réguliers et `st_nlink == 1` sont vérifiés avant lecture/exécution (`pi-operator-job.py:26-55,118-157,176-193`). UUID canonique et `mkdir` sans réutilisation empêchent traversal/replay/écrasement d'un job. Les deux locks shell refusent symlink, hardlink, mode ou propriétaire inattendu avant toute action service (`pi-v07-legacy-server-web-operator.sh:50-65`). |
| Lock et FD héritée | Le lanceur prend le verrou avant `QUEUED`, duplique l'open-file-description en FD9, puis la transmet explicitement au superviseur, à Bash et aux descendants (`pi-operator-job.py:132-173,189-203`). Le script réutilise FD9 seulement si device/inode correspondent au lock attendu, puis reprend `flock -n`; une FD9 sans rapport est remplacée (`pi-v07-legacy-server-web-operator.sh:61-65`). Le verrou séparé de backup interdit aussi la récupération du trap `EXIT` au-dessus d'un backup orphelin (`:72-76,217-224`). Aucun lock n'est supprimé. |
| Signaux, PID et résultat | HUP est détaché/ignoré au superviseur ; Bash route HUP/INT/TERM vers `EXIT`, SIGKILL laisse les descendants détenteurs du lock. L'identité `{boot_id,pid,start_tick}` empêche la réutilisation de PID (`pi-operator-job.py:81-90,176-223`). Un superviseur perdu devient `SUPERVISOR_LOST`, un départ non observé `START_NOT_OBSERVED`, toujours avec `exit_code:null`; aucun succès n'est inféré. |
| Durabilité et rollback | Les snapshots, requête et états sont créés sans clobber, fsyncés, remplacés atomiquement, puis le dossier du job est fsyncé. Le transcript est fsyncé avant le reçu terminal (`pi-operator-job.py:58-79,139-171,194-208`). Le journal de transition reste fsyncé avant arrêt, les hashes et builds ferment les états ambigus, et les arbres retenus/quarantaines portent un UUID de transaction unique (`pi-v07-legacy-server-web-operator.sh:66-68,72-135,195-233`). |
| Injection et confidentialité | Le sous-processus détaché reçoit seulement un `PATH` fixe et `LANG`; l'opérateur reçoit ensuite seulement ces valeurs et les `CC_*` capturés. Pas de `shell=True`, `eval`, `BASH_ENV`, `LD_PRELOAD`, `PYTHONPATH` ni IO SSH hérité (`pi-operator-job.py:147-163,197-203`). Les erreurs publiques sont génériques/redactées (`:247-253`). Requête, snapshots, état et transcript restent dans `0700/0600`, sont conservés et explicitement interdits de publication par le guide. |
| Provenance et dépendances | L'opérateur exécuté est un snapshot dont le SHA-256 doit égaler le pin fourni, puis est revérifié avant exécution (`pi-operator-job.py:123-131,143-149,181-188`). Aucun manifeste de dépendances/runtime n'est modifié : bibliothèque standard Python3 et utilitaires Linux existants seulement ; `flock` est vérifié avant tout arrêt. Les pins/signatures/hashes applicatifs historiques restent exigés. |
| Scope et absence d'exposition | Le diff n'ajoute ni écoute réseau ni téléchargement. Le seul SSH ajouté est un harness de test qui exige le hostname VM exact, vérifie `127.0.0.1`, refuse proxy/jump et désactive l'agent forwarding. L'opérateur réel reste un outil manuel local root. |

## Preuves reproductibles

| Outil / version | Commande ou action | Résultat | Portée / limites |
| :--- | :--- | :--- | :--- |
| Git 2.54.0 | `git rev-parse HEAD`; `git merge-base 0122c22… a6f0fc0…`; doubles `git diff --exit-code` des cinq scripts contre le candidat | `a6f0fc0…`; base exacte `0122c22…`; PASS | Lie les fichiers hôte au candidat ; seul `docs/storage/` était non suivi et hors scope. |
| Git 2.54.0 | `git diff --check 0122c22…a6f0fc0` et inspection ligne à ligne du diff | PASS | 12 fichiers, `1001` ajouts / `17` retraits ; revue centrée opérateur, launcher, tests et procédures. |
| SHA-256 hôte + VM | `shasum -a 256` puis `sha256sum` des cinq scripts | Identiques : launcher `96a90aeb…c9329c`, opérateur `b891f222…c659558`, tests `60d4afbe…7313b8`, `e01caccb…86392a`, `ca0fb445…cd092` | La métadonnée Git du worktree monté dans la VM était indisponible ; la concordance byte-for-byte et le checkout hôte propre sur ces chemins lient néanmoins l'exécution au SHA candidat. |
| Bash / Python hôte | `bash -n`; parse AST Python sous `python3 -I`; scan lexical secret/IP du delta ; recherche de manifestes de dépendances modifiés | PASS ; seuls loopback `127.0.0.1` et fixture `tmdb-token` correspondent | Syntaxe et heuristique, pas un scan d'entropie/historique. Aucun secret, clé privée, adresse foyer ou nouvelle dépendance trouvé. |
| Linux 6.8.0-117 aarch64, Python 3.12.3, Bash 5.2.21, util-linux flock 2.39.3 | `sudo -n python3 -B scripts/test-pi-v07-legacy-operator.py` puis `sudo -n python3 -B scripts/test-pi-operator-job.py` dans `colima-v07-security` | **20 PASS**, 0 échec (`10 + 10`) | Vrais processus, UID, locks, TERM/SIGKILL, backup orphelin et remplacement partiel ; systemd/TLS/payloads restent des fixtures locales jetables. |
| OpenSSH loopback, agent forwarding désactivé | `python3 -B scripts/test-pi-operator-ssh.py --ssh-config …/colima-v07-security/ssh.config` | `real_ssh_disconnect=PASS`, récupération concurrente `REFUSED`, reçu terminal `0`, job `8cd8377f-1d31-48f5-ad10-736cd4196416` | Vraie coupure du client SSH puis reconnexion à la seule VM locale `127.0.0.1`; aucun Pi, NAS ou service extérieur. |

## Findings et limites

Aucun finding Critical, High, Medium ou Low n'est ouvert sur ce candidat. Les
scénarios d'abus demandés — propriétaire/permissions, alias et hardlinks, FD9
réutilisée ou étrangère, cycle de vie des deux locks, TERM/SIGKILL, PID recyclé,
receipt durable, journal privé, environnement hostile, collision de chemins et
récupération concurrente — échouent fermés ou ont une preuve Linux réelle.

Limites explicites, non requalifiées en succès : panne électrique/corruption du
système de fichiers, stack systemd réelle complète et déploiement foyer restent
`NOT_RUN`. En particulier, la première création du dossier `operator-jobs` n'est
pas une preuve de résistance à une perte d'alimentation avant persistance de
son entrée dans le répertoire parent ; l'approbation couvre les interruptions
processus/transport testées, pas cette panne physique déjà exclue par BUG0128.
Un reçu `SUCCEEDED/0` ne remplace jamais la vérification indépendante de santé,
hashes, données et invariants de la cible.

## Porte pour une future opération manuelle

Utiliser uniquement les deux scripts dont les hashes exacts sont consignés
ci-dessus, staged dans un parent root `0700`, avec opérateur/launcher `0700`,
artefacts signés vérifiés et pins explicites. Une cible réelle exige une nouvelle
autorisation, un UUID choisi avant SSH, le suivi du même UUID jusqu'au reçu, le
refus de toute reprise tant que `operation_busy:true`, et une vérification
indépendante après quiescence. Ne jamais unlink les locks ni publier
`request.json`/`transcript.log`. Toute modification des octets source rend cette
approbation obsolète.

**FR :** le candidat exact est approuvé dans sa frontière root-only existante ;
les interruptions Linux et la vraie perte SSH locale sont PASS, sans contact du
foyer. **EN:** the exact candidate is approved within the existing root-only
manual boundary; real Linux interruption and local SSH-loss proofs pass, while
power-loss, real systemd and household deployment remain explicitly unproven.
