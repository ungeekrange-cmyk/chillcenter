# BUG-0057 — couvertures de listes, profil et annulation de la suite

14octobre2026, base `ae0ce44`. FIXED, validation visuelle utilisateur distincte. Création de liste non classée en panne certaine : l'utilisateur l'a réessayée avec succès.

Absences confirmées dans le code : libellé collection pour liste, pas de couverture personnelle/mosaïque9, profil simple sélecteur, annulation n'arrêtant pas le lecteur. Correction : libellés, mosaïque/custom image/CAS, nom+initiale/menu ; vignette suivante accessible, annulation ferme ; choix0/5/10/15/20/30/off. Nouveau profil Mac20s atomique ; anciens réglages conservés.

Tests DOM et `playlist_images_are_profile_scoped_cas_and_archive_v2_roundtrips` : CAS périmé refusé, isolation, remplacement sûr, archive2/import. Nouveau→ancien pour document2 non promis ; ancien→nouveau conservé. Vrai parcours EOF/clic/annulation/auto à vérifier et rapporter dans les preuves. Retour ancien bundle + sauvegarde pré-bascule si nécessaire. ADR0013.
