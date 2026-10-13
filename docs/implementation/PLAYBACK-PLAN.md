# Lot CC-PLAY-001 — flux HTTPS et reprise

2026-10-13 · /root · DEV_FAST, autorisation « Après poursuis les autres étapes du dev ». Base applicative/branding84b08b1645a0c6fbccd17012eb7cbcd0fc7ad74a, branche codex/player-streaming, checkout isolé. G-03 et contrats/ADR0003/0004 approuvés, pas de reprise de discovery.

Slice : session liée profil/appareil/média/révision, expiration30s et heartbeat ; HTTP Range brut sous racine indexée, sans symlink ni chemin libre, ETag et révocation entre chunks ; sauvegarde ordonnée des positions/temps final ; helpers client Rust sur le transport SPKI existant. SQLite via actor existant, fichiers via workers bornés séparés, chunks64KiB. Trois profils actifs au plus, pas de transcodage ni accès NAS côté client.

Preuve : tests déterministes de sessions/temps/replays/profils, Range206/416/412 et confinement POSIX ; tests router/Store ; vrai serveur HTTPS loopback et client Rust avec fixture vidéo générée, sans bypass TLS ; régression workspace et clippy. Revue code/sécurité indépendante avant remise du nouveau parcours. L'auteur fait les checks, pas sa propre approbation.

Non livré par ce slice : surface AppKit/Tauri, UI de lecture, transfert/SSE complet, sous-titres indexés, pistes/vitesses/enchaînement, bundle libmpv distribué ou mesure Pi. Les routes ne doivent pas être présentées comme US016 complète. Une fixture/lecture headless ne prouve pas un rendu Tauri ni le NAS réel. Ces éléments restent prochains au DAG.

Retour : reprendre la base dans une branche isolée, arrêter clients incompatibles ; aucun schéma destructif ni vidéo modifié. Données de test privées neuves uniquement. Racine/IP foyer restent configurables et non contactées.
