# Revue indépendante migration/quota/packaging

Candidat `d94418e0e9093bd49579d3ff5d66ffac64a45298`, parent `35c9cab`. Reviewer `/root/admin_contract_fix`, lecture seule, non-auteur du delta ; rapport reçu2026-10-14, conservé par coordinateur. **CHANGES_REQUESTED**,2P1 et1P2, aucune approval globale.

- P1 : les sagas fallback sans collections contournent les plafonds1000. Correction suivie BUG0025.
- P1 : publication du bundle par BSDmv peut imbriquer le stage dans un .app créé pendant le build. Correction suivie BUG0026 ; le fichier est au parent mais inclus dans le périmètre demandé.
- P2 : réouverture du chemin SQLite après validation du FD, remplacement local possible. Garde avant/après suivie BUG0027 ; ne prétend pas à atomicité d'un filesystem entièrement contrôlé localement.

Points positifs : quota et réservations communs imports/exports ; permis réel conservé au worker ; images descriptor-relatives dans cache explicite ; ZIP publié par hard link sans remplacement. Tests reviewer : legacy_export2/2 et vrai CLI2/2, diff checkPASS, target isolée ; aucun fournisseur/Pi/donnée foyer. Ces preuves précédentes ne couvrent pas les findings. Nouvelle révision sera soumise après corrections, aucun rapport historique effacé.
