# BUG0073 — spécificité CSS de la pause des décorations

16 octobre 2026, candidat v0.5 non livré, Chrome isolé et vraie interface connectée avec pont synthétique. Attendu : animations décoratives en pause pendant une lecture ; réel : `animationPlayState` reste `running` quand la classe de présentation `player-open` est présente.

Une hypothèse confirmée : la règle d'animation événementielle `:root[data-ambiance=…] .ambiance-bat` a une spécificité supérieure à `.player-open .ambiance-weather *`. Son raccourci animation rétablit implicitement play-state=running. Le contrôle masque/onglet avait déjà passé, isolant la branche lecture.

Correction ciblée : ajouter `:root` au sélecteur de pause lecture pour égaliser la spécificité et laisser l'ordre de déclaration appliquer la pause. Preuve RED puis GREEN : `scripts/test-appearance-v05.cjs`, lecture **simulée par classe**, sans lecture vidéo ni accès au NAS. Rien modifié dans le moteur lecteur. Source exacte liée par REL-V0.5 ; statut FIXED/VERIFIED_LOCAL, validation native distincte à venir. Rollback par retour du bundle précédent, aucun état persistant.
