# BUG-0005 — profil affiché différent du contexte Rust après échec

2026-10-13 ; macOS Apple Silicon ; auteur `/root`, observation indépendante `/root/ui_code_review` ; source `b8737bba65f010e349a8387201faf8d73cebe2b0`.

Attendu : si la sélection de B échoue, le profil A reste autoritaire dans l'interface, les requêtes et le dernier choix local. Réel : `Connected.active` et `last_profiles` passaient à B avant GET/persistance ; le webview conservait A sur erreur. Risque de modification de l'état de B sous l'affichage de A (HIGH).

H1 unique : l'absence de rollback cause cette divergence. Expérience : fixture locale A/B, transport fermé `127.0.0.1:1`, sélection B puis assertions profil/choix mémoire/choix disque A. Résultat avant correction : test FAILED (`left B / right A`, cargo exit101). Cause confirmée ; pas de données utilisateur utilisées.

Correction : charger la vue prospective sans mémoriser B, valider/persister ensuite, restaurer A et le choix antérieur sur chaque erreur. Aucun appel personnel supplémentaire entre la vue validée et la remise au webview. Le test couvre aussi une panne de sauvegarde après une vue prospective réussie, par un répertoire temporaire absent ; fichier précédent inchangé. En parallèle, la connexion restaure réglages/confiance et neutralise le contexte si l'identité ou la vue échoue.

État : FIXED, preuve et SHA corrigé dans `docs/implementation/INTERFACE-EVIDENCE.md` après suite/revue exacte. Régression : `failed_profile_switch_preserves_displayed_context_and_last_choice`. F-05/F-06, ADR-0002/0005. Rollback : checkpoint `fe85cea`, pas de migration ni vidéos touchées ; ne pas réutiliser le candidat défectueux b8737bb.
