# ADR-0002 — SQLite locale, profils et identité stable

- Statut : `ACCEPTED` ; révision `CC-DOC-001` ; 2026-10-13. Validation : [G-02](../../gates/G-02-CC-DOC-001.md), proposition originale/hash conservés dans le manifeste historique.
- Propriétaire/auteur : `/root` ; reviewer : voir rapport indépendant.
- Baseline : `90c149b797b45cba8a56dca2b74eed06c4225060` ; commit Chill Center : `NOT_CREATED`.
- Supersedes/superseded_by : `NONE` ; exigences F-05–06, F-14, F-16.

## Contexte et options

Cinéfolio stocke vu/note dans `Media` et utilise le chemin relatif comme ID. Plusieurs profils et renommages exigent de dissocier ces dimensions. SQLite WAL ne doit pas être hébergée sur NAS. Sources L-02/L-03 et S-05.

| Option | Bénéfices | Risques/coûts | Réversibilité |
| --- | --- | --- | --- |
| SQLite serveur, UUID et tables personnelles | Réutilisation, peu de ressources, transactions | Migrations et réassociation à construire | Moyenne, export portable |
| Une base par profil | Séparation physique | Catalogue dupliqué, conflits des sagas et scans | Moyenne |
| PostgreSQL | Concurrence avancée | Service supplémentaire disproportionné pour trois lecteurs | Élevée via API |

## Décision proposée

Base unique locale au serveur, accès via file dédiée, WAL et FK. Catalogue commun ; reprises/vus/notes/playlists/préférences reliés à `profile_id`. UUID stables séparés de la localisation. Écritures sensibles avec révision attendue. Mapping de racines explicite ; empreintes partielles pour candidats, preuve complète/identité fiable ou confirmation humaine avant réassociation.

## Conséquences et validation

Positif : une source de vérité et histoires conservées. Négatif : un déplacement non déclaré peut demander confirmation ; pas de hash intégral imposé sur tout le NAS. Une empreinte partielle ne prouve pas l'identité. Sécurité : isolation logique, sans confidentialité vis-à-vis d'un autre membre pouvant sélectionner le profil. Coût : disque local, empreintes bornées et suspendues pendant lecture. Observabilité : conflits/réassociations et délais DB, sans chemins sensibles.

V-07/V-08/V-10/V-12 : profils, fichiers déplacés, concurrence et restauration. Migration transactionnelle et snapshot cohérent avant changement ; exporter les données de Cinéfolio depuis copie sans modifier sa base. Retour via snapshot pré-migration et ancien binaire. Une modification du modèle invalide import, API personnelle et stories correspondantes.

Revue : [ADR-0002-review](reviews/ADR-0002-review.md). Gate utilisateur : [TRACEABILITY](../../TRACEABILITY.md), étape 2 `APPROVED`, preuve [G-02](../../gates/G-02-CC-DOC-001.md).
