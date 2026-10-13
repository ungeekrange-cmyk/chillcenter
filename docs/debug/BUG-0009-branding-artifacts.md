# BUG-0009 — Marques fantômes dans deux propositions de logo

2026-10-13 · VERIFIED · propriétaire /root. Lot statique de branding uniquement.

## Contexte et reproduction

Base : `135d78c029cde269434186e4a0b8432962739665`, branche de correction `codex/branding-refinements`. Galerie locale sur macOS ; aucun serveur, NAS, réseau ou lecteur impliqué. Ouvrir directement `icons/branding/summer-light.png` et `autumn-dark.png` avec un visualiseur d'images.

Attendu : espace négatif sans marque fantôme. Réel : trace irrégulière au centre du disque été clair, entre le texte et le dossier ; marque discrète dans le fond extérieur de l'automne sombre. Signalement utilisateur confirmé par inspection des fichiers sources avec view_image, pas seulement par la galerie.

## Hypothèse et cause

H1 : les marques sont dans les images raster, pas introduites par le CSS ou le cache du navigateur. Expérience : afficher les deux PNG directement. Résultat : marques visibles, hypothèse confirmée. Cause à la couche asset : artefacts présents dans les propositions générées. Le mécanisme interne du générateur n'est pas instrumenté ; aucune cause plus précise n'est affirmée.

## Correction et preuve

Retouches ciblées avec image_gen, conservant les couleurs, canapé et lettrage. Nouveaux PNG copiés dans les mêmes chemins ; provenance, prompts et hashes avant/après dans `icons/branding/manifest.json`, révision `branding-r2`.

- Été clair corrigé : SHA-256 `e2a04bdf5760c68017c416500b826f3cec4a2a67d920561bf556940218b6aa56`.
- Automne sombre corrigé : SHA-256 `108d3fcee103ab7f05c1f620808bf2e3e04d9f12487787af536d53e53a9135fd`.

Inspection directe des résultats : les marques signalées ne sont plus visibles. Dimensions 1254×1254 et canal alpha conservés ; hashes du défaut approuvé et des six autres variantes comparés à la base. Références galerie versionnées pour les six images retouchées. Pas de preuve de contraste quantitatif ou d'export final à petite taille ; aucun changement/runtime test applicatif dans ce lot.

Le même lot réalise la demande esthétique distincte Noël (sapin derrière le canapé et les lettres) et Halloween (citrouille sur l'assise), en clair/sombre. Le premier nouveau sapin clair trop bas est remplacé par une deuxième retouche, conservée dans la provenance. Ces changements ne sont pas présentés comme des corrections de bugs applicatifs.

## Retour arrière et traçabilité

Restaurer les six PNG concernés, le README, le manifeste de marque et la galerie depuis la base Git ci-dessus dans une branche isolée ; régénérer ensuite `MANIFEST.sha256`. Conserver ce journal historique. Aucun fichier utilisateur ou média supprimé. F-20/F-21 et US-032/033 restent inchangés ; assets seulement, activation automatique non raccordée. Aucun nouvel ADR ni dépendance runtime.
