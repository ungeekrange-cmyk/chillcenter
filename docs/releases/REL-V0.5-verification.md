# Vérification indépendante — Chill Center v0.5 Mac

- **Lane** : vérification de release locale bornée ; inspection des preuves de test en mode `EXECUTION_REVIEW`, sans approbation de test distincte revendiquée
- **Environnement** : macOS 27.0.1 (26A434), Apple Silicon `arm64`
- **Périmètre** : packaging, publication locale, gel du tag et reçu de lancement uniquement
- **Travaux inclus** : CC-CONCEPT-20261016 révision 5 ; BUG-0073
- **Source / tag** : `1e16ee2058269b04e3bad7f57c2975eb07d77011` / tag annoté local `v0.5`
- **Artefact livré** : `/Users/ghillaumesoubigou/Documents/Dev/Perso/ChillCenter/artifacts/macos/Chill Center-v0.5.app`
- **SHA-256 exécutable** : `169ac5126ea4369174db35ad00f12ea5029bf51e686c0cf4ae2283c5fd604a84`
- **Déployeur/coordonateur** : `/root`
- **Vérificateur** : `/root/verify_v05_bundle`, distinct de l'auteur, du déployeur et du reviewer code
- **Revu à** : `2026-10-16T10:02:00Z`
- **Verdict** : **APPROVED_SCOPED** (`APPROVED` pour le seul périmètre packaging + lancement décrit ici)

## Preuves indépendantes

| Contrôle | Attendu | Observé | Résultat |
| --- | --- | --- | --- |
| Source et gel | tag `v0.5` vers la source revue | les tags annotés des dépôts source et canonique se déréférencent tous deux vers `1e16ee2058269b04e3bad7f57c2975eb07d77011` ; branche canonique active `codex/player-streaming` inchangée | PASS |
| Copie publiée | copie exacte du bundle local figé | manifestes SHA-256 relatifs identiques : `3d27348c599a5f387866ac658229eb083a9d69fc0b048852c0e3aa28723d1600` | PASS |
| Version / architecture | `0.5.0`, build `5`, arm64 | `Info.plist` = `0.5.0` / `5` ; Mach-O arm64 | PASS |
| Signature | signature locale profonde valide | `codesign --verify --deep --strict` exit `0` ; ad hoc, CDHash `74587ab033b7cff071c667f0b40d9f0d035ea29a` | PASS |
| Dépendances natives | fermeture autonome, 48 dylibs arm64, libmpv 2.5 | helper de vérification : `Loaded bundled libmpv client API 2.5` et `Verified 48 relocated arm64 dylibs`; identifiant `@rpath/libmpv.dylib`, zéro référence Homebrew, `/usr/local` ou chemin de build | PASS |
| Assets v0.5 emballés | références Noël et Halloween dans l'exécutable | chaînes `/branding/events/christmas.png` et `/branding/events/halloween.png` présentes ; les octets source ont été inspectés comme PNG 1536×1024 dans le candidat | PASS limité à la présence emballée |
| Frontières inchangées | aucun delta serveur/API/web/lecteur contre f3fdac5 | zéro fichier modifié sous `crates/chill-server`, `docs/api`, `web`, `crates/chill-player`, `native` et `apps/desktop/src` | PASS source |
| Lancement | processus du bundle canonique présent | reçu opérateur : `open -n` réussi, PID `33148`, actif à `09:59:17Z`; à `10:02Z`, `lsof -a -p 33148 -d txt -Fn` exit `0` et mappe exactement l'exécutable canonique | PASS lancement uniquement |
| Repli | ancien bundle conservé | `Chill Center-f3fdac5.app` présent, arm64, exécutable SHA-256 `d31bfff6e722c086b57a135fcd5744c1f8d0fe6383bb821a32e5cad019a7327c` | PASS disponibilité |

Commandes de preuve principales, toutes exit `0` sauf la lecture `ps` explicitement non disponible :

- `/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' .../Contents/Info.plist`, `...CFBundleVersion`, `lipo -archs`, `shasum -a 256` ; ensemble inférieur à une seconde.
- `codesign --verify --deep --strict '/Users/.../Chill Center-v0.5.app'` puis `codesign -dv --verbose=4` ; exit `0`.
- `python3 scripts/helpers/macos_bundle.py --verify-only --load-test --frameworks '/Users/.../Chill Center-v0.5.app/Contents/Frameworks'` ; exit `0`, 1,4 s.
- `git rev-parse 'refs/tags/v0.5^{}'` dans les deux dépôts et `git cat-file -t refs/tags/v0.5` ; deux objets `tag`, même SHA déréférencé.
- `lsof -a -p 33148 -d txt -Fn` ; exit `0`, chemin exact du bundle canonique. `ps` a répondu `operation not permitted` dans le sandbox : heure de départ et durée ne sont donc pas présentées comme relues indépendamment.

## Validité des preuves de test consommées

La revue a inspecté les assertions plutôt que de transformer les seuls comptes en approbation :

- suite ciblée Node **17/17 PASS** : choix dédupliqués, années décroissantes, OU entre genres, ET avec année/période, bornes inclusives, années inconnues, troisième clic, fermeture/focus, cache chaud, invalidation, publication atomique et réponses tardives ; assertions comportementales et de frontière jugées significatives ;
- interface connectée avec pont synthétique, filtres **PASS** : pagination 48/48/7, absence d'IPC après chauffe, focus, Escape et rechargement après invalidation ; ce n'est pas une preuve serveur réel ;
- apparence connectée synthétique **PASS** : quatre couples ambiance/thème, contraste calculé au seuil 4,5:1, assets calculés, ancrage, nombre borné de particules, pauses reduced-motion/masqué/lecture, mode off et largeurs 760/1000/1440 ; ce n'est pas une preuve WebKit/Tauri native ni une appréciation visuelle humaine ;
- `cargo check --locked --offline -p chillcenter-desktop` **PASS** sous Rust/Cargo 1.99.0 ;
- suite Node globale **74/76**, donc **pas PASS global** : deux assertions textuelles QA lecteur échouent aussi sur la base immuable f3fdac5 et aucun source lecteur n'a changé. Elles sont une limite historique non masquée, pas une régression démontrée de v0.5.

La revue code indépendante exacte est `docs/reviews/v0.5-code-1e16ee2.md`, verdict `APPROVED_SCOPED`. Le candidat antérieur `cef6f6e` et son hash `12f47a…` sont supersédés et ne sont pas livrés.

## Limites et rollback

**NOT_RUN / non revendiqué** : inspection visuelle humaine de la fenêtre, interaction native Tauri/WebKit, filtres sur la bibliothèque réelle du foyer, connexion API réelle, logs applicatifs, lecteur/libmpv en lecture, vidéo réelle, consommation RAM/GPU, Pi, NAS, Wi-Fi, web/PWA et fournisseur externe. Le processus présent prouve le démarrage, pas ces parcours.

Le bundle est signé ad hoc et non notarisé Apple. Aucune migration de données ni modification serveur n'accompagne cette release. Aucun déclencheur de rollback n'est observé dans le périmètre vérifié. En cas d'échec natif ou visuel humain, le déployeur doit quitter la nouvelle application puis relancer le bundle conservé `Chill Center-f3fdac5.app`; le vérificateur n'a exécuté aucun arrêt, relancement ou rollback.
