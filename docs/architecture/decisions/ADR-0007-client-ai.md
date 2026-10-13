# ADR-0007 — IA sur Mac et application serveur contrôlée

- Statut : `ACCEPTED` ; révision `CC-DOC-001` ; 2026-10-13. Validation : [G-02](../../gates/G-02-CC-DOC-001.md), proposition originale/hash conservés dans le manifeste historique.
- Propriétaire/auteur : `/root` ; reviewer : voir rapport indépendant.
- Baseline : `90c149b797b45cba8a56dca2b74eed06c4225060` ; commit Chill Center : `NOT_CREATED`.
- Supersedes/superseded_by : `NONE` ; exigences F-14–15, F-18.

## Contexte et options

IA incluse, petit bouton, dictée exclue. Cinéfolio utilise des CLI connectés par l'utilisateur. Le Pi ne doit pas porter cette charge ni exposer une exécution de commande sélectionnée par un profil. Source L-04 ; l'éligibilité et les paramètres actuels des fournisseurs devront être revérifiés avant implémentation.

| Option | Bénéfices | Risques/coûts | Réversibilité |
| --- | --- | --- | --- |
| CLI sur Mac, plan et validation serveur | Réutilise connexion existante, Pi léger | CLI nécessaire, métadonnées envoyées au fournisseur | Élevée via interface assistant |
| CLI sur serveur | Configuration commune | Secrets/exécution/charge sur Pi | Moyenne |
| Modèle local sur Pi | Pas de fournisseur externe | Ressources incompatibles avec priorité vidéo sur Pi 3 | Moyenne |

## Décision proposée

Exécution explicite depuis Mac, environnement isolé, limites de Cinéfolio ; seulement métadonnées pertinentes et état du profil actif, IDs temporaires. Prévisualisation avant confirmation. Le serveur valide de nouveau profil, UUID, types et révisions puis applique la transaction. Aucun shell/outil libre, chemin NAS, vidéo ou secret dans les prompts. Un plan d'un ancien profil devient périmé. Petit bouton accessible, aucune dictée.

## Conséquences et validation

Positif : serveur économe, données personnelles correctement ciblées. Coût : dépendance CLI/compte fournisseur et éventuelle consommation de l'abonnement existant ; aucune licence nouvelle décidée. Recherche locale reste disponible sans CLI. Les métadonnées quittent le foyer seulement lors d'une demande explicite. Tokens API et prompts ne sont pas loggés. V-10 et tests de plans invalides, injection de titre, limites et conflit prouvent les gardes ; tests live optionnels distincts des fixtures.

Migration : adapter assistant existant au snapshot personnel et séparer patches partagés/personnels. Rollback : désactiver intégration IA et garder le catalogue ; aucune vidéo modifiée. Changement de fournisseur ou de données envoyées exige revue de confidentialité et ADR si matériel.

Revue : [ADR-0007-review](reviews/ADR-0007-review.md). Gate utilisateur : [TRACEABILITY](../../TRACEABILITY.md), étape 2 `APPROVED`, preuve [G-02](../../gates/G-02-CC-DOC-001.md).
