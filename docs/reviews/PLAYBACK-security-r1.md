# CC-PLAY-001 — revue sécurité r1

2026-10-13 · CHANGES_REQUESTED · source761496fa41d994002cf57d0a176bafebac92fdd3, base84b08b1645a0c6fbccd17012eb7cbcd0fc7ad74a · reviewer indépendant /root/playback_security_review (security_reviewer, aucun fichier écrit). Transcription par /root.

PLAY-SEC-001 MEDIUM : saturation permanente des huit permis de flux par client LAN défectueux/malveillant. Permis acquis media_stream.rs:203 et détenus par StreamState :184/:234 ; timeout5s :124-149 seulement quand pollé ; middleware api.rs:203 ne couvre pas le corps. Huit grandes plages arrêtées sous backpressure ne repollent plus, gardent capacité même après release/suppression/racine. Autres clients :503MEDIA_BUSY.

Demande : échéance/annulation indépendante du polling, récupération démontrable de capacité, idéalement limite par bail/profil. Retest huit consommateurs arrêtés, release/expiration/suppression/racine, nouveau flux légitime. Finding ouvert sans risque accepté ; bloque approbation.

Conforme statiquement : confinement POSIX/no-symlink/fichier régulier/métadonnées par bloc ; Range/ETag, liaison bail/génération/média/révision, profils/progression ; Host/Origin/TLS inchangés, pas de nouvel exposé de secret/log/path/donnée ; seules dépendances internes/déjà verrouillées. Preuves138 tests/build/clippy/smoke fournies, non réexécutées par reviewer, ne couvrent pas backpressure. Limites NAS/Wi-Fi/Pi/cohérence filesystem, nativeTauri/transfert/SSE/sous-titres externes non prouvés. Aucun environnement modifié. Suivi BUG-0014, r2 requise.
