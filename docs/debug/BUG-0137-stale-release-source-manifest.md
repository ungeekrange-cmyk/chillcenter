# BUG-0137 — Manifeste de sources périmé avant publication

2026-11-07. Revue indépendante des artefacts0.7.5, sourceeaab291 : les248 sommes
externes des sources correspondantes passent, mais le MANIFEST.sha256 inclus
dans le git archive comporte151 sommes périmées sur506. Aucun paquet0.7.5 n'est
publié ; brouillons et candidats initiaux conservés, exclus de la publication.

Cause confirmée : manifeste racine non régénéré depuis les changements livrés.
Correction : régénération avec scripts/update-manifest.sh sur tous les chemins
suivis, nouveaux fichiers inclus, sauf le manifeste lui-même. Le packaging de
composant vérifie désormais ce manifeste avant toute signature. Régressions :
octets modifiés, entrée manquante, doublon ou chemin non suivi refusés. Aucune
modification de logique runtime ni de version de produit supplémentaire.

Les binaires, archives, sources correspondantes et signatures du candidat sont
reconstruits avec le nouveau commit exact ; aucune substitution silencieuse à
l'intérieur d'un git archive. Les anciens artefacts/brouillons restent non publics.
Retour arrière : anciens paquets publics inchangés, aucun Pi/NAS ou binaire installé.
