# G-DEV-FILENAME-SCROLL-001

2026-10-15, auteur/opérateur `/root`. Livraison locale ciblée réversible DEV_FAST, selon les consignes Chill Center et l'autorisation explicite de l'utilisateur de construire le bundle puis déployer sur son Pi. Pas de nouvelle authentification/cloud/public-exposure/coût/migration de données.

Source exacte `c610ea9a542f289902ebd6c356bcb1f369b7f150`, base livrée5bebf10. BUG0062/0063 uniquement et entrée de test manuel debug. Fonctions série/saison incomplètes préservées sur d'autres branches, hors artefacts. 129Rust de la source produit équivalente71bd7a0,16Node catalogue, contrat70routes, Clippy desktop/all-targets PASS. Build/signature/loader Mac et build/ELF Linux PASS. Validation humaine réelle du scrolling Tauri reçue après ouverture isolée. Pas de répétition de tests lecteur/vidéo.

Contrôles auteur ciblés : métadonnées en texte/chemins relatifs, aucune interprétation comme shell/HTML/URL, aucun changement de frontière identité/TLS/secrets/configuration/NAS ; mode QA debug isolé seulement. Ce gate n'est pas une triple revue indépendante de code ni une clôture globale des US. Le vérificateur post-déploiement est une instance distincte.

Autorité : « Ouvre la fenêtre Tauri isolée, je vais tester le défilement moi-même. Ensuite, termine le bundle Mac et le déploiement sur le Pi. », puis « test concluant sur le scrolling de mon côté ». Sauvegarde/rollback réel conservé avant remplacement, ancien binaire vérifié ; [release et hashes](../releases/REL-FILENAME-SCROLL-001.md). Contrôle post-déploiement à consigner avant remise finale. Anciennes sources, worktrees, bundles, historiques et sauvegardes conservés ; aucun push distant.
