# BUG0108 — Durcissement du premier adaptateur smartphone web

- Date : 2026-10-17. Statut : FIXED / VERIFIED_LOCAL.
- Environnement : candidat web non livré, Chrome isolé avec API synthétique.
  Aucun défaut de ce candidat n'a été déployé avant cette vérification.
- Portée : navigation mobile, grille virtuelle, cibles tactiles et présentation
  plein écran. Aucun moteur, tampon, serveur ou client Mac modifié.

Symptômes reproduits avant livraison : profil mal aligné, certaines actions de
collection accessibles seulement au survol, cibles inférieures à 44px,
colonnes calculées à partir du mauvais seuil, décor encore décalé de 205px et
navigation pouvant rester cachée après redimensionnement pendant la lecture.
La revue a aussi reproduit un plein écran accordé tardivement après fermeture.

Causes : réutilisation partielle des sélecteurs desktop, valeur héritée fixée
sur `.app-shell` plutôt que la racine, confusion largeur utile/viewport,
écoute d'un événement de visibilité jamais émis et nettoyage anticipé de la
promesse de plein écran sans traiter son acquisition tardive.

Corrections : adaptateurs exclusivement web ; cibles ≥44px ; calcul des
colonnes et hauteurs virtuelles cohérent (deux jusqu'à 360px, trois au-delà sur
téléphone), variable événementielle portée directement par `.app-shell`,
observation ciblée des classes du corps et génération invalidant les retours
tardifs. Le plein écran desktop ne déclenche pas le verrouillage mobile.

Preuves : `web/test/mobile-ui.test.js`, `web/test/mobile-playback.test.js`,
`scripts/test-web-mobile.cjs` sur le vrai build, avec données fictives. Entrée
et sortie plein écran réellement exercées dans Chromium ; refus de politique
et courses testés par dépendances simulées. Essais physiques iPhone/Android
restants ; pas de lecture NAS ni de fluidité matérielle déduite de ces tests.

Les faux échecs de fixture (tableau de collections au lieu de `{items:[]}`,
langue réinitialisée à chaque rechargement, référence à un nœud virtuel retiré)
ont été corrigés dans le test, pas par un changement du contrat applicatif.
