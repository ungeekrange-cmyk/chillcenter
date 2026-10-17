# Réparation ciblée des deux tests QA et reconstruction Mac

17 octobre2026, auteur `/root`. Branche `codex/qa-player-contracts`, base
`c071b426f2bab9a8948dce0f266b560684312347`.
Demande utilisateur explicite : corriger les deux tests restants, reconstruire Mac,
ne toucher ni Pi ni client web.

## Remise finale

- Source exacte : **095631ad55d1502480c874016a24f3b79968ff28**.
- Bundle canonique neuf : `artifacts/macos/Chill Center-test-095631a.app`.
- Version interne0.5.1/build6 conservée ; choisir le nom de bundle exact.
- SHA256exécutable : `2865690fef80cc01ec00e8c7475f34cc81b01c5c2017f811376f966f4f915ab1`.
- Packaging terminérc0 : buildreleaseverrouillé, signature ad hoc profonde PASS,
  48bibliothèquesARM64 relocalisées PASS, chargement du libmpv embarqué API2.5 PASS.
- Publication par staging/rename exclusif, copie canonique de même hash et signature
  vérifiée. Ancien15c2d63 toujoursSHA `d26d53034aa895a443fb8918a4e14e0294a1286f515859babbb678754a4a75b0`.
- Aucun lancement du bundle, accès ou déploiement Pi/NAS/web. Quitter l’ancienne
  application avant d’ouvrir le nouveau bundle ; rollback = ancien bundle conservé.

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
   à contrôler avant remise. Ce checkpoint antérieur est accompli dans la remise
   finale ci-dessus ; pas une nouvelle action en attente.

La modification est locale, réversible et limitée aux tests : pas de nouvel ADR.
Les anciennes validations humaines Tauri/NAS ne sont ni rejouées ni élargies.

## Observation d’environnement de compilation

La première tentative hors ligne avec le cache temporaire précédent échoue sur
une entrée `regex` manquante. Le cache Cargo utilisateur résout le graphe, mais il
lui manque aussi des archives (`alloc-no-stdlib`, entre autres). Reprise autorisée
avec ce cache et récupération réseau des seules versions verrouillées via le script
de packaging (`cargo build --release --locked`). Aucun contournement par changement
de version, aucun changement de `Cargo.lock`, aucun lien avec les deux tests QA.
