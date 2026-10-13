# BUG-0006 — tâches SQLite survivant au délai de la requête

2026-10-13 ; auteur `/root`, cause identifiée indépendamment `/root/ui_code_review` à `b8737bba65f010e349a8387201faf8d73cebe2b0`. Pas une panne reproduite sur Pi : contrôle de flux statique puis fixture déterministe locale.

H1 unique : une mutation en file derrière le snapshot n'est pas annulée avec sa réponse ; son closure s'exécute même après destruction du receiver. Inspection : `tx.send(job(store))` exécutait job sans tester la fermeture, scan occupait le même acteur. Cause confirmée. Échec possible : timeout affiché, création/suppression/préférence appliquée tardivement. HIGH.

Correction : scan fusion/sérialisation/persistance sur connexion SQLite dédiée en `spawn_blocking` ; WAL conserve les lectures interactives. Acteur interactif borné16, délai10s inférieur aux20s client/30s middleware, receiver fermé ignoré, token annulé au drop, hooks SQLite de progression et pré-commit qui refusent une écriture déjà annulée/expirée ; installation des hooks fail-closed. Pas de garantie distribuée « exactement une fois » : une coupure après commit exige toujours de recharger avant retry ; message de timeout explicite.

Régression : `cancelled_queued_and_started_mutations_never_commit_late` bloque l'acteur, expire la requête, libère et vérifie aucune ligne créée, avant et après démarrage. `interactive_reads_remain_responsive_during_large_snapshot_write` maintient une transaction de2000 vidéos ouverte sur le writer distinct et vérifie lecture profils sous1s. Ces fixtures ne mesurent pas les performances Pi/NAS/Wi-Fi.

État FIXED ; SHA et vérification exacte dans INTERFACE-EVIDENCE. F-01/F-04/F-05/F-14, ADR-0001/0002 ; mécanisme d'annulation réversible, aucune migration. Ne pas remettre b8737bb ; checkpoint de retour fe85cea.
