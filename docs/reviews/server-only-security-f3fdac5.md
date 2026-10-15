# Revue de securite independante — serveur seul f3fdac5

- Reviewer : `/root/server_only_security_gate`, distinct des auteurs ; lecture seule du candidat et de l'operateur, aucun acces au Pi.
- Source gelee : `f3fdac503c4b845571981c1fd1bc00bd0b13ec21`.
- Delta cible : `02b6b8e6c542c869759558a02e53fa61aaa80ce3..f3fdac503c4b845571981c1fd1bc00bd0b13ec21` ; les constats approuves anterieurs restent reutilises hors de ce delta.
- Operateur exact, inchange : `/private/tmp/chillcenter-server-only.30Fmwa/deploy-server-only.sh`, SHA-256 `62081da7b574c4cd04c0ed3d09e238ea88dca4b4d4ba293b6963bc31a9d7f06e`, 9 548 octets.
- Verdict source : **APPROVED_SCOPED**.
- Verdict operateur : **APPROVED_SCOPED**.

Les sources `be57d9f` et `02b6b8e`, leurs rapports defavorables et l'ancien operateur `31de45de...c5e00` restent **STALE / NE PAS DEPLOYER**. La presente approbation vaut exclusivement pour les empreintes ci-dessus et la livraison binaire-seul dans la frontiere LAN existante.

## Portee et frontieres

La livraison autorisee remplace seulement le binaire serveur du Pi familial. Elle ne modifie ni fichiers web deployes, configuration, certificat/cle, unite systemd, NAS ou firewall. Aucun nouveau perimetre d'identite, d'autorisation, de secret, de cloud ou de tenant n'est introduit. Les profils isolent les donnees personnelles mais ne sont pas une frontiere de securite contre un client LAN natif ; cette decision existante n'est pas elargie.

La revue delta couvre la correction du finding de confusion titre/cle de `02b6b8e`, le nouveau parametre opt-in `selector`, son appel par le client natif, les regressions API et les octets exacts de l'operateur. Elle ne couvre pas le futur bundle Mac, l'artefact ARM64 encore a lier au SHA source, son transfert, l'execution sur le Pi, le NAS/TMDB reels ni la verification post-deploiement depuis le LAN.

## Constat source

- GET et PUT `series/{series}` extraient un `selector` optionnel avant tout acces base. Absent ou egal a `title`, il prend toujours le chemin titre historique, meme si le titre commence par `media:` ou `tmdb:tv:`. Seul `selector=key` prend le chemin opaque strict. Une autre valeur renvoie 400 avant mutation.
- Le mode cle refuse un segment sans prefixe opaque connu, puis reutilise les validations d'identite existantes de `Store::series` et `update_shared_series`. Il ne retombe jamais silencieusement vers une recherche par titre.
- Le mode titre reutilise exclusivement `legacy_series_view`/`update_legacy_series_view` approuves : profil existant, etat personnel applique par profil, transaction `IMMEDIATE`, CAS global, validation/collision et requetes SQL parametrees. Il ne change que les metadonnees communes historiques, jamais TMDB, chemin, identite source, saison, episode, poster ou etat personnel.
- Les parametres de requete historiques inconnus restent ignores : `SeriesSelector` ne porte pas `deny_unknown_fields`. L'extension ne rend donc pas invalides les URLs anciennes ; le choix de cle exige un opt-in exact.
- Le client natif ajoute `selector=key` aux cles de catalogue `media:`/`tmdb:tv:`. Le web deploye n'est pas modifie et continue sans parametre, donc en mode titre historique. Aucun cookie, session, redirect, secret, log ou donnee sensible nouveau n'est ajoute.
- La regression construit simultanement un titre litteral egal a une vraie cle opaque pour chacun des deux prefixes. Elle verifie les deux alias, GET implicite/`title`/`key`, PUT legacy et opaque, et l'egalite complete du groupe non selectionne, y compris identifiants et chemins. Le test homonyme precedent conserve CAS, profils et absence de fusion d'identite.
- Aucun manifeste, lockfile, package, runtime, listener, controle Host/origine, CORS, TLS ou serveur statique ne change.

## Preuves

| Preuve | Resultat | Limite |
| :--- | :--- | :--- |
| Revue ligne a ligne `02b6b8e..f3fdac5` | **PASS** | Delta serveur/client/API/tests/docs ; aucun acces Pi. |
| `git diff --check 02b6b8e..f3fdac5` | **PASS** | Source gelee locale. |
| Diff manifests/`Cargo.lock` | **AUCUN CHANGEMENT** | Pas d'audit dependency additionnel requis. |
| 7 tests serveur filtres, incluant collisions | **PASS** rapporte par le coordinateur | Tests locaux ; pas de serveur Pi. |
| `node scripts/validate-api-contract.mjs` | **PASS**, 76 chemins, OpenAPI `2026-10-14` | Validation statique locale. |
| `bash -n deploy-server-only.sh` et SHA-256 | **PASS**, `62081da7...7f06e` | Script non execute ; `shellcheck` indisponible. |

La suite Rust complete, Clippy et le cross-build sont executes par le coordinateur afin d'eviter la contention Cargo ; leurs resultats restent des gates d'integration/release distincts de cette revue de securite delta.

## Revue operateur exact

L'operateur est octet pour octet celui deja approuve. Les probes locaux utilisent les ports reels HTTP 8080 et HTTPS 8443. Il conserve les gardes de hash binaire/processus/configuration/unite/SPKI, montage et arbre web, le verrou cooperatif, le refus sur connexion active, le staging avant arret, la sauvegarde privee hors ligne validee exactement, le rollback vers l'ancien hash connu et la verification HTTPS stricte apres reprise.

Il ne comporte aucune ecriture sous `/usr/lib/chillcenter/web`, dans configuration/TLS/unite, le NAS ou le firewall. L'invariance web couvre contenu, inode, mode, proprietaire, taille et mtime. Les preuves read-only deja fournies placent les chemins concernes sur `/dev/mmcblk0p2`, avec environ 9 Gio libres pour environ 85 Mio de donnees.

## Verdict et conditions

**APPROVED_SCOPED**, sans finding HIGH/CRITICAL/MEDIUM/LOW dans `02b6b8e..f3fdac5`. Le finding MEDIUM de confusion titre/cle de `02b6b8e` est corrige par une selection explicite et couvert par regression.

L'operateur `62081da7...7f06e` peut etre utilise uniquement apres les gates independants restants, avec un binaire ARM64 reproductible construit depuis `f3fdac5...3ec21`, dont le hash passe en argument correspond exactement au fichier entrant. La verification de release doit constater suites/Clippy/cross-build verts, puis verifier depuis le LAN ports publics 80/443, alias API, certificat epingle, hash binaire, rollback et invariance web. Toute modification de source, binaire ou script rend cette approbation stale.
