# BUG0098 — Pause/reprise vidant le tampon et réinitialisant l'interface

16octobre2026, FIXED_LOCAL, `codex/configuration-matching`, base66aef9c. À la reprise : barre au début et icône lecture jusqu'au redémarrage. L'utilisateur précise que le tampon doit rester et continuer à charger pendant la pause.

H1 confirmée : PlayerCommand::Pause appelait release après sauvegarde ; Resume, sans bail, appelait native_player::load. Le moteur et son cache étaient rechargés. Le code respectait l'ancienne règle de libération à la pause, mais cette règle est désormais explicitement remplacée par [ADR0022](../architecture/decisions/ADR-0022-paused-playback-retains-buffer.md).

Correction attendue : même moteur/bail et remplissage borné du tampon en pause, heartbeat maintenu, aucun crédit vu pendant l'arrêt ; reprise normale sans loadfile. Position conservée, bouton pause immédiat et roue de chargement pendant l'attente. Échec/transfert/échéance ne doivent pas permettre une lecture sous bail perdu. Test ciblé du cache, pas un nouveau parcours intégral de lecteur. [État exact et preuves](../implementation/CONFIGURATION-MATCHING-20261016.md).

Correction implémentée dans le bridge (pause sans release, tick/renouvellement conservés). Preuve empirique ciblée `libmpv_keeps_bounded_readahead_alive_while_paused_without_reloading` : fixture synthétique30s, stream mémoire ralenti, lectures encore en croissance sur2s de pause, durée de cache≤60,1s, reprise du même Player sans deuxième loadfile. Aucun réseau/Pi/NAS/fournisseur ou fenêtre UI. Test réussi en3,44s au premier passage ; avertissement macOS kIOSurface non fatal conservé. Position/icône/roue et restauration sur erreur également couvertes en DOM ; cela ne certifie pas le débit réel du Wi-Fi.
