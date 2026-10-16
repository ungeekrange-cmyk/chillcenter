# BUG0096 — Fichier absent sans scan : lancement et retour graphique incorrects

16octobre2026, FIXED_LOCAL, `codex/configuration-matching`, base66aef9c. L'utilisateur cite « Avatar The Last Airbender - Aang »2024. Lecture seule de sa fiche sur le Pi : available=true, révision1 ; tentative d'ouverture autorisée : HTTP503 MEDIA_UNAVAILABLE, aucun bail acquis, aucun octet vidéo/progrès/transfert ou scan. Cela confirme une disponibilité indexée obsolète et un refus d'ouverture, pas à lui seul l'absence du fichier final plutôt qu'une panne de montage.

H1 confirmée dans le code : l'ouverture physique échouait sous un code générique sans mise à jour ciblée du catalogue. Correction : ne persister l'indisponibilité que lorsque racine et dossier parent sont accessibles et que seul le fichier final est absent. Autres erreurs restent temporaires. Conserver UUID/historiques, publier la révision par delta, refuser le bail, quitter proprement le lancement. Fiche avec lecture désactivée et action Configuration › Bibliothèque pour retrouver/gérer la référence.

H2 confirmée sur vrai DOM synthétique : body.player-open masque le catalogue, réduit la hauteur du document et fait perdre scrollY1200→0 avant le retour. Conserver/restaurer la position au rollback sans recharger le catalogue entier. Les gardes doivent empêcher un ancien échec de toucher un nouveau profil ou lancement.

Tests exigés : indexer fixture puis retirer le fichier, API réelle du serveur local, marquage/révision atomiques et idempotents, panne parent sans marquage, changement de chemin/réglage entre observation et publication, reprise de la grille sans saut. Aucun fichier utilisateur/NAS modifié. Résultats exacts et limites matérielles : [lot](../implementation/CONFIGURATION-MATCHING-20261016.md).

Résultats locaux : requête HTTP sur fixture supprimée PASS503/code ciblé, aucune création de bail, entrée voisine disponible ; tests Store atomique/CAS/rollback et classification parent/racine PASS. Parcours DOM connecté complet PASS avec scroll1200 conservé, fiche désactivée et action de récupération. Pas de correction du cas réel sur Pi annoncée avant son déploiement.
