# Revue de securite independante — serveur seul 02b6b8e

- Reviewer : `/root/server_only_security_gate`, distinct des auteurs ; lecture seule du candidat et de l'operateur, aucun acces au Pi.
- Source gelee : `02b6b8e6c542c869759558a02e53fa61aaa80ce3`.
- Delta cible : `be57d9f5ca535ffc48d5beb42d18a0adbc2df884..02b6b8e6c542c869759558a02e53fa61aaa80ce3` ; les preuves de securite approuvees du runtime `d95d5ea`, du catalogue `11aa529` et du web `aad34ce` sont reutilisees hors de ce delta.
- Operateur exact : `/private/tmp/chillcenter-server-only.30Fmwa/deploy-server-only.sh`, SHA-256 `62081da7b574c4cd04c0ed3d09e238ea88dca4b4d4ba293b6963bc31a9d7f06e`, 9 548 octets.
- Verdict source : **CHANGES_REQUESTED**.
- Verdict operateur : **APPROVED_SCOPED**.

Le rapport `server-only-security-be57d9f.md` et son operateur `31de45de...c5e00` restent **STALE / NE PAS DEPLOYER**. La source `02b6b8e...80ce3` ne doit pas etre deployee en raison du finding ci-dessous. L'approbation operateur vaut exclusivement pour son empreinte ci-dessus et devra etre revalidee si ses octets changent.

## Portee et frontieres

La livraison autorisee remplace seulement le binaire serveur du Pi familial. Elle ne modifie ni les fichiers web deployes, ni configuration, certificat/cle, unite systemd, NAS ou firewall. Aucun nouveau perimetre d'identite, d'autorisation, de secret, de cloud ou de tenant n'est introduit. L'absence d'authentification reste la decision de produit LAN existante : les profils isolent les donnees personnelles de lecture, mais ne sont pas une frontiere de securite contre un client LAN natif. HTTP 8080/80 reste explicitement accepte ; HTTPS 8443/443, le controle Host/origine et l'identite TLS epinglee restent inchanges.

Cette revue delta couvre le retablissement de la semantique historique GET/PUT indexee par titre, l'absence de fusion d'identite entre homonymes, et les octets exacts de l'operateur. Elle ne couvre pas l'artefact ARM64 encore a lier au SHA source, son transfert, l'execution sur le Pi, un fournisseur TMDB reel, le NAS reel ni la verification post-deploiement depuis le LAN.

## Constat source

- Le delta de production est borne a `administration.rs` et `catalog_api.rs`; `api.rs` ajoute le test de regression. Aucun manifeste, lockfile, package ou runtime ne change.
- Les chemins legacy exigent toujours un profil existant. La lecture applique `profile_view` a chaque episode : les etats regardes/notes d'un autre profil ne sont pas divulgues. La mutation reste celle, historique, des metadonnees communes a tous les profils ; dans la frontiere sans authentification acceptee, elle n'ajoute pas une nouvelle elevation de privilege.
- Le PUT legacy utilise une transaction SQLite `IMMEDIATE`, compare la revision globale avant ecriture, refuse nom vide/surdimensionne et collision, valide `SeriesPatch`, parametre les requetes SQL puis commit atomiquement. Une revision per-media est incrementee et la vitesse legacy est migree pour chaque profil sans exposer son contenu.
- Les seuls champs modifies par le helper legacy sont `series`, `series_manual`, `series_overview` et `series_metadata`. Identifiant TMDB, chemin, identite source, saison, episode, poster et etat personnel ne sont ni associes ni fusionnes.
- Un GET legacy homonyme agrege les episodes pour l'ancien web mais publie `tmdb_id: null` et `season_metadata: {}` lorsque plusieurs identites existent. Il ne persiste aucune donnee. Les deux alias API `2026-10-14` et `v1` restent couverts.
- Le delta ne touche pas listeners, controle Host/origine, CORS, statique, TLS, archives/secrets ni journalisation. Il n'ajoute ni cookie, session, redirection ou mecanisme CSRF ; aucune nouvelle donnee sensible n'apparait dans les erreurs ou logs.

## Preuves

| Preuve | Resultat | Limite |
| :--- | :--- | :--- |
| Revue ligne a ligne `be57d9f..02b6b8e` | **PASS** | Delta Rust/API et test uniquement ; aucun acces Pi. |
| `git diff --check be57d9f..02b6b8e` | **PASS** | Source gelee locale. |
| Diff manifests/`Cargo.lock` | **AUCUN CHANGEMENT** | Pas d'audit dependency additionnel requis. |
| Regression API homonymes | **PASS** rapporte par le coordinateur | Deux alias, deux profils, GET sans mutation, PUT batch + CAS, identites preservees, chemins opaques isoles. |
| `bash -n deploy-server-only.sh` et SHA-256 | **PASS**, `62081da7...7f06e` | `shellcheck` indisponible ; script non execute. |

Les suites completes et Clippy sont executes par le coordinateur afin d'eviter la contention Cargo ; leur resultat reste un gate d'integration/release distinct de cette revue de securite delta.

## Finding bloquant

**MEDIUM — confusion entre titre legacy et selecteur opaque, integrite d'une autre serie**

- Localisation : `crates/chill-server/src/catalog_api.rs:494-497` (GET) et `:553-575` (PUT).
- Cause : toute composante de route commencant par `media:` ou `tmdb:tv:` est classee comme cle opaque. Le contrat historique acceptait pourtant tout titre non vide jusqu'a 500 octets et ne reservait pas ces prefixes ; un titre issu d'un nom de fichier ou d'un fournisseur peut donc etre litteralement `tmdb:tv:1001` ou `media:<uuid>`.
- Impact : l'ancien web demandant ce titre ne recoit plus la vue agregee historique. Plus grave, un PUT peut etre dirige vers un groupe opaque distinct portant cet identifiant et modifier ses metadonnees communes. Les champs d'identite ne sont pas reecrits, mais l'autorite de la requete est appliquee au mauvais objet : c'est une atteinte a l'integrite et une confusion de selecteur a la frontiere legacy/opaque.
- Probabilite : le cas est rare mais atteignable par une donnee catalogue valide ; aucune validation existante n'interdit ces prefixes. Le test `legacy_title_series_api_keeps_web_reads_and_edits_without_merging_identities` ne couvre que `Doctor Who` et ne detecte pas la collision.
- Remediation exigee : rendre sans ambiguite le choix entre titre legacy et cle opaque tout en conservant la semantique historique pour **tous** les titres autorises. Ajouter une regression API reelle ou coexistent un titre prefixe et la cible opaque correspondante ; verifier GET/PUT legacy, absence de mutation de l'autre groupe, les deux alias et le chemin opaque explicite. Ne pas resoudre en interdisant/faisant echouer un ancien titre valide, puisque la compatibilite complete est une exigence explicite.

## Revue operateur exact

Le correctif remplace les quatre probes locaux errones par les ports reels du service : HTTP `8080` et HTTPS `8443`. Les redirections publiques `80/443`, qui ne s'appliquent qu'au trafic `prerouting`, restent a verifier depuis un hote LAN distinct apres deploiement. Les octets approuves conservent les controles precedents : hash exact du binaire installe, entrant et processus, hashes configuration/unite, SPKI, montage, arbre web, verrou cooperatif et refus si connexion active.

Le staging root:root `0755` est verifie avant arret. La sauvegarde privee hors ligne est validee entree par entree (chemin relatif, type, taille, SHA-256, SQLite `quick_check`, ensemble exact). Seuls les sidecars SQLite connus de tailles attendues peuvent etre deplaces dans une quarantaine root recuperable. Le rollback conserve un ancien binaire de hash connu ; toute erreur apres arret restaure ou conserve uniquement un hash connu, redemarre et exige la sante HTTPS stricte.

Le script ne comporte aucune ecriture sous `/usr/lib/chillcenter/web`, dans la configuration/TLS/unite, le NAS ou le firewall. Il reverifie apres publication configuration, unite, SPKI, montage, processus et l'arbre web (contenu, inode, mode, proprietaire, taille, mtime). Le coordinateur a fourni la preuve read-only que les chemins concernes sont sur `/dev/mmcblk0p2`, avec environ 9 Gio libres pour environ 85 Mio de donnees ; le risque de staging/sauvegarde insuffisants releve dans le rapport precedent est donc leve pour cette execution.

## Verdict et conditions

Source `02b6b8e...80ce3` : **CHANGES_REQUESTED — NE PAS DEPLOYER**, avec un finding **MEDIUM** bloquant. Un nouveau SHA corrigeant l'ambiguite de selecteur et sa regression ciblee exige une nouvelle revue delta.

Operateur `62081da7...7f06e` : **APPROVED_SCOPED**. Il ne peut etre execute que lorsque la source de remplacement a recu les gates independants, avec un binaire ARM64 reproductible dont le hash est passe en argument et correspond exactement au fichier entrant. La verification de release devra encore constater les suites/Clippy verts, executer l'operateur une seule fois, puis verifier depuis le LAN les ports publics 80/443, les alias API, le certificat epingle, le hash binaire, le rollback et l'invariance des fichiers web. Toute modification du script rend son approbation stale.
