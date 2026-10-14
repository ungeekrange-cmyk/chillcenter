# BUG-0033 — Révisions initiales absentes après scan

2026-10-14, vraie webview/serveur synthétique source0456474 + root. Scan créait bien les deux nouvelles identités ; le catalogue paginé et les notes fonctionnaient, mais les premières opérations dépendant d'une révision (empreinte/fiche) échouaient. SQL lecture seule : cinq identités, seulement trois révisions ; les deux nouvelles UUID n'avaient aucune ligne dans cc_media_revision.

H1 confirmée : insert_identities publiait les identités mais pas leurs révisions. L'ancien Store::catalog, non paginé, créait ces révisions paresseusement ; les fixtures de tests l'appelaient et masquaient le défaut. Le vrai catalogue utilise browse_catalog et ne doit pas charger toute la bibliothèque pour initialiser une fiche.

RED : `scan_publishes_new_media_revision_without_legacy_catalog_warmup` ouvre une base vide, snapshot, puis media_revision/fingerprint_target directement, sans catalogue legacy. Correction : INSERT OR IGNORE des révisions1 dans la même transaction de snapshot/identités, conserve les révisions existantes et rollback intégral sur échec. Aucun chemin vidéo, historique ou source Cinéfolio modifié.

RED puis GREEN exécutés, le test passe avec Cargo offline/locked ; build serveur/desktop PASS. L'état d'empreinte d'une référence pas encore traitée devient `not_requested` (200, sans hash/chemin), plutôt qu'une erreur « valeur invalide ». Le parcours natif réassociation reste à rattacher après exécution. Les premiers essais échoués restent des FAIL, pas des PASS. Revue finale requise.
