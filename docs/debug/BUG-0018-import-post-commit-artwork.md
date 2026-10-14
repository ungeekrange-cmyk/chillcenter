# BUG-0018 — Affiches supprimées après commit d'import

2026-10-14 · CORRECTION_CANDIDATE · source revue86df190392df25403ddd6f1bae78f15e61e30c73.

Revue indépendante archive_code, HIGH : apply_portable_import commit puis lecture epoch fallible ; un échec post-commit renvoie Err, donc OwnedPosters supprime des affiches déjà référencées. Confirmé par lecture exacte de la frontière transactionnelle ; pas une hypothèse spéculative. Le diagnostic est distinct de l'auteur de correction /root. Aucun import foyer exécuté.

Correction causale : calcul de l'epoch sous transaction, commit comme dernière opération SQLite fallible ; retour construit sans lecture après commit. L'acteur conserve les nouvelles affiches dès résultat de commit réussi, indépendamment de l'annulation HTTP.

Régression locale exécutée : `cargo test --offline --locked -p chill-domain import::tests::successful_commit` PASS. commit_hook active un authorizer qui interdit toute lecture d'epoch après commit ; apply renvoie encore succès, la destination contient le profil, et une tentative indépendante de lecture post-commit échoue bien. Le test prouve la frontière, pas un test réel de ressources ou migration. Rerevue indépendante requise avant livraison.
