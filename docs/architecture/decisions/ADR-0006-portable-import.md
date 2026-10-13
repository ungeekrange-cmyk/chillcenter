# ADR-0006 — Export portable et import transactionnel

- Statut : `ACCEPTED` ; révision `CC-DOC-001` ; 2026-10-13. Validation : [G-02](../../gates/G-02-CC-DOC-001.md), proposition originale/hash conservés dans le manifeste historique.
- Propriétaire/auteur : `/root` ; reviewer : voir rapport indépendant.
- Baseline : `90c149b797b45cba8a56dca2b74eed06c4225060` ; commit Chill Center : `NOT_CREATED`.
- Supersedes/superseded_by : `NONE` ; exigences F-16–18.

## Contexte et options

L'utilisateur demande de récupérer les informations de Cinéfolio dans un format choisi et de garder l'import dans Administration. Une copie de DB interne n'est pas un contrat portable ; WAL et données Mac rendent une copie brute fragile. Sources L-02, S-05 et F-17.

| Option | Bénéfices | Risques/coûts | Réversibilité |
| --- | --- | --- | --- |
| ZIP JSON versionné + affiches | Lisible, profils explicites, export portable | Validation archive et conflits à gérer | Élevée |
| Copier directement le SQLite | Simple sur schéma identique | WAL, secrets, couplage au schéma et chemins Mac | Faible |
| Nouveau scan seul | Aucune migration | Perd notes, corrections, playlists | Élevée mais ne répond pas au besoin |

## Décision proposée

Archive avec manifest/version/export_id, données et affiches contrôlées ; pas de vidéo, secret ou bail. Export Cinéfolio en lecture depuis snapshot cohérent. Import avec aperçu, mapping racines/profils, contrôle des tailles et conflits ; application atomique, import répété identifié. Restauration remplaçante distincte de l'import ordinaire, avec sauvegarde et lectures arrêtées.

## Conséquences et validation

Positif : continuité et audit lisible. Coût : staging disque et validations ; aucune archive complète en RAM sur Pi. Les fichiers d'export portent des données personnelles : choix explicite du fichier par l'administrateur. Pas de migration automatique des credentials. Refuser traversal, symlinks, bombes ZIP, collisions et versions inconnues ; originaux intacts. Observabilité : compteurs et conflits, sans contenu personnel dans les logs.

V-07/V-12 : aller-retour, import répété/invalide, interruption et rollback. Sauvegarder destination avant remplacement ; retour via sauvegarde complète DB/affiches correspondante. Les secrets opérationnels se sauvegardent séparément. Changer le format requiert nouvelle version et migration explicite, sans effacer l'historique d'import.

Revue : [ADR-0006-review](reviews/ADR-0006-review.md). Gate utilisateur : [TRACEABILITY](../../TRACEABILITY.md), étape 2 `APPROVED`, preuve [G-02](../../gates/G-02-CC-DOC-001.md).
