# REL-0.7.1 — vérification indépendante de release

**Statut : APPROVED_SCOPED (artefacts publics et Pi contrôlés)**

**Vérificateur : instance indépendante, distincte des auteurs, réviseurs et opérateur**

**Moment : 2026-10-21**

**Périmètre courant :** artefacts publics, intégrité/signatures, et déploiement Pi déjà réalisé. Les checkpoints prépublication ci-dessous sont historiques. Aucune modification distante ou locale de runtime n'a été faite par ce vérificateur.

## Référence vérifiée

La source immuable fournie est exactement `c603f29db2936ee40f7bf66484187b4e237d9bfd`. Les revues candidates code, sécurité et tests consultées approuvent ce même SHA. Les preuves consignées par la revue tests sont : Rust `443 PASS`, `2 ignored` NFR historiques (non comptés comme passes), Node `323/323 PASS`, navigateur `8` rendus/échecs contrôlés PASS, et opérateur `31/31 PASS`.

Ces résultats admettent la source au packaging ; ils ne constituent ni preuve de publication ni preuve de déploiement.

## Contrôles des artefacts présents

| Élément | Résultat indépendant | Observation |
| --- | --- | --- |
| `chill-server` | PASS | ELF PIE Linux ARM64/aarch64 ; SHA-256 `8cf14421630cfd552e3af4ed340de107c0570046cc5ecb2784936ca8c552491c`, identique à la valeur fournie. |
| `chill-update` | PASS | ELF PIE Linux ARM64/aarch64 ; SHA-256 `b9d522381e5afe93ffdfb3beefecb5de1c0455df48dcbe0b7581ebbc99dfd281`, identique à la valeur fournie. |
| App Mac de staging et `.app.tar.gz` | PASS | `diff -rq` de l'arbre extrait contre l'app de staging : vide ; archive intègre, sans AppleDouble ; signature `codesign --verify --deep --strict` valide. |
| Métadonnées Mac | PASS | App `0.7.1`, build `10`, `LSMinimumSystemVersion=27.0`, 49 Mach-O arm64. Certaines bibliothèques déclarent un minimum Mach-O antérieur ; cela n'abaisse pas le minimum déclaré de l'application. |
| DMG | PASS | SHA-256 local `5b281dd9c4847c2eac5f9505da64b5e538e06305a76f2d5230cfd265aee83a14` ; `hdiutil verify` PASS. Montage isolé lecture seule réalisé, app montée identique à l'app de staging (`diff -rq` vide), signature stricte PASS, lien `Applications` et instructions présents ; volume éjecté après contrôle. |
| Web ZIP | PASS | Test d'intégrité ZIP PASS, 112 entrées, aucun chemin dangereux ; arbre extrait identique au répertoire web assemblé, `build.json` version `0.7.1`, build `e496ef4b7ca6`, 101 fichiers. |
| Thèmes ZIP | PASS | Test d'intégrité ZIP PASS, 23 entrées, aucun chemin dangereux ; arbre extrait identique à `themes/default` de la source. `renderer_version=2`; les 18 fichiers déclarés ont tailles et SHA-256 exacts. Les ressources inspectées sont des PNG réels. |

## Distribution signée

Le jeu de clés public est identique dans les quatre composants et la livraison.
Les signatures Minisign vérifient les quatre manifestes, `PROVENANCE.json`, les
quatre archives de livraison, les trois scripts installateurs et les signatures
des composants bruts. Les commentaires fiables des manifestes lient chacun le
composant à `c603f29db2936ee40f7bf66484187b4e237d9bfd`.

`SHA256SUMS` vérifie l'intégralité de son inventaire. Chaque entrée `assets`
des quatre manifestes correspond exactement, en taille et SHA-256, au fichier
signé local. Les trois wrappers manuels ont été ouverts et leurs copies du
serveur/web/thèmes, manifestes, signatures, clé et scripts ont été comparées
octet à octet aux entrées signées ; aucun chemin dangereux n'a été trouvé.

Le wrapper serveur contient les unités attendues. `chillcenter.service` est la
seule unité qui ajoute `AF_NETLINK`, avec le commentaire qui borne son usage à
la découverte d'interfaces/Bonjour ; le service de mise à jour conserve
`AF_UNIX AF_INET AF_INET6`. Le checkout source est propre, sans fichier non
suivi. Aucun marqueur de clé privée, mot de passe ou secret n'a été trouvé dans
les fichiers de livraison inspectables.

La signature Tauri encodée dans le manifeste Mac est le même Minisign que celui
fourni pour l'archive updater ; sa vérification reste incluse dans le contrôle
du couple archive/signature. Les signatures sont ad hoc et l'application n'est
pas notarised Apple, conformément à la provenance : cela n'est pas présenté
comme une notarisation.

## Checkpoint prépublication historique

Les artefacts signés ont été approuvés avant publication. Les contrôles post-déploiement et les téléchargements publics décrits plus bas ont ensuite été réellement exécutés ; ne pas lire ce checkpoint comme un état courant.

**Verdict de ce périmètre :** les artefacts signés sont approuvés pour la
création des brouillons de publication. Cette approbation ne vaut ni une
publication effective, ni un déploiement, ni la clôture indépendante des
constats sécurité externes encore suivis par leur propriétaire. Aucun rollback
n'est recommandé : aucun changement de cible n'a été observé dans ce périmètre.

## Vérification post-déploiement Pi

Contrôles directs en lecture seule réalisés après le déploiement : le binaire
actif correspond au pin serveur, le service est actif et conserve son identité,
ses capacités vides et `NoNewPrivileges`; la famille ajoutée est `AF_NETLINK`.
Les invariants de configuration et de TLS contrôlés par empreinte sont inchangés.
L'API HTTPS stricte répond sur la route datée, expose OpenAPI et rejette la route
`v1` par une réponse JSON 404. Son identité annonce la version 0.7.1 et la date
API attendue; aucune analyse n'est en cours. L'intégrité SQLite, le schéma et le
nombre de médias correspondent aux invariants logiques attendus. Le helper et
son timer ne sont pas actifs.

La sonde CLI mDNS du checkout immuable a parcouru le multicast pendant quatre
secondes et a découvert une annonce Chill Center 0.7.1 avec un candidat HTTPS.
Elle ne se connecte à aucun candidat. La GUI Mac et le consentement réseau local
macOS n'ont pas été exécutés : **NOT_RUN**, et ne sont pas inférés de cette
preuve headless.

La sauvegarde fraîche a été retrouvée sous la racine de données autorisée ; son
répertoire privé et son manifeste requis sont valides. Le snapshot SQLite actuel
reproduit aussi le digest logique du snapshot de sauvegarde avec l'algorithme
exact de l'opérateur, y compris la table SQLite interne préexistante.

L'empreinte d'un ancien relevé externe, capturé avant le preflight final, ne
correspond pas au digest courant. Ce relevé n'est pas le baseline transactionnel
de l'opérateur : celui-ci est maintenu seulement en mémoire, comparé avant arrêt
et après redémarrage, et son égalité a passé pendant l'opération. Le digest
courant a été recalculé indépendamment avec la même méthode et correspond au
handoff privé post-opération. La preuve disponible établit donc que le
déploiement n'a pas modifié le web ; elle ne permet pas de rétrodater l'ancien
relevé externe comme baseline transactionnel. Aucun NAS n'a été parcouru.

**Verdict post-déploiement : APPROVED_SCOPED.** Le scope couvre binaire,
invariants protégés, API/TLS, données, sauvegarde, web transactionnel et mDNS
headless. Il exclut explicitement la GUI Mac/consentement réseau local et toute
publication effective, qui est traitée séparément ci-dessous.

## Vérification des téléchargements publics

Les cinq tags publics sont annotés et pointent tous vers
`c603f29db2936ee40f7bf66484187b4e237d9bfd`. Les releases sont publiées, non
brouillon et non prerelease. GitHub désigne `v0.7.1` comme release courante ;
son inventaire contient 22 assets. Les inventaires composants sont cohérents :
Mac 7, serveur 14, web 4 et thèmes 4 assets.

Les quatre manifestes publics vérifiés indiquent la séquence de release `2`,
la version `0.7.1` et la même source immuable ; aucune ancienne séquence n'est
présentée comme courante.

Les 22 assets de la release globale ont été téléchargés dans un dossier neuf,
puis comparés octet à octet à la livraison locale signée. `SHA256SUMS` et les
signatures Minisign de provenance, manifestes et quatre archives vérifient sur
ces octets publics. Les digest GitHub des assets composants correspondent aux
références signées locales, ce qui couvre leurs doublons sans les télécharger
une seconde fois.

**Verdict des octets publics : APPROVED.** Les contrôles GUI Mac et consentement
réseau local macOS restent **NOT_RUN**. Le scan CodeQL Rust le plus récent était
encore en cours lors de cette vérification : aucun succès de ce scan n'est
revendiqué ici. Les constats CodeQL ouverts et l'alerte Dependabot medium non
livrée restent suivis hors de ce gate d'intégrité de release.
