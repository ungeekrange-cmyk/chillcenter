# BUG0100 — Collision du helper de configuration avec son pont global

16octobre2026, FIXED_LOCAL, candidat non livré `codex/configuration-matching`. Détecté pendant le parcours navigateur du nouveau panneau, pas sur le Pi.

H1 confirmée : une déclaration globale function configurationLibraryCommand était remplacée par window.configurationLibraryCommand ; l'adaptateur se rappelait lui-même. Le formulaire capturait l'erreur et aucune commande save_library n'était envoyée. Les seuls tests CommonJS du helper ne reproduisaient pas cet environnement navigateur.

Correction : helper pur buildLibraryConfigurationCommand distinct de l'adaptateur global, export de test conservé. Le parcours réel DOM vérifie désormais explicitement la commande avec les deux emplacements supplémentaires. Première preuve défavorable conservée dans le rapport du lot ; aucun utilisateur ou serveur n'a reçu ce candidat. [Preuves](../implementation/CONFIGURATION-MATCHING-20261016.md).
