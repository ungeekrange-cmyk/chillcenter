# Revue indépendante — rails de recommandations

16 octobre2026. Source exacte **fa40e080edae6dd9d36180e00db0969e4252829e**, base **8e4523aec4cb5469b2f499580d902fec100429a0**. Reviewer `/root/review_series_completion`, distinct des auteurs, lecture seule ; compte rendu transcrit par le coordinateur.

## Première revue —779e299

**CHANGES_REQUESTED**, P2 : cibles d'affiches du conseil encore connectées après navigation ; le callback ferme directement une boîte de dialogue sans passer par le retrait des affiches. Inspection complémentaire : le conseil appartient au panneau `advice-panel`, pas à la boîte d'organisation `ai-dialog`. Le défaut pertinent est le panneau masqué avec chargements encore inscrits, pas une boîte de conseil fermée. Aucun autre changement de portée autorisé par cette correction.

## Revue finale —fa40e08

**APPROVED_SCOPED**. Le changement de visibilité du panneau retire les observers, invalide leurs tokens et supprime leurs requêtes en attente. Au retour, seules les affiches non décodées sont réobservées ; sélection/ordre/défilement conservés, sans nouvel appel fournisseur. Le chemin carte→série→shell déclenche bien ce cycle. Le candidat779e299 défavorable est remplacé, pas rétrospectivement approuvé.

Reviewer : **8/8** tests Node ciblés PASS. Pas de Playwright disponible dans son environnement : test navigateur non exécuté indépendamment, sans installation de dépendance ni réseau. Le coordinateur a exécuté avec succès `scripts/test-recommendation-rails.cjs` sur le candidat final : réponse d'affiche retenue puis libérée après masquage, non appliquée, reprise au retour sans relancer le fournisseur.

Périmètre : regroupement Rust/identités/métadonnées, contexte IA autorisé et lecture seule, intégration des deux rails, cycle de vie DOM et tests. Aucun accès fournisseur/Pi/NAS ni lecture vidéo. Pas une validation humaine Tauri, une mesure de qualité d'un fournisseur réel ou une revue sécurité globale.
