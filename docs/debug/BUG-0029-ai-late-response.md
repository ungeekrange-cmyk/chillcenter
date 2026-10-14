# BUG-0029 — Réponses IA après fermeture / poll interrompu

2026-10-14, reproduction indépendante en lecture seule de `ab252856` par admin_contract_fix. Le contrôleur ne changeait pas de génération à la fermeture pendant Start : la réponse tardive installait un job et un timer dans une fenêtre fermée. Une erreur CLIENT_BUSY au Status arrêtait le poll ; une erreur d'ancien profil écrivait dans le statut courant. La fermeture pendant Apply donnait à tort une impression d'annulation.

Correction : génération invalidée à la fermeture, annulation explicite du job retourné tardivement, erreurs gardées par génération, quatre tentatives uniquement pour la lecture Status. Start et Apply ne sont jamais rejoués automatiquement. Apply désactive fermeture et double confirmation pendant transmission ; le changement de profil invalide l'affichage sans promettre le rollback d'une application confirmée. Une réponse Apply inconnue impose de vérifier le catalogue.

Preuves : `node --test ui/connected/ai.dom.test.js ui/connected/ai.test.js` PASS, 5 tests. Évalue le vrai contrôleur avec promesses/timers contrôlés (fermeture Start, profil/erreur tardive, Status busy, Apply confirmé). Pas de fournisseur externe appelé. Revue finale indépendante requise.
