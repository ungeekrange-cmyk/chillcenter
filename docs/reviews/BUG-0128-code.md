# Revue de code indépendante — BUG0128

- **Mode :** `DEV_STANDARD`, revue exacte avant toute intégration.
- **Base :** `0122c22d5b00030fc0cdcd08763b133985371be0`.
- **Candidat :** `a6f0fc0c0c98ee1d4a4ecfb2236177fa157a2415`.
- **Branche :** `codex/bug0128-durable-operator`.
- **Reviewer :** `/root/bug0128_code_review`, distinct de l'auteur.
- **Verdict :** **APPROVED**.

FR : aucun constat correctif ouvert sur le delta exact. EN: no actionable code
finding remains on the exact candidate. Ce verdict couvre le lanceur durable,
les verrous de l'opérateur legacy, la récupération et leurs tests ; il ne vaut
ni certification sécurité, ni déploiement Pi/NAS, ni publication.

## Analyse du delta

- `scripts/pi-operator-job.py:118-173` prend le verrou avant l'acquittement,
  refuse la réutilisation d'un UUID, snapshotte les octets revus, détache la
  session et ne transmet que FD9 et un environnement reconstruit. Les échecs de
  lancement restent non terminaux et ne peuvent pas devenir un faux succès.
- `scripts/pi-operator-job.py:176-223` revérifie l'inode hérité, publie les états
  par remplacement atomique/fsync, flush le transcript avant le reçu terminal,
  et traite perte du superviseur, réutilisation de PID et départ non observé
  comme une incertitude avec `exit_code:null`.
- `scripts/pi-v07-legacy-server-web-operator.sh:50-75,217-226` fait partager le
  même verrou noyau à tout l'arbre de processus et ajoute un bail de sauvegarde :
  un déploiement, un appel direct, un trap `EXIT` ou une récupération concurrente
  refuse avec code 42 avant toute action service tant qu'un descendant vit.
- `scripts/pi-v07-legacy-server-web-operator.sh:72-135,195-233` conserve la
  récupération basée sur journal et état disque, vérifie l'ancienne paire avant
  redémarrage, et nomme copies/quarantaines avec un UUID de transaction. Le delta
  final `d14b342..a6f0fc0` ferme ainsi la collision à la seconde qui pouvait
  réutiliser une paire de rollback antérieure.
- Les pins d'artefacts, contrôles de manifeste/compatibilité, sauvegarde privée,
  TLS, double garde d'occupation et empreinte de persistance du legacy restent
  en place. Aucun runtime, dépendance, API, helper/timer, configuration, paquet
  ou version publique n'est ajouté ou modifié.

## Tests et qualité

- `scripts/test-pi-operator-job.py:131-304` couvre HUP/TERM/SIGKILL, superviseur
  perdu, sauvegarde orpheline IO fermée, UID réel différent, remplacement
  partiel, identité boot/start, liens/hardlinks, permissions et refus avant
  service. `scripts/test-pi-operator-ssh.py:16-44` borne la vraie coupure SSH à
  la VM loopback nommée.
- La revue tests indépendante exacte [BUG-0128-tests.md](BUG-0128-tests.md) est
  **APPROVED** : 20/20 tests Linux, aucune relance/skip, plus coupure/reconnexion
  SSH réelle, récupération concurrente refusée et reçu terminal 0.
- La revue sécurité indépendante exacte
  [BUG-0128-security.md](BUG-0128-security.md) est **APPROVED**. Elle reste le
  gate de sécurité distinct ; la présente revue n'en duplique pas la certification.
- Vérifications propres au reviewer : `git diff --check` PASS ; Bash `-n` PASS ;
  AST des quatre fichiers Python PASS ; suite legacy locale 10/10 PASS. Hashes
  observés : launcher `96a90aeb…c9329c`, opérateur `b891f222…c659558`.

## Réutilisation des preuves et limites

Les preuves historiques du legacy sont réutilisées seulement pour les gardes
restées inchangées (pins, sauvegarde, TLS, état persistant). Elles ne valident
pas l'orchestration de la base : l'incident BUG0128 demeure la preuve que sortie
SSH et fin distante étaient auparavant confondues. Les preuves Linux/SSH
ci-dessus sont donc nouvelles et attachées au candidat exact ; le test de
collision est également nouveau après `d14b342`.

Panne électrique, corruption/disk-full, stack systemd réelle, cible foyer et
vérification post-déploiement restent `NOT_RUN`. La conservation sans purge des
reçus est intentionnelle et doit rester surveillée opérationnellement. Toute
modification source rend ce verdict et les deux revues parallèles caducs.
