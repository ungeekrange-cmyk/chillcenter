# Revue d’intégration finale — remplacement serveur seul

- **Mode** : `INTEGRATION_REVIEW`
- **Reviewer** : `/root/server_only_integration_gate`
- **Coordinateur / intégrateur** : `/root` (instance distincte)
- **Parent revu** : `02b6b8e6c542c869759558a02e53fa61aaa80ce3` / `CHANGES_REQUESTED`
- **Candidat source gelé** : `f3fdac503c4b845571981c1fd1bc00bd0b13ec21` (arbre `f88ae4d26c07f72cb150b948e58fa0c08b904bfa`)
- **Référence contrat historique** : `aad34cec60974782df959cf44c668e470fc4c658`
- **Référence source/assets intégrés avant correctif** : `be57d9f5ca535ffc48d5beb42d18a0adbc2df884`
- **Verdict** : `APPROVED` pour l’intégration source ; artefact ARM64 et smoke post-déploiement restent des gates distinctes

## Conclusion

Les deux findings bloquants précédents sont corrigés. GET/PUT `series/{series}` interprète désormais tout segment comme titre historique par défaut, y compris un titre littéral égal à `tmdb:tv:<id>` ou `media:<uuid>`. Seul `selector=key` active la résolution opaque stricte. `selector=title` est explicitement équivalent au défaut, une valeur de sélecteur invalide répond400, et les autres query parameters continuent d’être ignorés. Cette extension facultative conserve donc la sémantique du web existant sans réintroduire les écritures croisées dans le nouveau chemin opaque.

Le delta `02b6b8e..f3fdac5` ne modifie aucune migration ni aucun helper legacy ; il borne la correction au choix explicite du mode dans le handler, au client natif, aux tests, à l’OpenAPI et à la décision ADR-0019. Le helper legacy déjà revu conserve le CAS global et ne modifie que le nom/métadonnées communes et la préférence de vitesse associée au titre ; TMDB IDs, chemin, saison, épisode et association provider restent hors de cette écriture.

## Contrat et compatibilité

- Les 70 chemins et 89 opérations de l’OpenAPI historique restent tous présents ; le candidat expose 76 chemins et 95 opérations. Les six opérations sont additives et aucune opération historique n’est supprimée.
- Le seul ajout de requête au chemin série est le paramètre optionnel `selector` (`title` par défaut, `key` en opt-in). Les corps GET/PUT historiques restent inchangés et les alias `/api/2026-10-14` et `/api/v1` partagent toujours le handler.
- Les tests de collision construisent réellement deux groupes 1001/2002 puis donnent au second un titre égal à la vraie clé du premier. Ils couvrent les deux préfixes, les deux alias, GET par défaut/`title`/`key`, PUT legacy et opaque, l’égalité complète du groupe non sélectionné, ainsi que les invariants TMDB/path. Le test legacy adjacent couvre en plus agrégation homonyme, profils, lecture sans révision, CAS stale409, saisons/épisodes inchangés et mutation opaque limitée à un groupe.
- Le client natif ajoute `selector=key` aux commandes Series/EditSeries portant une clé opaque ; son test couvre les deux formats de clé et un titre ordinaire. L’ancien bundle Mac `11aa529` ne possède pas cet ajout et reste donc un client partiel pour les nouvelles opérations par clé. Il ne faut pas annoncer sa compatibilité complète ; le nouveau bundle est requis pour ce parcours.
- Les sources/assets web et scripts de build web sont identiques entre `be57d9f` et `f3fdac5`. Le déploiement autorisé reste strictement **binaire seulement** : aucune construction, copie ni permutation de `/usr/lib/chillcenter/web`.
- ADR-0019 supersède précisément l’inférence publique par préfixe d’ADR-0015, sans relâcher l’identité stricte interne. La documentation API décrit le défaut legacy et l’opt-in opaque.

## Preuves

Preuves rejouées par ce reviewer sans Cargo, afin de ne pas concurrencer le build ARM64 du coordinateur :

| Commande | Exit | Résultat |
| --- | ---: | --- |
| `node scripts/validate-api-contract.mjs` | 0 | API `2026-10-14`, 76 chemins |
| comparaison OpenAPI scriptée `aad34ce..f3fdac5` | 0 | 70→76 chemins, 89→95 opérations, zéro suppression |
| `git diff --check 02b6b8e..f3fdac5` | 0 | PASS |
| `git diff --quiet be57d9f..f3fdac5 -- ui/connected web scripts/build-web.sh scripts/build-web.mjs` | 0 | aucun delta web/build-web |

Preuves exact-SHA communiquées par le coordinateur, valides comme preuves de l’intégrateur mais non présentées comme réexécution indépendante :

- suite Rust complète : **296 PASS** (`75 catalog + 40 client + 64 domain + 117 server`), sans échec signalé ;
- Clippy workspace/all-targets avec warnings refusés : **PASS** ;
- validation API 76 chemins : **PASS** ;
- tests serveur legacy ciblés : **7 PASS**, dont les collisions et alias ci-dessus ;
- revue code : `APPROVED` ; revue sécurité : `APPROVED_SCOPED` pour `f3fdac5` et l’opérateur `62081`.

Le build ARM64 était encore en cours au gel de ce rapport. Une réussite de compilation ne prouve pas l’artefact déployé : avant promotion, enregistrer SHA-256, architecture ELF et provenance exacte reliant le binaire à `f3fdac5`.

## Gate opératoire et smoke requis

1. Vérifier l’artefact ARM64 exact-SHA, la sauvegarde offline, le préflight de l’état Pi autorisé et le rollback vers l’ancien binaire. Échec de build/provenance bloque la promotion.
2. Remplacer uniquement le binaire. Comparer avant/après le manifeste ou les hashes de `/usr/lib/chillcenter/web`, ainsi que config, unité, SPKI, `http_listen` et redirections ; toute différence inattendue bloque et impose rollback.
3. Confirmer premier démarrage sans erreur migration/sidecar, service actif, nouveau PID, `NRestarts=0`, listeners 8443/8080 et redirections 443/80. Conserver `user_version=3` et le sidecar service-owned0600.
4. En TLS strict, vérifier `/api/2026-10-14/server`, `/api/v1/server`, l’OpenAPI 76 chemins, puis une lecture titre legacy sans `selector` sous les deux alias. Vérifier aussi une lecture de clé du catalogue avec `selector=key`; ne pas utiliser un lecteur, une vidéo, un scan NAS ou un fournisseur réel.
5. Charger les assets web déjà installés et vérifier profil, catalogue, fiche et ouverture d’une série par titre. La preuve décisive est que la requête titre-only réussit sans mise à jour d’asset. Les PUT legacy/opaque sont déjà couverts localement ; ne les rejouer sur le foyer que sur une fixture explicitement sûre et réversible.
6. Pour annoncer le parcours Mac opaque comme livré, installer/tester séparément le nouveau bundle qui envoie `selector=key`. Le remplacement serveur seul ne transforme pas `11aa529` en client pleinement compatible.

Sous ces gates, `f3fdac503c4b845571981c1fd1bc00bd0b13ec21` est approuvé pour la promotion serveur seule. Une disparition de route historique, un refus d’un titre prefixé sans sélecteur, une modification d’asset web, un échec de démarrage ou un smoke titre legacy en échec impose le rollback du binaire.
