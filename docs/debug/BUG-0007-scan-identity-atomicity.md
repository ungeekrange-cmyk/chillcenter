# BUG-0007 — catalogue validé avant les identifiants stables

2026-10-13 ; auteur `/root`, revue `/root/ui_code_review`, source `b8737bba65f010e349a8387201faf8d73cebe2b0`. MEDIUM.

H1 unique : `Store::snapshot` validait le catalogue puis les identifiants dans une transaction différente. Inspection confirme que l'échec du second commit ne pouvait annuler le premier ; message « catalogue conservé » inexact. Pas de panne de stockage utilisateur reproduite.

Correction : `Database::snapshot_with` fusionne/sérialise avant la transaction et ajoute le mapping stable dans **la même transaction** que media et last_scan. `Store::snapshot` appelle cette extension ; l'API conserve réellement l'ancien catalogue sur échec. Aucune réassociation automatique par titre, les UUID existants restent inchangés.

Test `failed_identity_extension_rolls_back_catalogue_scan_marker_and_mapping` : trigger SQLite ABORT entre media/last_scan et nouvelle identité C, puis comparaison catalogue entier, marqueur et nombre d'identités aux valeurs précédentes. État FIXED ; résultat/SHA exact dans INTERFACE-EVIDENCE. F-06/F-14/F-16, ADR-0002. Aucune migration ou vidéo touchée ; rollback fe85cea.
