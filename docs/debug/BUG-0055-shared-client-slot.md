# BUG-0055 — navigation, affiches et lecteur en concurrence

14octobre2026, Mac Tauri/Pi, base `ae0ce44`, branche `codex/catalog-cache-and-client-fixes`. FIXED, preuve native finale en cours. Attendu : navigation sans refus causé par une image. Réel utilisateur : « Une opération est déjà en cours », sagas semblant inertes.

H1 confirmée : verrou unique/try_acquire_owned commun aux images/commandes/player et attente de loadSagaArtwork avant membres ; tick EOF sur le même verrou. Correction : images et tick/contrôles séparés, file de commandes/génération anti-contexte périmé ; membres rendus sans attendre l'image. Recharger/enrichir indiquent progression et erreur, pas de succès fictif.

Régression `saga-loading.test.js` : membres disponibles pendant attente artificielle de l'image. Tests Administration/lecteur et vrai Tauri dans les preuves du lot. Ni panne NAS ni permission de modifier des vidéos. Retour ancien bundle, sauvegarde serveur si bascule. ADR0013.
