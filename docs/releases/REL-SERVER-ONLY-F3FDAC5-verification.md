# Vérification indépendante — REL-SERVER-ONLY-F3FDAC5

- **Mode / périmètre** : `FULL_ASSURANCE`, Raspberry Pi familial, remplacement **serveur uniquement** ; aucun déploiement web, scan, fournisseur, lecture vidéo ou lancement GUI.
- **Source immuable** : `f3fdac503c4b845571981c1fd1bc00bd0b13ec21`, arbre `f88ae4d26c07f72cb150b948e58fa0c08b904bfa`.
- **Carrier documentaire** : `0d0d9517fe7f336325ad3cb4d49807ac143de1ab` ; le diff depuis la source ajoute uniquement les cinq rapports/gates de cette livraison.
- **Artefact déployé** : `d77e9e1669287a6d10f279c17f33d7f776efe7e194acc01cd542bcea33511c90`.
- **Cible** : `admin@chillcenter.local`, unité `chillcenter`.
- **Déployeur** : `/root/deploy_server_only_f3fdac5`.
- **Vérificateur indépendant** : `/root/verify_server_only_f3fdac5` (distinct du constructeur, des reviewers et du déployeur).
- **Observé** : `2026-10-15T22:18:01Z` à `2026-10-15T22:20:30Z`.
- **Verdict** : **APPROVED_SCOPED** — état serveur seul déployé et vérifié ; les exclusions ci-dessus ne sont pas implicitement approuvées.

## Preuves de version, état et préservation

| Contrôle | Attendu | Observé | Résultat |
| :--- | :--- | :--- | :---: |
| Archive publiée | archive liée à `f3fdac5`, sans web | SHA archive `35310c439f54fc0b45cbd4342c9154ec7302d8a86148fc7a4975252beb2ccd6f` ; manifeste source/target/glibc/API conforme ; binaire embarqué `d77e…`; opérateur embarqué `62081…`; `included_web_assets=false` | PASS |
| Service / version | actif, nouveau binaire, aucun restart | PID `3117`, `active`, `NRestarts=0`; installé **et** `/proc/3117/exe` = `d77e9e…e194`; démarrage `2026-10-15T22:14:29Z` | PASS |
| Stabilité après smoke | même PID/hash, aucun restart | à `22:20:30Z` : PID `3117`, `NRestarts=0`, process `d77e9e…e194` | PASS |
| Listeners | serveur sur `8080/8443` | `chillcenter.local:8080` et `:8443`, tous deux détenus par PID `3117` | PASS |
| Web interdit en écriture | empreinte complète inchangée | contenu + inode/mode/uid/gid/taille/mtime : `07868e61cd75dc36d5bf62eefd258b7409b6ed264bde726cdcf626d45b57fe2b` | PASS |
| Configuration / unité | empreintes inchangées | config `d6167cb1624b6d5888473af76d5470102afedce25b2e29ddbaa032730cd0ceb1`; unité `76f7d34921ff344e344ba2bffa51263248ec6118dad9e15a39ef7078877d72e2` | PASS |
| TLS / NAS | SPKI et montage inchangés | SPKI `3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df`; montage `1935a8db9901c414782eaa05dc5ee81f6ec092a499755034e90e562f8dee0d37` | PASS |
| Migration | schéma maintenu | base live `PRAGMA user_version=3` | PASS |
| Observabilité | aucune alerte nouvelle | journal `chillcenter` depuis `22:14:57Z` : `0` warning-ou-plus, `0` error-ou-plus | PASS |

Lecture seule SSH exécutée avec le socket partagé et `sudo -n`; commande de vérification exit `0` en `11,7 s`, puis relecture de stabilité exit `0` en `2,6 s`. Aucun redémarrage, kill, restore, scan ou changement de configuration n'a été effectué par le vérificateur.

## Sauvegarde et retour arrière

La seule sauvegarde acceptée par cette vérification est `/var/lib/chillcenter/data/backups/5dd8df08-adcb-4d09-a6c1-2eb549c0c414`, privée `0700`, propriétaire service UID `997` :

- manifeste SHA-256 `3088c9c71d68a27889ce9d832a9b8477baf2ea04987a234a97ec78f1c743b935`, format `1`, schéma `3` ;
- `1 746` entrées, tailles et SHA-256 de chaque entrée relus avec succès ; agrégat indépendant `299d9f045904429bc6368765683a90a80a34231f83572c24b6b473f902abd2fe` ;
- ensemble de fichiers exactement égal au manifeste, aucun lien symbolique ;
- une base SQLite, `PRAGMA quick_check=ok` en lecture seule immuable.

Le premier chemin de sauvegarde rapporté pendant l'incident (`ba950…`) n'est ni utilisé ni présenté comme preuve. Le rollback privé `/root/chillcenter-rollback/chill-server-ffcc634923f6` existe en `0755 root:root`, SHA-256 `ffcc634923f637331cac25fec1e5283b062ef5e2943778bcab59460b6269b0f7`. Aucun déclencheur de rollback n'est atteint et aucune restauration de données n'est recommandée.

## Smoke réseau, API et compatibilité

Depuis le Mac, avec vérification TLS stricte via le certificat public épinglé :

- racine, `/api/2026-10-14/server` et `/api/v1/server` répondent `200` sur HTTPS public `443`, HTTPS direct `8443`, HTTP public `80` et HTTP direct `8080` selon les alias testés ;
- les quatre racines renvoient les mêmes octets, SHA-256 `4bf69bf403915de925f27db9f4db8432fcf2560ed56b4a0d6d9f11b147b7d09a` ;
- les réponses serveur partagent le même `server_id`, annoncent `supported_api_versions=["2026-10-14","v1"]`, et portent respectivement `api_version="2026-10-14"` et `api_version=1` ;
- OpenAPI datée et v1 sont identiques : version `2026-10-14`, `76` chemins, `95` opérations ; les `70` chemins et `89` opérations de la référence historique `aad34cec60974782df959cf44c668e470fc4c658` sont tous conservés, zéro suppression ; SHA déployé `4765951392165d255f148e420e477d5e07f056120a5b2f30d848208bed89fa50` ;
- garde origine/hôte : same-origin HTTPS `443` et HTTP `80/8080` = `200`; cross-site et Origin étrangère = `403`; Host inattendu sur `8443` = `403`.

Le script de smoke externe a terminé exit `0` en `4,6 s`. Il n'a envoyé que des GET de lecture.

## Smoke réel `UiClient` et revue d'exécution

Probe Rust compilé pour la source `f3fdac5`, exécutable SHA-256 `d73a6e089aa1c1257aaf0101349a5b660590164d89a636b5c471c0abdfc3cade`, répertoire neuf privé `/private/tmp/chillcenter-compat-readonly.YrFzIc/client-post.qa1SVh` :

| Parcours / oracle significatif | Observation | Résultat |
| :--- | :--- | :---: |
| Connexion TLS épinglée et cache froid | `PINNED_CONNECT_OK`, `offline=false`, `cold_ms=4151` | PASS |
| Catalogue et détails | `2 496` éléments ; détail + sagas + playlists lisibles | PASS |
| Synchronisation détachée initiale | `changed=true`, publication non rejetée, API `2026-10-14`, `4717 ms` | PASS |
| Série | révision présente, `53` épisodes | PASS |
| Réouverture chaude | `offline=false`, `228 ms` | PASS |
| Delta inchangé après cache chaud | deuxième exécution : `changed=false`, `discarded=false`, `21 ms` | PASS |
| Bornes d'effet | `household_mutations=false`, `playback=false`, `provider_calls=false` | PASS |

Les deux exécutions ont terminé exit `0` (`18,8 s`, puis `7,3 s`). Les assertions contrôlent des valeurs réelles (taille du catalogue, présence de révision, listes/détail, résultat et latence du delta, réouverture chaude) ; il ne s'agit pas d'un simple statut HTTP ou d'un mock. Aucun check critique ignoré, retry masquant un échec, profil créé, mutation foyer, appel fournisseur ou lecture vidéo.

Vérification auxiliaire du nouveau bundle Mac, sans lancement GUI : exécutable `arm64` SHA-256 `d31bfff6e722c086b57a135fcd5744c1f8d0fe6383bb821a32e5cad019a7327c`, `48` dylibs `arm64`, signature ad hoc profonde valide. Cette preuve de packaging n'étend pas le présent verdict au lecteur/GUI. L'ancien bundle `11aa529` reste explicitement partiel pour les opérations opaques sans `selector=key`.

## Historique opératoire observé et conclusion

Le canal du déployeur a perdu les métadonnées de session/stdout : le code retour final de l'opérateur n'est donc pas affirmé. Pendant cette perte de visibilité, un démarrage manuel prématuré de l'ancien service a rencontré le `DataLock` tenu par la sauvegarde et produit une boucle de restart transitoire ; elle a été arrêtée. L'opérateur approuvé demeurait en cours et a ensuite publié/démarré le nouveau binaire. Le verdict repose exclusivement sur l'état final relu indépendamment — nouveau binaire installé et exécuté, PID stable, zéro restart courant, health/API réels, préservation exacte et sauvegarde `5dd8…` entièrement validée — et non sur le stdout ou le RC manquant.

**APPROVED_SCOPED** : la gate post-déploiement du remplacement serveur seul peut passer à `VERIFIED`. Aucun rollback n'est recommandé. Les exclusions web, GUI/lecteur, fournisseur, scan NAS et benchmark restent hors périmètre ; toute modification ultérieure du binaire, du web, de la configuration, du TLS, de l'unité ou du montage rend cette preuve obsolète.
