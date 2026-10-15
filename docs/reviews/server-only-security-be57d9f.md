# Revue de securite independante — serveur seul be57d9f

- Reviewer : `/root/server_only_security_gate`, distinct des auteurs ; lecture seule du candidat et de l'operateur, aucun acces au Pi.
- Source gelee : `be57d9f5ca535ffc48d5beb42d18a0adbc2df884` (`d95d5ea` porte le runtime revu ; `be57d9f` n'ajoute que le rapport de compatibilite).
- Base web active : `aad34cec60974782df959cf44c668e470fc4c658` ; source Mac/catalogue approuvee : `11aa529f2eced060156a3017456bf1d9634f8a56`.
- Operateur exact : `/private/tmp/chillcenter-server-only.30Fmwa/deploy-server-only.sh`, SHA-256 `31de45de4ae8faacf56dda6863f4c07c75339d289849f9c1bc3182ca465c5e00`, 9 528 octets.
- Revu a : `2026-10-15T21:49:23Z`.
- Verdict source securite historique : **APPROVED_SCOPED**, mais gate de deploiement **STALE** apres constat d'integration distinct sur la compatibilite web des homonymes.
- Verdict operateur courant : **CHANGES_REQUESTED** ; ne pas executer le SHA ci-dessus.

## Addendum apres revue — gate STALE, ne pas deployer

Une revue code/operateur distincte a etabli apres ce rapport que les redirections nftables 80/443 installees sur le Pi utilisent un hook `prerouting`. Les appels locaux du script vers le port 80 ne traversent pas ce hook : le probe HTTP local peut echouer avant l'arret alors que le service 8080 est sain. Ce comportement est fail-safe, mais les octets `31de45de...c5e00` ne sont pas un operateur executable valide pour cette livraison. Les probes locaux doivent viser directement 8080/8443 ; les ports rediriges restent a verifier depuis un hote LAN distinct lors de la verification post-deploiement.

La revue d'integration a aussi trouve une regression semantique de compatibilite pour les anciennes routes web indexees par titre lorsque deux series sont homonymes. Ce n'est pas une elevation de privilege dans la frontiere LAN sans authentification, mais cela rend le candidat exact impropre a la livraison demandee. Toute correction doit conserver les chemins opaques strictement isoles et borner le comportement legacy par titre aux champs communs historiques, sans pouvoir modifier identifiants TMDB, chemins, identite source, appartenance de saison ou autre champ propre au groupe. Un nouveau SHA source et un nouveau SHA operateur exigent une revue delta avant toute execution.

## Portee et frontieres

Livraison autorisee du seul binaire serveur sur le Pi familial deja expose sur le LAN. Aucun nouveau perimetre d'identite, d'autorisation, de secret, de cloud ou de tenant. L'absence d'authentification reste une decision de produit existante : les profils sont isoles fonctionnellement, pas une frontiere de securite contre un client LAN natif. HTTP 8080/80 reste le canal LAN explicitement accepte ; HTTPS 8443/443 et l'identite TLS epinglee restent inchanges.

La revue couvre les ajouts serveur/catalogue de synchronisation, identite serie/saison et affiches, leur integration avec les protections web deja deployees, puis les octets exacts de l'operateur binaire-seul. Elle ne couvre pas une nouvelle livraison des fichiers web, un acces au Pi, l'artefact ARM64 encore a fournir, un fournisseur TMDB reel, le NAS reel ou une verification post-deploiement.

## Constat source

- `d95d5ea..be57d9f` ne change aucun build input Rust/API ; `11aa529..be57d9f` est identique pour `chill-domain`, `chill-catalog` et `Cargo.lock`. `aad34ce..be57d9f` conserve octet pour octet `config.rs`, `main.rs`, `connection.rs` et `web_static.rs`. Le delta ajoute les routes Mac dans le routeur protege, sans modifier les controles Host/origine/listener.
- Les pages de delta sont liees au profil, au serveur et a un UUID de transfert ; chaque page reverifie l'existence du profil. Trois transferts/32 Mio memoire/120 s, 10 000 fiches et 32 Mio de document bornent le travail. Les checkpoints SQLite ne contiennent que UUID/signatures SHA-256, quatre par profil et 16 Mio globalement ; un transfert refuse ne persiste rien.
- `catalog.sync-generation` est cree en `0600`, avec `O_NOFOLLOW`, controle proprietaire/mode/taille/UUID ; un restore officiel change la generation. Aucun chemin NAS ou poster prive n'est serialise dans les DTO/checkpoints.
- Les imports d'affiches sont limites a 8 Mio, decodes sous limites 4096x4096/64 Mio, formats JPEG/PNG/WebP seulement, fichiers immuables `0600`. La lecture d'affiche reste sous la racine `posters` par ouverture composant-par-composant `openat/O_NOFOLLOW` et limite de 8 Mio.
- Les publications serie/saison relisent revision et groupe complet dans la transaction `IMMEDIATE`; le profil, l'ancre media et le parent TMDB de saison sont verifies. Les requetes SQL sont parametrees. Les erreurs HTTP et fournisseur restent generiques/redigees ; aucun nouveau log de secret, PII, chemin local ou payload.
- Le routeur unique conserve le refus DNS rebinding, les navigateurs cross-site, l'absence de CORS et la meme origine. Il n'ajoute ni cookie, session, redirection ni CSRF token : aucune authentification navigateur n'existe dans cette frontiere. Les routes qui transportent secrets TMDB ou archives restent refusees sur le listener HTTP. Le serveur statique lit `web/` a cote du binaire ; le binaire n'embarque ni ne reecrit les fichiers web.
- Aucun package/runtime nouveau : `sha2 0.10.9` est seulement reutilise par `chill-domain` et etait deja verrouille/utilise par `chill-server`.

## Preuves reproduites

| Commande | Resultat | Limite |
| :--- | :--- | :--- |
| `cargo test --locked --offline -p chill-server --lib --quiet` | **115 PASS**, 0 echec, 0 ignore | Relance locale avec permission loopback ; premier run sandbox : 114 PASS et seul bind loopback `EPERM`, puis test isole PASS. Aucun reseau externe/Pi. |
| `cargo test --locked --offline -p chill-catalog -p chill-domain --lib --quiet` | **75 + 64 PASS** | SQLite/fixtures temporaires seulement. |
| `node scripts/validate-api-contract.mjs` | **PASS**, 76 chemins, API `2026-10-14` | Contrat local, pas serveur actif. |
| `git diff --check aad34ce..be57d9f` | Sources sans erreur ; une ligne blanche finale documentaire historique signalee | Aucun defaut source. |
| `bash -n deploy-server-only.sh` et SHA-256 | **PASS**, hash exact `31de45de...c5e00` | `shellcheck` indisponible ; script non execute. |

## Revue operateur

L'operateur refuse les symlinks et les hashes inattendus avant arret, verrouille cooperativement le dossier d'installation, exige le binaire/processus `ffcc6349...b0f7`, la configuration `d6167...`, l'unite `76f7d...`, le SPKI `3018fb...`, le montage et l'absence de connexions etablies. Il stage et verifie le nouveau binaire avant l'arret. La sauvegarde hors ligne est creee sous l'UID 997 puis verifiee entree par entree (chemin relatif, type, taille, SHA-256, `quick_check`, ensemble exact). Seuls les sidecars SQLite connus, vides et de tailles exactes peuvent etre deplaces vers une quarantaine root `0700`, operation recuperable.

La publication est un `mv` du stage root:root `0755`. Toute erreur apres tentative d'arret restaure ou conserve exclusivement un hash connu, redemarre l'ancien binaire et exige la sante HTTPS stricte. Apres succes, l'operateur compare configuration, unite, SPKI, montage et l'empreinte du web incluant contenu, inode, mode, proprietaire, taille et mtime. Aucun chemin sous `/usr/lib/chillcenter/web`, aucune configuration, cle/certificat, unite, regle firewall ou NAS n'est ecrit par le script.

Note operationnelle non bloquante : le seuil libre de 256 Mio est teste seulement via `/var/lib/chillcenter/data`. Cette revue sans acces au Pi ne prouve pas que `/usr/lib/chillcenter` et `/root/chillcenter-rollback` partagent ce filesystem ; s'ils sont distincts, ajouter un controle libre pour chacun eviterait un echec de staging/rollback. Les echecs actuels restent surs : le stage `/usr` precede l'arret et un echec `/root` declenche le redemarrage de l'ancien binaire.

## Verdict et conditions

Verdict historique initial : **APPROVED_SCOPED** sans finding de securite dans la portee alors examinee. Etat courant apres addendum : **STALE / CHANGES_REQUESTED — NE PAS DEPLOYER**. Attendre un nouveau SHA source corrigeant la compatibilite homonyme, un nouvel operateur utilisant les ports locaux reels 8080/8443, leurs regressions ciblees et une nouvelle revue delta. L'ancien rapport ne vaut ni preuve de compilation reproductible ni autorisation d'execution.
