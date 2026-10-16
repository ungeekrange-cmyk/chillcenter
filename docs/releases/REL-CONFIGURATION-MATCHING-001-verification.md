# Vérification indépendante — REL-CONFIGURATION-MATCHING-001

- **Environnement** : Pi familial de développement, livraison binaire serveur seul.
- **Artefact déployé** : source `b36c916f0ea24fe40fe90df61971fd3e8e6044a0`; binaire ARM64 SHA-256 `dbda76e377a0f8ce84262396e6c339e44474846434351f9b4ec5bb5db4f5f100`.
- **Déployeur** : `/root/configuration_deployer`.
- **Vérificateur indépendant** : `/root/configuration_release_verifier`.
- **Vérifié à** : `2026-10-16T21:16:08Z`.
- **Verdict** : **APPROVED_SCOPED**.

Cette vérification est DEV_FAST, centrée sur le binaire serveur et les régressions demandées. Aucun client web, réglage, certificat ou fichier NAS n'a été modifié par le vérificateur. L'unique écriture applicative est l'effet explicitement autorisé du test Avatar : son fichier final confirmé absent est passé à `available=false` avec une nouvelle révision.

| Contrôle | Attendu | Observé | Résultat |
| :--- | :--- | :--- | :--- |
| Binaire en service | L'exécutable installé et celui du processus sont l'artefact livré | Deux SHA-256 identiques à l'artefact; service actif, redémarrages `0` | PASS |
| Invariants hors binaire | Web, configuration, unité, certificat et montage inchangés | Empreintes comparées au pré-déploiement, toutes identiques | PASS |
| Sauvegarde / retour arrière | Sauvegarde exacte et ancien binaire disponibles | Sauvegarde privée `0700`, manifeste SHA-256 `f3deafece5f28763eeee4454bb8fae6d4561059aae6eda0c42cfad0b8ed4a654`, 1 905 entrées vérifiées, un `quick_check` SQLite valide; retour arrière présent | PASS |
| Compatibilité API et web | API datée et `v1`, HTTP et HTTPS strict; client web inchangé | API datée : HTTP/HTTPS; API `v1` : HTTP/HTTPS; web : HTTP/HTTPS — six sondes `200` | PASS |
| Fichier absent ciblé | Aucun bail; erreur explicite, puis état persistant ciblé | La fiche Avatar fournie renvoie `503 MEDIA_FILE_MISSING`, sans bail; la fiche suivante a `available=false`, révision `2` | PASS |
| Signal runtime | Démarrage stable sans prétention d'optimisation | `VmRSS` observé à 15 064 KiB après stabilisation; mesure ponctuelle seulement | OBSERVÉ |
| Journaux | Aucun avertissement post-déploiement inattendu | Aucune entrée journal de priorité avertissement ou supérieure depuis le déploiement | PASS |

Le premier essai d'opérateur a échoué avant remplacement, en phase de sauvegarde, et sa restauration a été vérifiée indépendamment avant le second essai. La seconde tentative a remplacé uniquement le binaire; aucun rollback n'est recommandé.

## Limites

La vérification ne remplace pas un essai humain Tauri ni une lecture réelle sur NAS. Elle n'a volontairement exécuté ni scan, ni fournisseur TMDB, ni transfert, ni progression, ni flux vidéo. Le test d'absence utilise uniquement la fiche et l'opération ciblée autorisées par l'utilisateur.
