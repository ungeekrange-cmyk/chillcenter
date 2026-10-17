# BUG-0107 — Écarts d'adaptateur web détectés avant remise

17 octobre2026, candidat local issu de Mac3b2e28b + webe6e23db.
**FIXED / VERIFIED_LOCAL**, source a13902e. Ce n'est pas un signalement du Pi.

Symptômes reproduits ou confirmés par lecture des contrats : une association
TMDB réussie ne rafraîchissait pas la fiche du miroir ; séries opaques ou saisons
perdaient des informations ; transfert historique non borné ; import/export
sans le même cycle de confirmation/annulation ou avec réponse hors contexte.

Causes : mutations absentes de la liste d'invalidation, clé `media:<id>` traitée
comme représentant unique, retour anticipé sans métadonnées de saisons,
affiche de saison versionnée à zéro, ancien téléchargement snapshot non borné,
adaptation incomplète des travaux d'archives et des gardes de contexte.

Corrections : invalidation puis delta complet avant relecture, ancre résolue vers
le groupe conservateur, sélecteurs `key/title` explicites, métadonnées/révisions
et ID TMDB conservés, affiche versionnée par manifeste, limites UTF-8 des deux
protocoles, import annulable/plan explicite, export direct sans ZIP en RAM,
annulation serveur et garde des réponses tardives.

Preuves : suite Node intégrée265/265 et parcours web construit/IndexedDB/API
synthétique PASS, tests miroir montrant une nouvelle récupération d'affiche
après changement de révision, tests de requêtes apply/cancel et changement de
profil en vol. [Revue initialement défavorable puis APPROVED_SCOPED](../reviews/web-parity-20261017.md).
Aucun accès serveur domestique ni preuve de lecture matérielle tirée de ces tests.
