# Revue de sécurité indépendante R4 — opérateur binaire 0.7.1

- **Candidat figé revu** : `aba640284cd1dcbb27bfb4d0c1bec2b9deef3c82`
- **Base R3** : `f48b351724cb9e62b03fa199599ecb4bb24638a0`
- **Correctif opérateur** : `16167979`
- **Preuves de régression** : `aba64028`
- **Reviewer indépendant** : `/root/discovery_security_review`
- **Date** : `2026-10-20`
- **Verdict sécurité** : **APPROVED**

Cette revue est volontairement limitée aux trois constats R3 sur l'opérateur
manuel root. Les sources runtime Mac/serveur/web sont identiques au candidat R2
déjà revu. Aucun Pi, service, NAS, configuration, certificat, secret ou état
cloud n'a été lu ou modifié.

## Constats R3

### SEC-R3-01 — HIGH — fermé

La maintenance n'est plus supprimée par le `finally` après un échec. Le chemin
nominal ne la libère qu'après journal `complete`; le chemin d'erreur délègue à
`recover()` et n'efface son token qu'après récupération réussie
(`scripts/pi-server-binary-operator.py:276-295`). Une récupération incomplète
conserve donc le flag et le journal non résolu. `old_health()` attend désormais
jusqu'à 60 secondes et exige simultanément le hash attendu, l'unité active et
la réponse HTTPS stricte avant toute libération (`:216-250`).

### SEC-R3-02 — MEDIUM — fermé

Un parent créé par l'opérateur est explicitement remis à `0755`, indépendamment
de l'umask. Créé ou préexistant, il doit être un répertoire non-symlink, possédé
par root et exactement `0755`; un parent `0777` est refusé
(`scripts/pi-server-binary-operator.py:118-124`). Le flag est publié par lien
dur atomique, fsync du parent, puis contrôlé root:`0600`. Sa suppression exige
le même inode, type, propriétaire, mode et nonce (`:125-144`).

### SEC-R3-03 — MEDIUM — fermé

Le stage est copié, chown root et chmod `0755`, puis le fichier et le répertoire
de rollback sont fsync avant la validation de hash et avant le stop
(`scripts/pi-server-binary-operator.py:267-282`). Les remplacements durables
fsync toujours leurs répertoires source et destination (`:175-179`).

## État consolidé des constats R2

- **SEC-R2-01 — fermé** : flag de maintenance autoritatif, génération fraîche
  liée au flag et double preuve `busy=false`, `active_mutations=0`,
  `quiescing=true` avant le stop.
- **SEC-R2-02 — fermé** : recovery valide avant action les chemins absolus, le
  rollback root `0700`, le journal root `0600`, le service, le binaire et les
  deux hashes liés à la CLI ; le nonce empêche de nettoyer une autre opération.
- **SEC-R2-03 — fermé** : journal durable avant staging, stage et métadonnées
  durables avant stop, renames durables et reçu d'état conservé après échec.
- **SEC-R2-04 — fermé** : le backup validé retourne son snapshot sémantique et
  doit être identique au baseline pré-stop avant le premier swap.

## Contrôles de périmètre

- L'opérateur exige root, HTTPS sans credentials/chemin/query, hashes SHA-256
  explicites, fichiers réguliers et candidat ELF AArch64 ; il ne télécharge rien.
- Le helper, l'unité systemd, la configuration, le web, les réglages et les
  contenus NAS ne sont pas installés ou modifiés par ce script.
- Les chemins de recovery restent liés au journal privé et aux arguments de
  l'opérateur avant tout `systemctl` ou remplacement privilégié.
- Le delta R2→R4 ne contient aucun fichier sous `apps/`, `crates/`, `web/` ou
  `ui/`, ni `Cargo.toml`, `Cargo.lock` ou `versions.json`.
- Aucun secret ou identifiant n'est ajouté aux logs ou au journal.

## Preuves

| Commande/inspection | Résultat | Limite |
| --- | --- | --- |
| `git rev-parse HEAD` | PASS, `aba640284cd1dcbb27bfb4d0c1bec2b9deef3c82` | SHA exact |
| `git diff --name-only fdb80b5 aba6402 -- crates apps web ui Cargo.toml Cargo.lock versions.json` | PASS, aucun runtime | Revue delta opérateur uniquement |
| `python3 -m unittest scripts/test_pi_server_binary_operator.py` | PASS, 21/21 | Simulation locale ; pas de root/systemd/Pi réel |
| `PYTHONPYCACHEPREFIX=/private/tmp/chillcenter-security-r4-pycache python3 -m py_compile scripts/pi-server-binary-operator.py scripts/test_pi_server_binary_operator.py` | PASS | Contrôle syntaxique |
| `git diff --check f48b351 aba6402` | PASS | Delta R4 |
| inspection des chemins succès, crash, rollback et permissions | PASS ; aucun constat ouvert | Exécution opérateur réelle non effectuée |

Les tests couvrent notamment l'umask `077`, le parent préexistant `0777`, le
nonce étranger, la publication/lecture/nettoyage réels du flag dans un répertoire
temporaire, la quiescence générationnelle fraîche, le chemin nominal complet et
le maintien du flag/journal après échec de rollback.

## Décision

Aucun constat **CRITICAL**, **HIGH**, **MEDIUM** ou **LOW** n'est ouvert dans ce
périmètre. Le candidat exact `aba640284cd1dcbb27bfb4d0c1bec2b9deef3c82` est
approuvé pour l'exécution opérateur 0.7.1 avec les chemins et hashes exacts
validés par le preflight. Le backup, le swap sur le Pi et le smoke test réel
restent des preuves d'exécution de release à recueillir ; ils ne sont pas
remplacés par cette approbation statique et locale.
