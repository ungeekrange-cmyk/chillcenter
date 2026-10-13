# Preuves locales des fondations

Date : 2026-10-13. Mac Apple Silicon, macOS 27.0.1, Rust 1.99.0. Candidat source `be83139559144dec1fd7e55be8b900ab1c1d6036`, branche `codex/implementation`. Aucun Pi/NAS ni app Tauri testés par cette suite.

## Suite intégrée

Commande : `cargo test -p chill-catalog -p chill-domain -p chill-server -p chill-client -p chill-player`. Sortie 0. 96 tests : catalogue 64, client 8, domaine 9, lecteur 4, serveur 11 ; doc-tests 0 échec. Cargo.lock fixe les versions ; exécution avec les deux nouvelles dépendances téléchargées. Défaut initial d'include fixture corrigé avant ce succès.

Preuve lecteur : libmpv 0.41.0 chargé réellement depuis Homebrew, MP4 généré via FFmpeg avec audio et sous-titre intégré, fourni par `stream_cb` en mémoire ; découverte des pistes, pause et seek vérifiés, annulation réveillant un flux bloqué testée. Ceci **ne prouve pas** le rendu AppKit ni une lecture HTTPS de bout en bout. Initialisation VideoToolbox headless infructueuse, repli logiciel observé ; pas de promesse d'accélération matérielle acquise.

## Smoke service réel de configuration

Binaire source serveur `5115afd`, lancé temporairement sur `127.0.0.1:18443` avec données sous un répertoire `mktemp`, racine initiale fictive `/mnt/smoke-initial`. Validation TLS par `curl --cacert <certificat temporaire>` ; pas de `-k`, aucune confiance globale ajoutée. Les seules données créées sont temporaires.

- `GET https://localhost:18443/api/v1/server` : sortie 0, identité/API v1, HTTP/2 accepté après correction.
- `PUT /api/v1/admin/library`, JSON racine `/mnt/configurable-smoke-video`, révision 0 : sortie 0, racine renvoyée et révision 1.
- `POST /api/v1/profiles`, profil fixture : sortie 0, UUID créé. Le bootstrap n'exige pas de profil préalable ni de montage NAS.
- Arrêt/redémarrage du même service, puis `GET /api/v1/admin/library` : sortie 0, même racine et révision 1. Service de test arrêté ensuite.
- Tests API négatifs : Origin navigateur et nom Host non autorisé refusés ; modification de config avec ancienne révision refusée 409. Tests TLS : calendrier 12/18 mois y compris fin de mois, renouvellement et changement de SAN sans changement de clé.

Le démarrage sandbox avait refusé l'écoute ; le smoke loopback a été exécuté après autorisation ciblée. Les premiers défauts réels TLS-provider et HTTP/2 sont conservés dans BUG-0001/0002. Aucun résultat physique Pi/Wi-Fi inventé. Rollback : checkout du commit parent, configuration/database de test isolées ; aucun fichier vidéo touché.

Outillage local : installation Homebrew mpv 0.41.0_10 (et ses dépendances, dont FFmpeg 9.0.2). Ce n'est pas le packaging autonome final du client. Versions de SHA2 0.11.1 et libloading 0.9.0 vérifiées dans leur documentation primaire avant la mise à jour et testées par cette suite.

## Corrections et re-tests ultérieurs

Source `9fb4cbc026b72ae0464328700db86ddeefa17f7c` : `cargo test --workspace --locked --offline`, sortie 0, **105 tests** (64/9/10/7/15). `cargo check --workspace --all-targets --locked --offline`, sortie 0. Le workspace exclut maintenant le squelette desktop et le probe autonome ; ces commandes ne prouvent pas une app Tauri fonctionnelle.

Les corrections de revue comprennent les positions de reprise hors derniers 10 %, le lifetime renderer/core/library, l'ABI libmpv API2.0, validation stricte de Content-Range et un seul worker IO borné/joint par flux. Les rapports historiques CHANGES_REQUESTED restent conservés ; seules de nouvelles revues peuvent fermer les findings, pas ces tests auteur.

Serveur durci `a06d95f` : 15 tests réussis. Smoke réel loopback avec le même certificat validé : la racine reste persistante après nouveau build/redémarrage. Une socket TCP qui n'envoie aucun handshake TLS est fermée après **5002 ms** (test Python réseau, sortie 0) ; aucune connexion au Pi. Service de test arrêté ensuite. Revue sécurité r2 APPROVED pour le serveur seulement, LOW fsync ouvert et propriété `/root`.

Preuve AppKit auteur `9f96b0f`, répétée après remediation native par l'auteur : sortie 0 hors sandbox, NSOpenGLContext réel sur thread principal, rendu de la vidéo H264/AAC générée en FBO320×180 et640×360, pixels non nuls et256 valeurs distinctes, seek1,25s/pause/resume/resize. Le sandbox graphique avait refusé NSOpenGLPixelFormat ; ledger séparé `native/macos-render-probe/DEBUG.md`, ce n'est pas un défaut lecteur résolu par un faux rendu. Le Tauri NSView et un flux HTTPS dans ce même probe restent NOT_RUN.

## Checkpoint final de ce lot de fondation

Source `3062ffec596c797bf4713433c34d5cf7bc54484f` : **108 tests locked passés**, 64 catalogue +9 client +10 domaine +10 lecteur +15 serveur. Clippy workspace/all-targets avec `-D warnings` passe. Correction finale : channel worker capacité1 + single-flight atomique, tests déterministes lecture avant recv/relectures/backpressure/cancel/reuse/join. Le lock du probe est à jour et sa compilation/exécution locked est passée dans la revue indépendante.

Revue test-execution r2 APPROVED pour ce périmètre borné ; revue code r3 APPROVED uniquement pour le delta worker (autres capabilities NOT_READY conservées) ; sécurité serveur r2 APPROVED pour `a06d95f` avec LOW ouvert. Rapports sous `docs/reviews/`. Le premier registre de 96/105 tests ci-dessus demeure historique et ne représente pas le produit fini. Aucune sortie Pi/Linux/NAS, application `.app`, import ZIP ou HTTPS vidéo E2E n'est ajoutée implicitement.
