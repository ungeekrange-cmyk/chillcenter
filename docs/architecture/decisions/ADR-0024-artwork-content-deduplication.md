# ADR0024 — déduplication physique des affiches du client Mac

- Statut : PROPOSED, détail réversible de l'ADR0013 ; aucune suppression de références ou nouvelle rétention.
- Auteur/propriétaire : `/root` ; reviewer indépendant : en attente.
- Date : 2026-10-16 ; branche `codex/resource-optimizations`, base228c4c6.
- Supersedes : NONE ; Superseded by : NONE. Exigences F-01/F-14/F-16, US021, NFR ressources.

## Contexte et options

L'audit a confirmé des copies d'affiches identiques dans le cache privé. L'utilisateur demande leur optimisation avec mesure, sans changement fonctionnel ni modification du client web. Les chemins nommés existants portent l'identité/révision du média et doivent rester lisibles hors ligne par les anciens bundles.

| Option | Bénéfice | Coût/risque et retour arrière |
| --- | --- | --- |
| Copies actuelles | Aucun changement | Stockage redondant conservé |
| Mapping SQLite vers fichiers CAS | Déduplication | Migration schéma/lecteurs, ancien client incompatible : rejeté |
| CAS privé + liens physiques vers les chemins historiques | Mêmes octets, mêmes noms, anciens lecteurs compatibles | Inodes partagés et orphelins internes à gérer ; retour ancien binaire sans migration inverse |
| Supprimer anciennes affiches ou cache hors ligne | Gain apparent | Change le fonctionnement/la rétention : exclu |

## Décision proposée

Pool `.artwork-blobs/{raw,jpeg}/<SHA256>` privé par serveur, distinct des répertoires UUID de profils. Chaque ancien `<clé>.image/.jpg` reste un fichier ordinaire lisible avec O_NOFOLLOW : lien physique vers un contenu immuable validé, ou copie privée atomique si le filesystem ne permet pas le lien. Aucun lien symbolique introduit, aucun lien interserveurs. SHA/limites et directories contrôlés ; publication temporaire0600/fsync/rename, jamais d'écriture en place dans un inode partagé.

Les compteurs de budget restent **logiques par profil**, plafond2Gio historique inchangé. Les notes, l'historique, les bases/WAL/SHM, les UUID, le tampon vidéo60s/30s et les API sont inchangés. Aucun fichier `web/`, `ui/`, configuration serveur ou NAS n'est modifié. Le client web ne consomme pas ce cache Rust.

Migration explicite uniquement lorsque le client est fermé : parcourir les profils/images valides avec bornes, lire un seul fichier borné à la fois, vérifier identité et bytes, publier le lien atomiquement tout en conservant chaque chemin historique. Idempotente, interruption reprenable. Pas de VACUUM, purge de versions ni suppression de référence. Pas de scan global du cache sous mutex UI. Aucun GC sur chaque téléchargement : il serait quadratique et pourrait courir avec un autre écrivain. Des objets de pool sans référence peuvent donc rester après suppression d'un profil/échec de lien ; nettoyage différé, aucune prétention d'avoir traité cette rétention interne future.

## Preuves, conséquences et réversibilité

Fixtures : mêmes/distincts contenus, fallback sans hardlink, mauvais blob/liens symboliques, ancêtre profil symbolique, maintien des anciens chemins, suppression de profil sans perte de pool partagé, répétition stable, fichiers SQLite auxiliaires inchangés et relecture hors ligne. Mesure : compter les blocs alloués des **inodes uniques** avant/après, contrôler toutes les empreintes des références ; les tailles logiques restent identiques. APFS/clones/snapshots empêchent de promettre que cette estimation égale exactement les octets immédiatement rendus au volume.

Rollback : relancer l'ancien bundle, qui lit les mêmes chemins. Si besoin de désolidariser les inodes, recopier chaque feuille vers un fichier privé temporaire puis rename après vérification ; ne jamais modifier un blob ou une référence en place. Les contenus ne quittent pas le Mac ; aucune dépendance, coût cloud, authentification ou nouvelle exposition.

Validation : revue indépendante du candidat exact avant toute migration du cache réel ; demande utilisateur explicite couvrant optimisation/build/déploiement, pas de nouveau gate visuel. Mesures/résultats dans `docs/performance/RESOURCE-OPTIMIZATION-20261016.md`. Aucun gain seulement estimé n'est présenté comme réalisé.
