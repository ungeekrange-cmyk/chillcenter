# Vérification indépendante — serveur df2870f

16 octobre 2026, après bascule14:44:03UTC. Vérificateur **`/root/verify_pi_df2870f`**, instance distincte des auteurs, du reviewer et du déployeur ; compte rendu transcrit par le coordinateur. Lecture seule, aucun déploiement ou restart par le vérificateur.

Verdict **APPROVED_SCOPED** pour la livraison serveur seul source **df2870f5776d20213bcb8c6c88a5bcb967091e97**, binaire SHA **14d84a0478b9f394bfe59b3b88588edb4a9305dfedfc0fb0b66449b39d034169**. [Exécution, approbation et retour arrière](REL-NEXT-POSTERS-SCROLL-MATCHING-001.md).

## Contrôles réels exécutés

- `systemctl` : service active/running, PID2326, NRestarts0 ; SHA installé **et** `/proc/2326/exe` égaux au livrable.
- Arbre web : empreinte exacte contenu/inodes/métadonnées **07868e61cd75dc36d5bf62eefd258b7409b6ed264bde726cdcf626d45b57fe2b**, inchangée. Configurationd6167cb1…, unité76f7d349…, TLS SPKI3018fb4d… inchangés. Montage4524c19a… identique au préflight, CIFS toujours en lecture seule, aucun accès aux vidéos demandé.
- Sauvegarde privée9b975f1e-1667-4565-aa87-a4e7e3da089e : ensemble exact du manifeste, tailles/hashes et SQLite `PRAGMA quick_check` en mode readonly/immutable PASS ; aucun secret exporté. Ancien binaire de rollbackd77e9e… présent et conforme.
- API2026-10-14 et aliasv1 PASS. **Vraie requête read-only `POST .../playback/next`**, catalogue réel : « Avez-vous déjà vu » **001→002 PASS**, sans acquérir de bail, lire une vidéo ou modifier progression/vu/notes.
- Pages HTTP8080/HTTPS8443 PASS avec TLS strict utilisant le certificat approuvé côtéPi. Depuis le Mac : page HTTP80 PASS, négociation TLS443 et SPKI attendu PASS. Une tentative curlHTTPS443 sans CA locale a refusé normalement le certificat auto-signé ; pas d'utilisation de `-k`, pas de preuve de téléchargement HTTPS443 strict depuisMac revendiquée.
- Aucun warning signalé dans le journal du service depuis la bascule.

## Limites conservées

Pas de nouveau test de lecture réelle Pi/NAS, fournisseur TMDB, scan, charge simultanée ou GUI Mac. La résolution API de la suite est démontrée, pas l'exécution humaine du compte à rebours. La règle courte et les correctifs de fiche/scroll disposent des tests ciblés antérieurs ; marquage court réel, enchaînement visuel et rendu Tauri restent à tester humainement. Le client web et son worktree n'ont pas été modifiés. Aucune clôture de toutes les US ou réévaluation des anciens échecs `player-qa` déduite de ce verdict.
