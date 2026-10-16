# Sous-titres, grille et langues du client Mac

Date : 16 octobre 2026. Auteur : `/root`. Branche : `codex/playback-locale`.
Base : `0c49a8caa9de5d370edd74f4d1f647fe2fa1932a` (produit précédent f980355).

## Bundle de test livré

Source exacte **15c2d63c2bca5058456a16c8ac7d0320d440ad4c** ; bundle canonique
`artifacts/macos/Chill Center-test-15c2d63.app`, interne0.5.1/build6.
SHA256 exécutable : `d26d53034aa895a443fb8918a4e14e0294a1286f515859babbb678754a4a75b0`.
Reconstruction release verrouillée/hors ligne puis packaging : signature ad hoc
profonde PASS,48bibliothèquesARM64 relocalisées PASS, chargement réel du libmpv
embarqué API2.5 PASS. Publication par staging/rename exclusif sans remplacer un
ancien bundle, copie canonique de même hash/signature vérifiée. Branche conservée
dans le dépôt canonique sans changer son checkout ni celui du web, aucun push.
Le bundle intermédiaire8a084f7 reste privé au worktree, pas la version à utiliser.

Pas de lancement natif ni de données réelles modifiées par les tests. Quitter
l’ancienne application avant d’ouvrir ce nouveau bundle. Aucun déploiement Pi.

## Périmètre et acceptation

- BUG0104 : changer de sous-titre ne suspend/recharge pas la vidéo ; téléchargement
  externe détaché, dernière sélection prioritaire, tampon et garde de bail inchangés.
- BUG0105 : lecteur → fiche → fermeture reconstruit le layout visible des vignettes,
  sans publier une mesure nulle pendant le lecteur.
- Langue locale `auto/en/fr`, persistée atomiquement dans les réglages existants du Mac.
  Ancien fichier sans champ : `auto`. Automatique : français si langue principale
  du système/webview commence par `fr`, anglais sinon. Choix explicite prioritaire.
- Configuration → Général → Langue de l’interface. Le choix recharge l’interface
  puis revient dans Configuration, pas le service ou la base. Entrer dans Configuration
  ferme déjà le lecteur suivant le comportement existant. Un changement de langue
  système ne recharge jamais l’interface pendant une lecture.
- Catalogue de traductions sans dépendance, textes statiques initiaux et littéraux
  UI explicitement marqués, paramètres séparés. Titres, genres TMDB, synopsis, noms
  de profils, chemins, saisies et identifiants ne sont pas traduits automatiquement.
  Les diagnostics de bas niveau non répertoriés restent des diagnostics techniques.
- Commande Tauri locale `save_language`, aucune route ni contrat HTTP nouveau.
  API datée/v1, serveur Pi, configuration serveur, NAS, fichiers du client web et
  worktree web ne sont pas modifiés.

## Choix local et réversibilité

Réutilisation de `Settings` avec valeur par défaut et validation fermée des trois choix.
Un indice localStorage ne contient que cette préférence, pas des données de profil ;
Rust reste autoritaire et corrige l’indice au boot. Cela évite de construire les libellés
dans la mauvaise langue avant la réponse native. `CFBundleLocalizations` déclare en/fr.
Une langue supplémentaire nécessitera son catalogue et une variante autorisée.

Alternatives écartées : traduire tout le DOM, qui modifierait les métadonnées ; ajouter
une bibliothèque/framework pour deux langues ; stocker cette préférence sur le Pi,
qui imposerait inutilement le même choix aux autres appareils/web.
Décision mineure, locale et réversible : pas de frontière de service, migration SQL,
nouveau fournisseur ou dépendance ; pas de nouvel ADR obligatoire selon la politique.
Retour à l’ancien bundle possible sans migration ; il ignore le champ de langue et
peut le perdre s’il réenregistre ses réglages. Les autres données restent compatibles.

## Vérifications exécutées

- 52 tests client Rust et 25 desktop release PASS, un benchmark mémoire ignoré
  intentionnellement. Deux warnings dead_code natifs préexistants.
- Vrai test libmpv ciblé de changement de sous-titre PASS (fixture locale 4s,
  aucun Pi/NAS), voir BUG0104.
- 89 tests Node ciblés PASS : lectures, grille, collections, transitions, langues.
- Suite Node élargie : 124/126 PASS ; les deux assertions QA de visibilité/trace
  dans `player-qa.test.js` échouent également sur la base inchangée, conservées.
  Aucun parcours lecteur général rejoué pour masquer ces échecs.
- Chromium connecté synthétique : RED/GREEN du layout, système français,
  système anglais, autre langue → anglais, choix manuel persistant, retour à
  Configuration et métadonnées inchangées PASS.
- Chromium virtualisation catalogue et collections : cartes/affiches chevauchantes
  conservées, pas de fetch supplémentaire PASS.
- Clippy release client/player/desktop PASS avec `-D clippy::all -A dead_code` ;
  seule la catégorie de warnings natifs préexistants est autorisée.

## Reste à confirmer humainement

Deux parcours seulement : sous-titres sur un fichier réel (y compris téléchargement
lent), puis fermeture lecteur/fiche dans Tauri avec la collection réelle. Pas de
nouveau débit simultané, test complet NAS/Wi-Fi ou déploiement Pi revendiqué.

Le nettoyage préalable et sa mesure sont séparés dans
[STORAGE-CLEANUP-20261016](../performance/STORAGE-CLEANUP-20261016.md).
