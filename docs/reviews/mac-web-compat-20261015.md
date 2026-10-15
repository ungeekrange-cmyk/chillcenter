# Revue indépendante — compatibilité Mac/web

Reviewer `/root/review_mac_web_compat`, distinct des auteurs, sans modification des sources, sans accès au Pi. Compte rendu consigné par le coordinateur ; les verdicts restent ceux du reviewer.

## Première revue : CHANGES_REQUESTED

Candidat `1b251cd7da888f9ad856ef468885a2e2e46d9f25`, arbre `841fd47a5430250b6641cb29da9a9fbd78d150a3`, parents37a5b35/aad34ce.

P1 : `web/src/bridge.js` mémorisait la révision dès le GET, avant réussite du `loadPage` commun. Si une page échouait, l'interface conservait les anciennes vidéos mais annonçait « Catalogue à jour » ; un nouveau focus/vérification ne réessayait pas cette même révision. Correction requise : acquittement après application complète, preuve d'échec puis réessai.

Aucun autre défaut de correction/sécurité/compatibilité dans la portée examinée. Union des modules/routes et conservation HTTP8080/TLS8443/statique/configuration/API Mac confirmées par lecture des différences aux deux parents.

Le reviewer a rejoué indépendamment15 tests bridge, le validateur76 chemins et le test Rust du GET de révision : PASS. Les293 tests Rust/56Node/Clippy/build54 du coordinateur sont des preuves réutilisées, pas des réexécutions revendiquées du reviewer.

## Correction proposée

`d95d5eaed963dc73b1e57529598085b0367b8553` : ticket d'acquittement local au bridge, invalidé par mutation/contexte/nouveau contrôle ; `loadPage` indique explicitement réussite/échec et conserve la vue précédente en échec. L'interface n'acquitte qu'après application complète et ne présente plus un échec comme à jour. Pas de nouvelle commande envoyée au client Rust natif ni de route d'acquittement serveur.

Coordinateur :59 tests Node PASS, build web54 fichiers/53ae7149a7ff, contrat76 chemins PASS. Rust inchangé depuis la première revue.

## Nouvelle revue : APPROVED, portée bornée

Le même reviewer indépendant a examiné `d95d5eaed963dc73b1e57529598085b0367b8553`, arbre `0ffb0d4b2af4bfa016e86218b487e4b6be647fd8`, contre son parent1b251cd. Aucun défaut concret dans ce delta ; le P1 initial est résolu. Acquittement validé par contexte/ticket, invalidation sur mutation/déconnexion/changement de contexte, absence de nouvelle commande Rust natif confirmés.

Réexécution indépendante de `node --test web/test/bridge.test.js ui/connected/sync.dom.test.js ui/connected/catalog-loading.dom.test.js` :27/27 PASS ; `git diff --check 1b251cd..d95d5ea` PASS. Les preuves293Rust/Clippy/contrat76 restent applicables au code Rust/API inchangé ;59Node et build53ae7149a7ff/54 fichiers acceptés comme preuves fournies. L'approbation concerne la correction source, pas une livraison sur le Pi ni la parité fonctionnelle complète du navigateur.

## Limites conservées

Pas de déploiement/smoke du binaire intégré sur le Pi, ni essai graphique PWA/Mac, ni nouvelle lecture vidéo. Les travaux web non commités du worktree voisin sont préservés et hors candidat. La compatibilité réelle de la version active ffcc6349 avec Mac11aa demeure partielle ; voir [preuve](../implementation/MAC-WEB-COMPATIBILITY-20261015.md).
