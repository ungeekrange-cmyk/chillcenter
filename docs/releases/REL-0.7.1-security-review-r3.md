# Revue de sécurité indépendante R3 — opérateur binaire 0.7.1

- **Candidat figé revu** : `f48b351724cb9e62b03fa199599ecb4bb24638a0`
- **Base R2** : `fdb80b502db45d10c5384104912f5f246678c32a`
- **Commits opérateur** : `61c1a494`, `ffe8567`, `6e68e9f`
- **Reviewer indépendant** : `/root/discovery_security_review`
- **Date** : `2026-10-20`
- **Verdict production** : **CHANGES_REQUESTED**

Revue volontairement limitée au delta opérateur/guides demandé. Les sources
runtime Mac/serveur/web sont identiques à R2 et leurs contrôles ne sont pas
réattribués à ce rapport. Aucun Pi, service, NAS, configuration, certificat,
secret ou état cloud n'a été lu ou modifié.

## Constats

### SEC-R3-01 — HIGH — la maintenance est supprimée même si le rollback échoue

Le bloc d'erreur appelle `recover(a)` puis le `finally` supprime toujours le flag
avec le token initial (`scripts/pi-server-binary-operator.py:281-284`). Dans
`recover()`, le binaire précédent est redémarré avant `old_health()`
(`:229-241`). Si ce contrôle de santé échoue, recovery remonte une exception avec
le journal encore `stopping`/`swapped`, mais le `finally` efface malgré tout le
flag. L'admission est donc rouverte sur un service actif dont le rollback n'est
pas prouvé, pendant une transition durable non résolue.

Le flag possédé doit rester en place après toute récupération incomplète. Ne le
supprimer qu'après succès complet du déploiement ou de `recover()` ; ce dernier
supprime déjà son token après `old_health()` réussi. Ajouter une régression où
`old_health()` échoue après restart : flag et journal doivent rester, sans
réouverture d'admission.

### SEC-R3-02 — MEDIUM — le répertoire du flag ne garantit pas son autorité d'unlink

`acquire_maintenance()` crée le parent avec `mkdir(mode=0o755)` mais l'umask root
peut le réduire à `0700`; pour un parent existant, les lignes 121-123 vérifient
propriétaire/type/symlink mais acceptent aussi un répertoire group/world-writable.
Or la suppression/remplacement du flag dépend des permissions du répertoire,
pas du mode `0600` du fichier.

Pour un parent créé par l'opérateur, imposer explicitement root:`0755` sans
dépendre de l'umask et fsync le parent `/run`. Pour un parent existant, accepter
uniquement root-owned, non-symlink et non inscriptible par groupe/autres (le mode
déployé attendu peut être vérifié exactement). Tester umask `077`, parent `0777`,
parent symlinké et parent sûr existant.

### SEC-R3-03 — MEDIUM — les métadonnées du stage ne sont pas fsync après chmod/chown

La ligne 265 fsync le stage avant `chown(0,0)` et `chmod(0755)`, sans nouveau
fsync du fichier ni du rollback directory après ces métadonnées. Le journal est
bien durable avant le stage et les renames finaux fsync leurs deux répertoires,
mais une coupure peut laisser la propriété/mode du candidat non durable au
moment du swap. Effectuer chown/chmod, puis fsync le fichier et le répertoire du
stage ; couvrir l'ordre par un test ciblé.

## État des constats R2

- **SEC-R2-01** : **partiellement corrigé, reste bloqué par SEC-R3-01/02**.
  L'opérateur crée désormais un flag, attend la preuve fraîche liée à sa
  génération et exige `busy=false`, `active_mutations=0`, côté API et pulse.
- **SEC-R2-02** : **corrigé dans le périmètre mainteneur root**. Recovery exige
  rollback root `0700`, journal root `0600`, chemins absolus/confinés, service,
  binaire et deux pins identiques à la CLI avant `systemctl`/replace. Le nonce
  empêche de nettoyer le flag d'une autre opération.
- **SEC-R2-03** : **corrigé sous réserve de SEC-R3-03**. Le journal préparé et son
  répertoire sont fsync avant la copie stage ; les renames fsync les deux parents.
- **SEC-R2-04** : **corrigé**. `validate_backup()` retourne le snapshot sémantique
  validé et le déploiement le compare au baseline pré-stop avant tout swap
  (`scripts/pi-server-binary-operator.py:172,197,275-277`).

## Contrôles positifs

- Le journal root `0600` contient service, binaire, anciens/nouveaux SHA-256,
  chemins confinés et nonce aléatoire ; recovery valide la liaison CLI avant
  toute action privilégiée.
- La publication du flag par hard-link est atomique, refuse un flag préexistant
  et la suppression vérifie inode, type, owner, mode et nonce.
- Une preuve de quiescence doit être récente, appartenir au mtime-generation du
  flag et être confirmée par le statut HTTPS ; le service bloque simultanément
  les nouvelles mutations et travaux de fond.
- Le binaire/certificat/config/clé/web/library/token et l'unité restent des
  invariants en mémoire ; l'opérateur ne télécharge rien et n'installe aucun
  helper, unité, réglage, web ou contenu NAS.
- Un crash en `prepared`/`quiescing` ne stoppe pas l'ancien service ; les états
  `stopping`/`swapped` restaurent seulement depuis les hashes épinglés et exigent
  la santé HTTPS avant de considérer le rollback fini.

## Preuves

| Commande | Résultat | Limite |
| --- | --- | --- |
| `git rev-parse HEAD` | PASS, `f48b351724cb9e62b03fa199599ecb4bb24638a0` | SHA exact |
| `git diff --name-only fdb80b5 f48b351 -- crates apps web ui Cargo.toml Cargo.lock versions.json` | PASS, aucun runtime | Guides/concepts/reports changent aussi |
| `python3 -m unittest scripts/test_pi_server_binary_operator.py` | PASS, 14/14 | Aucun test direct du vrai flag/quiescence ; n'attrape pas SEC-R3-01/02/03 |
| `git diff --check fdb80b5 f48b351` | ÉCHEC documentaire uniquement : blank line finale dans le rapport R1 | Sans incidence exécutable |
| inspection des états/crash paths et ordre fsync | SEC-R2-02/04 confirmés ; trois écarts ci-dessus | Pas d'exécution root/systemd/Pi |

Une nouvelle revue indépendante sur SHA immuable est requise après un patch
borné de SEC-R3-01/02/03. Aucun déploiement Pi/production n'est approuvé par R3.
