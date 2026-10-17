# Réparation ciblée des deux tests QA et reconstruction Mac

17 octobre2026, auteur `/root`. Branche `codex/qa-player-contracts`, base
`c071b426f2bab9a8948dce0f266b560684312347`.
Demande utilisateur explicite : corriger les deux tests restants, reconstruire Mac,
ne toucher ni Pi ni client web.

## Périmètre

[BUG0106](../debug/BUG-0106-stale-player-qa-assertions.md) : deux assertions obsolètes,
pas un défaut nouveau du lecteur. Le seul fichier de code modifié est le test Node.
Pas de dépendance, protocole, Rust, UI d’exécution ou réglage modifié ; aucune
compaction/nettoyage, connexion Pi/NAS, modification du chantier web ou publication
GitHub. Anciennes preuves d’échec conservées dans les rapports historiques.

## Exécution

1. Reproduction sur la base :3tests,1PASS/2FAIL ; comparaison des compteurs et
   du garde natif existant confirme les deux causes.
2. Contrats remplacés à leur couche responsable, contre-tests et exécution VM
   des préfixes de readiness. Aucune vraie lecture générale rejouée.
3. `node --test ui/connected/player-qa.test.js` : **7PASS/0FAIL**.
4. `node --test --test-reporter=spec ui/connected/*.test.js ui/connected/tests/*.test.js` :
   **130PASS/0FAIL**, aucun ignoré/annulé.
5. Bundle Mac à reconstruire depuis le candidat figé ; signature et dépendances
   à contrôler avant remise. Cette ligne est le checkpoint avant packaging.

La modification est locale, réversible et limitée aux tests : pas de nouvel ADR.
Les anciennes validations humaines Tauri/NAS ne sont ni rejouées ni élargies.
