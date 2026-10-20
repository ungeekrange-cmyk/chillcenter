# Revue code indépendante R4 — opérateur serveur 0.7.1

- **Mode** : `DEV_STANDARD`, revue opérationnelle delta, non mutante
- **Base du delta** : `f48b351724cb9e62b03fa199599ecb4bb24638a0`
- **Candidat exact** : `aba640284cd1dcbb27bfb4d0c1bec2b9deef3c82`
- **Source runtime inchangée** : `fdb80b502db45d10c5384104912f5f246678c32a`
- **Reviewer** : `/root/discovery_code_review`, distinct des auteurs
- **Verdict** : **APPROVED**

Cette revue est limitée aux trois constats R3 et aux tests opérateur ajoutés.
`apps`, `crates`, `web` et `ui` sont byte-identiques à la source runtime R2 ;
aucun Cargo ni suite runtime générale n'a été répété. Aucun Pi, NAS, service,
réglage ou certificat n'a été lu ou modifié.

## Fermeture des constats R3

| Constat | État R4 | Preuve |
| --- | --- | --- |
| `CR3-F-001` — racine maintenance legacy dépendante de l'`umask` | **CLOSED** | `acquire_maintenance()` impose explicitement le parent root `0755` après création et refuse owner/mode/type/symlink divergents (`scripts/pi-server-binary-operator.py:118-135`). Le test réel sur fichiers temporaires passe sous `umask 077`, vérifie parent `0755`, flag `0600`, nonce/ownership et nettoyage (`scripts/test_pi_server_binary_operator.py:97-126`). |
| `CR3-F-002` — métadonnées stage modifiées après le dernier `fsync` | **CLOSED** | La copie est désormais `chown`/`chmod 0755`, puis le fichier et le rollback directory sont synchronisés avant hash/swap (`scripts/pi-server-binary-operator.py:267-274`). Les renames continuent de synchroniser source et destination (`:175-179`). |
| `CR3-F-003` — documentation active encore v1 | **CLOSED** | `CONTRACTS` porte le titre daté ; `PLAYBACK-API`, `ARCHIVE-API`, `EXPORT-JOBS` et `CONFIGURATION` utilisent `/api/2026-10-14`. Les mentions `/api/v1` restantes dans les guides actifs décrivent explicitement son retrait ; les preuves historiques restent volontairement inchangées. |

Le correctif ferme aussi le constat de sécurité R3 associé : l'échec de
`recover()` ne déclenche plus un nettoyage inconditionnel du flag. Le déploiement
ne libère sa maintenance qu'après journal `complete`; la récupération ne la
libère qu'après santé HTTPS de l'ancien binaire (`scripts/pi-server-binary-operator.py:216-250`,
`:275-296`). Un échec de santé conserve donc flag possédé et journal non résolu.
`old_health()` dispose désormais d'un retry borné de 60 secondes, sans boucle
infinie ni relâchement anticipé (`:216-223`).

## Inspection des états et erreurs

- Le flag est publié par hard-link, avec inode/UID/mode/nonce vérifiés avant
  suppression ; un flag étranger n'est jamais effacé.
- La quiescence exige toujours le statut HTTPS et la pulse fichier fraîche de la
  même génération avec `busy=false` et `active_mutations=0` avant le stop.
- Le journal root/0600 reste lié à la cible binaire, au service et aux deux pins
  CLI avant toute action recovery.
- Les états `prepared`/`quiescing` ne stoppent pas le vieux service ; les états
  `stopping`/`swapped` ne rouvrent l'admission qu'après restauration et santé.
- Le chemin succès effectue uniquement le swap binaire attendu, marque
  `complete`, puis retire son propre flag. Un échec de rollback conserve la
  maintenance et le reçu pour intervention explicite.

Aucun nouveau constat `CRITICAL`, `HIGH`, `MEDIUM` ou `LOW` n'est ouvert dans ce
delta.

## Preuves exécutées sur le candidat exact

| Contrôle | Résultat |
| --- | --- |
| `python3 -m unittest scripts/test_pi_server_binary_operator.py` | **PASS 21/21**, aucun skip/retry |
| `PYTHONPYCACHEPREFIX=/private/tmp/chill-r4-pycache python3 -m py_compile ...` | **PASS** |
| `node scripts/test-bilingual-docs.mjs` | **PASS 23/23** |
| `git diff --check 50733ec..aba6402` | **PASS** |
| `git diff --quiet fdb80b5..aba6402 -- apps crates web ui` | **PASS**, runtime inchangé |

Les nouveaux tests utilisent de vrais fichiers temporaires, modes, hard-links,
renames et `fsync`, avec uniquement l'UID root et les appels système externes
simulés. Ils couvrent parent absent sous `umask 077`, parent writable refusé,
nonce étranger, récupération du flag depuis le journal, preuve de quiescence de
même génération, vrai `main()` succès/swap/cleanup et échec rollback conservant
flag et journal (`scripts/test_pi_server_binary_operator.py:19-39`, `:97-157`).

## Limites et disposition

L'exécution réelle root/Linux/Pi, `systemctl`, `runuser`, `ss`, backup SQLite,
persistance après coupure machine et pulse d'un serveur réellement déployé restent
des preuves de procédure/release `NOT_RUN`. Les essais mDNS LAN 0.7.1, permission
Local Network et GUI macOS restent également `NOT_RUN`; cette revue ne les
requalifie pas.

Le SHA `aba640284cd1dcbb27bfb4d0c1bec2b9deef3c82` est **APPROVED** pour rejoindre
les autres revues exactes et la préparation de l'opération autorisée. Cette
approbation source n'est ni un déploiement Pi ni une vérification post-déploiement.
