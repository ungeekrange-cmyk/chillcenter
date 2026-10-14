# BUG-0019 — Quota et durée de vie du staging d’import

2026-10-14 · auteur `/root` · état : CORRECTED_PENDING_INDEPENDENT_REVIEW.

La revue sécurité de `86df190392df25403ddd6f1bae78f15e61e30c73` a confirmé que le quota ne comptait que les aperçus enregistrés, après réception, et que l’abandon HTTP pouvait laisser un ZIP hors registre. Une interruption puis un redémarrage permettaient donc des fichiers orphelins et un dépassement de quota. Aucun import du foyer n’a été exécuté.

Correction causale : comptage disque fail-closed incluant plans et orphelins, réservations de chaque écriture, possession RAII jusqu’à l’enregistrement, signal d’annulation de l’inspection et permis conservé par le vrai travail bloquant. Nettoyage au démarrage et après expiration limité aux noms générés par l’application ; fichiers inconnus et opérations actives conservés. Aucun fichier vidéo ou Cinéfolio n’est concerné.

Preuve locale : `cargo test --offline --locked -p chill-server archive_api::tests` : 5/5 PASS, dont quota avec fichier sparse/réservation/lien symbolique, nettoyage simulant un redémarrage, et annulation du handler HTTP pendant une inspection réellement bloquée. Le dernier test vérifie l’absence de mutation de base et le maintien du permis jusqu’à la sortie du worker. Environnement Mac arm64, Rust 1.99.0, fixtures temporaires. Revue indépendante de la correction encore requise ; ce document ne constitue pas une auto-approbation.
