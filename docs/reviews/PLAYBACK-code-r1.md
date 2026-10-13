# CC-PLAY-001 — revue code r1

2026-10-13 · CHANGES_REQUESTED · source761496fa41d994002cf57d0a176bafebac92fdd3, base84b08b1645a0c6fbccd17012eb7cbcd0fc7ad74a · reviewer indépendant /root/playback_code_review (code_reviewer, aucun fichier écrit). Transcription du retour par /root, pas auto-approbation.

MEDIUM : EOF HTTP transformé en erreur libmpv. media_stream.rs:221-231 retourne416 pour start==file_size ; chill-client/lib.rs:658-663 remonte toute416 en erreur ; HttpMediaSource::read_at, chill-player/lib.rs:326-367, ne retourne pas0 quand offset>=taille connue. Callback renvoie-1 au lieu de0, risque de fin normale en erreur et progression finale cassée. Demande : EOF local seulement si taille connue, régression et smoke jusqu'à Ended sans erreur.

Source descendant direct, diff --check propre. Preuves fournies réutilisées :138 tests, builds, clippy et smoke HTTPS load/pistes/seek/reprise ; ne couvre pas EOF. Reste du delta conforme : actor/book, ordre racine→registre, revalidation après I/O, séquences/générations, openat, FileVersion/ETag,206/412/416.

Limites : Tauri/AppKit, transfert/SSE, enchaînement et Pi/NAS réel exclus. Correction/r2 nécessaires ; ne pas effacer ce verdict.
