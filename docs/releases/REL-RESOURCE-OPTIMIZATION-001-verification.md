# Vérification indépendante — REL-RESOURCE-OPTIMIZATION-001

- **Environnement** : Pi familial de développement et cache local du client fermé.
- **Source / artefact serveur** : `f9803557a452f59cbbe3cbf5b1c4bf1b48a9aa67`; SHA-256 ARM64 `6fecabc69258e372d71730a23451f5ff47565f60cbbc352552c12d8ac39424ea`.
- **Déployeur** : `/root/configuration_deployer`.
- **Vérificateur indépendant** : `/root/configuration_release_verifier`.
- **Vérifié à** : `2026-10-16T22:04:21Z`.
- **Verdict** : **APPROVED_SCOPED**.

Cette livraison DEV_FAST est bornée à la déduplication du cache d'illustrations local et aux optimisations mémoire/stockage sans changement fonctionnel. Le vérificateur n'a relancé ni application, ni migration, ni lecture vidéo, ni scan, ni fournisseur, ni écriture NAS.

| Contrôle | Attendu | Observé | Résultat |
| :--- | :--- | :--- | :--- |
| Cache fermé, intégrité existante | Les fichiers existants ne changent pas; seules des références blob peuvent être ajoutées | Ledger indépendant : 3 930 fichiers existants vérifiés, 600 blobs ajoutés; contenu, taille et mode préservés; état non-image également inchangé | PASS |
| Réduction locale | Réduire l'allocation sans réduire les références d'images | Allocation unique `198 754 304` → `49 860 608` octets, économie `148 893 696` octets; second ledger identique à l'état après migration | PASS |
| Binaire en service | L'exécutable installé et le processus correspondent à l'artefact | SHA-256 installé et processus conformes; service actif, redémarrages `0` | PASS |
| Invariants hors binaire | Web, configuration, unité, certificat, montage et bibliothèque inchangés | Empreintes identiques au pré-déploiement; réglage de bibliothèque toujours absent, comme avant | PASS |
| Sauvegarde / retour arrière | Sauvegarde exacte et ancien binaire disponibles | Sauvegarde privée `0700`, manifeste SHA-256 `985849e9f9d7efc4a28039e689427d76e666cff0ce0ec1934b457e202e786dd0`, 1 924 entrées vérifiées, un `quick_check` SQLite valide; rollback présent et conforme | PASS |
| Compatibilité API et web | APIs datée et `v1`, HTTP et HTTPS strict; web inchangé | APIs datée et `v1` : HTTP/HTTPS; web : HTTP/HTTPS — six sondes `200` | PASS |
| Journaux | Aucun avertissement post-déploiement inattendu | Aucune entrée journal de priorité avertissement ou supérieure | PASS |
| Signal runtime | Échantillon de santé, sans attribuer un gain à la livraison | `VmRSS` observé à 14 872 KiB après stabilisation; mesure ponctuelle seulement | OBSERVÉ |

Le rollback n'est pas recommandé. Les preuves privées de l'opérateur et les chemins du foyer ne sont pas publiés dans ce rapport. L'allocation est calculée avec `st_blocks`, une seule fois par couple `(dev,inode)` : elle mesure les blocs de fichiers du cache, pas nécessairement l'augmentation exacte d'espace libre du volume APFS, qui dépend aussi des clones et snapshots.

## Limites

Cette vérification ne remplace pas une validation humaine de l'interface Tauri, une lecture sur NAS ou un benchmark reproductible mémoire. Elle ne formule donc pas de gain RAM serveur à partir de l'échantillon RSS unique.
