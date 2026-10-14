# Export disque en tâche de fond

2026-10-14 · US-027/MIG-002 · complément du contrat ARCHIVE-API.

`POST /api/v1/admin/exports` répond désormais **202**, `{export_id,state:"building",bytes:0,entries:0}`. `GET /api/v1/admin/exports/{id}/status` retourne `building`, `ready`, `failed` ou `cancelled`, les tailles et un code d’erreur sûr. Le téléchargement ZIP reste `GET /api/v1/admin/exports/{id}`. `DELETE /api/v1/admin/exports/{id}/cancel` annule le travail ou retire l’export prêt, sans modifier la bibliothèque.

Un seul vrai worker disque par serveur, conservant son permis jusqu’à sa sortie ; six résultats maximum, expiration après quinze minutes. Les fichiers inconnus comptent dans le quota mais ne sont jamais supprimés. Snapshot SQLite par API Backup, connexion séparée en lecture seule, WAL inclus de façon cohérente ; aucune sérialisation sur l’acteur interactif. Le document parcourt les médias ligne par ligne depuis le snapshot, sans `Vec<Media>` de toute la bibliothèque. Deux passes sur les affiches immuables ne gardent qu’une image en mémoire. Données JSON 64 Mio, ZIP 1 Gio, staging 2 Gio, snapshot 256 Mio ; arrêt sur dépassement ou annulation. Publication seulement après flush/fsync ; cleanup RAII des fichiers incomplets.

Prévention du rollback silencieux : `Store::open` refuse un `user_version` supérieur à3 avant migration ou ouverture en écriture. La couche catalogue ne rétrograde plus le marqueur commun de schéma. Les snapshots d’export sont ouverts sans migration en lecture seule.

Preuves fixtures Mac arm64/Rust1.99 : suites catalogue65, domaine26, serveur49 PASS ; test snapshot/WAL avec modification de la source après copie, refus byte-for-byte d’un schéma99, export asynchrone ZIP→aperçu→plan→application/idempotence, quota et annulation avant écriture. Les essais physiques Pi/Wi-Fi et la restauration du foyer ne sont pas revendiqués. Revue indépendante de ce delta encore requise.
