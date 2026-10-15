# Corrections catalogue/cache — preuves du lot

14octobre2026. Branche `codex/catalog-cache-and-client-fixes`, base `ae0ce44`. Source finale7a0f480, serveur compilé07358f7 inchangé ensuite. Artefacts et état effectif dans [REL-CATALOG-CACHE-001](../releases/REL-CATALOG-CACHE-001.md). Revues successives CHANGES_REQUESTED conservées puis APPROVED exactes dans [rapport](../reviews/catalog-cache-final.md). ADR0013, BUG0055–0058. Auteur principal `/root`, UI `/root/client_ui_fixes`, premier serveur `/root/server_sync_and_playlist`, contrat `/root/api_snapshot_docs`.

## Vérifié

- `cargo test -p chill-client --lib --locked --offline` :35PASS, dont nouveau test navigation/filtres/séries/notes locaux après réouverture, transport vers un port non joignable (un HTTP accidentel ferait échouer), cache image persistant et liens refusés.
- Domaine :52PASS, dont nouveau CAS/image playlist/isolation/remplacement/nettoyage/export document2/import. Anciennes archives sans extension restent document1 ; conteneur1 inchangé.
- Serveur :98PASS, dont nouveau test route datée/v1, instantané201médias, pages immuables, positions/préférences et autre profil sans invalidation, vus du profil invalidants, profil supprimé refusé, défaut API10/newprofile20 atomique. Tests Axum/SQLite réels en processus, pas preuve de latence Pi.
- Contrat : `node scripts/validate-api-contract.mjs` PASS,70chemins, requêtes strictes/réponses extensibles, aucun renommage de version.
- UI :59tests dont vignette suivante accessible ; un ancien test de reconnaissance textuelle QA a échoué après élargissement du mode visible, corrigé et3tests QA repassés. Test Playwright synthétique PASS du worker : vrai DOM/menu profil/mosaïque9/image de liste ; pont synthétique, pas serveur réel.
- Vrai Tauri + HTTPS/SQLite locale, vidéos générées, réglages privés : `/private/tmp/chillcenter-cache-native.7iMcp3`. Essai initial page masquée timeout150s (outillage, NONPASS). Essai visible suivant : bandeau20s/nouvelle saison/clic PASS, puis relecture échoue PLAYER_ERROR ; BUG0058, étatfixture16000ms/durée16s.
- BUG0058 RED : test `a_completed_position_restarts_but_partial_playback_keeps_its_resume` renvoie16 au lieu0 avec ancien calcul ; GREEN après normalisation au FileLoaded (fin±250ms→0, reprise partielle inchangée).
- Vrai Tauri finale, nouvelles bases/client/port18633 : `UI_QA_RESULT true next-new-season-banner-20-click-cancel-closes-auto-0-series-speed`, exit0, traces visibles. Test du bandeau/clic/annulation/auto en mode panneau droit pour isoler cette régression, pas nouvelle validation plein écran. Mode shell/port18631 exit0 : `native-Tauri-shell-sidebar-search-admin-logo-wide-detail-series-drilldown-saga-playlist-picker-TMDB-secret-advice-filters`. Cette vérification, déjà lancée quand l'utilisateur a demandé de réduire les tests, est terminée ; aucun nouveau parcours lecteur général prévu.
- Pi lecture seule : service actifPID5145/NRestarts0, architectureaarch64, NAS CIFSro. Aucun changement serveur à ce stade. Autorisation humaine explicite pour sauvegarde privée/arrêt bref/remplacement/redémarrage reçue, sans modification NAS/configuration.

## Preuves finales supplémentaires ciblées

- Client36PASS sur073 par reviewer indépendant ; sur7a0, tests navigation locale/suppression immédiate avant échec GET et purge/symlinks repassés indépendamment. Le test d'échec réseau entre au point de confirmation DELETE via helper interne, puis exécute le vrai GET vers port9 inaccessible ; ne prétend pas simuler un serveur HTTP DELETE réel.
- Domaine : test53 supplémentaire déterministe seconde connexion/scan concurrent PASS ; miroir serveur ciblé PASS. Clippy trois crates/all-targets sur073 puis client/all-targets sur7a0 PASS.
-8testsNode sagas/mosaïque/profil/TMDB PASS, puis1test ordre Administration PASS. Aucun parcours général lecteur relancé après la restriction explicite.
- Builds Mac7a0 et serveurARM64/073 PASS ; signature/48dylibs/loader bundle Mac PASS. Pas de modification runtime/dépendance.

## Déploiement Pi du15octobre

Après report initial conservé dans la release, confirmation utilisateur « Non déploie la nouvelle version ». Opérateur inchangé/revu exit0 à02:06:48UTC : sauvegarde offline privée c8897485… vérifiée, rollback1eb24… conservé, binaire11cb… actifPID6451, HTTPS strict daté PASS. NAS/configuration/unité/SPKI inchangés. Vérification indépendante connexion/cache en cours, pas encore un résultat prétendu. Fenêtres/appdata/bases de tests séparées du foyer ; clés privées et sauvegardes restent sur le Pi.

## Limites

Préchargement progressif des images, plafond2Gio/profil. Si métadonnées changent : copie complète cohérente, pas delta élémentaire. Les profils existants conservent leur délai ;20s est initialisé pour les nouveaux profils créés par le Mac et configurable pour les autres. Mesure nouvelle consommation/RSS Pi en condition réelle et essais visuels humains restent à distinguer des tests fonctionnels. Aucun nouvel appel TMDB/IA payant, benchmark Dune ou validation son/HDR/multi-écran revendiqué.
