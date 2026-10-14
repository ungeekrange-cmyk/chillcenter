# BUG-0053 — premier rafraîchissement assimilé à un changement de profil

2026-10-14, candidat local `codex/ui-panels-player`, base0d0da0e. Cas d'initialisation révélé par la fixture à réponse rapide, pas une panne serveur ni un échec de certificat.

H1 : le hook du catalogue peut être enregistré après le retour du boot ; garder sa baseline ànull fait considérer son premier appel ultérieur comme un changement de profil, même si le contexte est identique. Reproduction avec le vrai module dans `tests/collection-pages.test.js` : ouvrir les sagas dans un contexte déjà connecté puis appeler le hook sans changer de profil. RED : sectioncatalog à la place decollections.

Correction : initialiser la baseline depuis l'état client déjà connu lors de l'enregistrement du module. Même scénario GREEN ; un vrai changement de profil remet toujours au catalogue, ferme les dialogues et invalide les lectures anciennes. Le contexte inclut serveur et profil. Pas de contournement de l'isolation ni d'écriture serveur. La correction est distincte de BUG0052 et ne prétend pas expliquer seule les deux manifestations d'Administration.

Résultats du lot/SHA : REL-MAC-UI-PANELS-001. Retour possible à l'ancien bundle, aucune migration.
