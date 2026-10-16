# Vérification indépendante — REL-LIBRARY-UX-SYNC-001

- **Lane** : `DEV_FAST`, vérification post-déploiement indépendante et revue d'exécution bornée
- **Source figée attendue** : `11aa529f2eced060156a3017456bf1d9634f8a56`
- **Serveur attendu** : SHA-256 `de98776dd5d0d20c72b7770591c78375186026e283b73ae4c66fe21b82398eb0`
- **Bundle Mac** : `artifacts/macos/Chill Center-11aa529.app`
- **Exécutable Mac attendu** : SHA-256 `d07bc889e077c1e2710caab01b66d9058941ee6ac11e5938e113667da31ec59d`
- **Déployeur / coordinateur** : `/root`
- **Vérificateur** : `/root/verify_catalog_release`, distinct du builder, du déployeur et des reviewers source
- **Contrôle UTC** : `2026-10-15T15:57:04Z`
- **Verdict global** : **BLOCKED**
- **Disposition Mac locale** : **APPROVED_SCOPED** pour l'identité et la signature du bundle uniquement

Le Pi ne sert pas l'artefact `11aa529` attendu. Le fichier installé et l'exécutable du processus actif portent tous deux un troisième hash non attribué, différent de l'ancien serveur connu comme du nouveau serveur attendu. La disponibilité HTTPS de cet artefact inconnu ne peut pas valider cette release. Toute nouvelle écriture sur le Pi est suspendue par le coordinateur en attendant l'identification de la mise à jour concurrente.

## Identité du candidat et bundle Mac

| Contrôle | Attendu | Observé | Résultat |
| :--- | :--- | :--- | :--- |
| Source locale | SHA exact `11aa529f…8a56` | `git rev-parse HEAD` exact ; aucun delta produit dans `crates`, `apps`, `ui`, `Cargo.toml` ou `Cargo.lock` | PASS |
| Travail préexistant | préserver les fichiers utilisateur | trois rapports sous `docs/reviews/` étaient déjà modifiés et n'ont pas été édités par ce vérificateur | PASS |
| Exécutable Mac | `d07bc889…ec59d` | hash exact, `15 526 688` octets, mode `0755` | PASS |
| Signature bundle | bundle cohérent sur disque | `codesign --verify --deep --strict --verbose=2` : exit `0`, exigences satisfaites | PASS borné |
| GUI/Tauri humain | hors périmètre | non exécuté | NOT_RUN |

Cette disposition n'est pas une preuve de lancement du bundle, d'interface native, de lecteur, de fournisseur, de NAS ou de correspondance source-vers-binaire indépendante.

## Incident de déploiement Pi

Le handoff du coordinateur indique qu'une première tentative opérateur s'est arrêtée avant le remplacement prévu, puis que la tentative suivante a refusé son préflight avant arrêt. Aucun nouveau backup ni rollback attribuable à cette livraison n'a été annoncé. Le contrôle indépendant suivant est strictement en lecture seule.

| Contrôle | Attendu pour `11aa529` | Observé sur `chillcenter.local` | Résultat |
| :--- | :--- | :--- | :--- |
| Service | artefact attendu actif | `active/running`, PID `9457`, `NRestarts=0`, démarré `2026-10-15T15:44:44Z` | disponibilité seulement |
| Fichier installé | `de98776d…98eb0` | `a7ab7b07633c916c358f1656aa8f154b207185d5ac306c374d2a7a52b0c7d527` | **FAIL** |
| Processus exécuté | même SHA attendu | `/proc/9457/exe` = `a7ab7b07…0c7d527`, identique au fichier inconnu | **FAIL** |
| Métadonnées fichier | artefact attribuable | root:root `0755`, `14 765 736` octets, mtime `2026-10-15T15:42:44Z` ; provenance inconnue | **BLOCKED** |
| Configuration | `ea24f2f0…83f9` inchangé | exact | PASS |
| Unité systemd | `76f7d349…72e2` inchangé | exact | PASS |
| SPKI TLS | `3018fb4d…740df` inchangé | exact | PASS |
| API datée | TLS strict, HTTP 200 | `200`, `393` octets, `45 ms` | PASS de disponibilité de l'artefact inconnu seulement |
| Alias `/api/v1` | TLS strict, HTTP 200 | `200`, `382` octets, `45 ms` | PASS de disponibilité de l'artefact inconnu seulement |
| Journaux warning+ | fenêtre post-bascule exacte | première commande pointait par erreur vers une fenêtre future ; résultat nul invalidé | NOT_RUN |
| Backup de cette bascule | chemin/manifest/hashes exacts | aucune sauvegarde attribuable transmise | NOT_AVAILABLE |
| Rollback de cette bascule | ancien binaire conservé et vérifié | aucun rollback attribuable transmis | NOT_AVAILABLE |

La configuration, l'unité et la SPKI inchangées réduisent le périmètre visible de l'incident, mais ne prouvent ni la provenance du binaire inconnu ni l'intégrité de ses données. Aucun hash source ou artefact local correspondant à `a7ab7b07…0c7d527` n'a été retrouvé dans la recherche privée bornée effectuée par le vérificateur.

## Revue d'exécution et qualité des assertions

En mode `EXECUTION_REVIEW`, le contrôle local suivant a été rejoué sur le SHA exact :

```text
node --test ui/connected/sync.dom.test.js ui/connected/catalog-loading.dom.test.js
```

- Environnement : macOS `27.0.1`, Node `v24.13.0`.
- Résultat : exit `0`, `9` PASS, `0` échec, `0` skip/todo/cancel, `107,9 ms`.
- Assertions significatives inspectées : conservation des cartes jusqu'au commit complet, vidage au changement de profil, rejet des réponses tardives de l'ancien profil ou de l'ancienne requête, pagination sans perte/doublon, nouvelle synchronisation après mutation et lancement détaché après connexion/approbation TLS.
- Limite : ces tests VM/DOM utilisent des appels simulés. Ils ne remplacent pas une limite réseau réelle, un Pi déployé ni un catalogue foyer.

Une sonde privée construite avec le `UiClient` exact était prête à vérifier le miroir chaud, la synchronisation détachée inchangée, la taille/latence de réponse et la réouverture. Elle n'a **pas** été exécutée : l'identité serveur a échoué avant le smoke. Ses contrôles sont donc `NOT_RUN`, jamais `PASS`.

## Verdict, rollback et limites

- **Rollback trigger de `11aa529`** : non applicable, car l'artefact attendu n'a pas été observé comme déployé.
- **Recommandation** : ne pas promouvoir le client comme validé face au Pi actuel et ne pas remplacer/arrêter le service tant que le propriétaire et la provenance du binaire `a7ab7b07…0c7d527` ne sont pas établis. Après résolution, déployer un artefact immuable attribué, fournir l'heure et le backup exacts, puis demander une nouvelle vérification indépendante.
- **État** : le bundle Mac local peut être publié uniquement avec la disposition `APPROVED_SCOPED` ci-dessus ; la partie Pi et le verdict de release restent **BLOCKED**.
- **Exclusions maintenues** : aucune mutation de déploiement, DB, profil, média, configuration, TLS, unité ou backup ; aucun appel fournisseur, lecteur, contenu NAS, vidéo, scan, GUI, restauration ou benchmark. Aucun identifiant, nom de profil, titre ou chemin de média réel n'a été affiché.

