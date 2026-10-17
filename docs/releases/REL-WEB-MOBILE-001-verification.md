# REL-WEB-MOBILE-001 — vérification post-déploiement / post-deployment verification

**Verdict : APPROVED_SCOPED**

**Vérificateur :** instance indépendante, lecture seule
**Portée :** web mobile installé uniquement ; aucune écriture de profil ou bail, aucune lecture NAS/vidéo, aucun redémarrage, aucune modification de configuration, binaire ou client Mac.
**Horodatage de vérification (UTC) :** `2026-10-17T14:40:27Z`

## FR

### Identité et intégrité

- Candidat contrôlé : commit `98ee3e37e8b19c0d2e8e92991b3fb917eb0a13b8`, build `b1722f00c0a6`.
- L'archive candidate conserve le SHA-256 attendu `281cf6eb6babca616004f425ac9e13332f38ef9a3af59f7793398640c81cc0ed` (6 616 576 octets).
- Les 81 entrées de somme du manifeste candidat sont toutes valides contre l'arborescence web installée ; 82 fichiers web sont présents (88 membres TAR, dont répertoires).
- Le différentiel TAR initial ne relève que des UID installés et deux modes WASM, pas de différence de contenu. La vérification SHA-256 du manifeste est la preuve de contenu retenue.

### Sauvegarde et retour arrière

- Sauvegarde privée présente à `/root/chillcenter-rollback/predeploy-mobile-20261017T143553Z`, permissions `0700`.
- Archive de pré-déploiement : SHA-256 `e340656ef3fdae44ed80aaca2e10c35a8cbdf8fb26bfade0a67f976ea470bca4`, 85 membres, conforme à la remise.
- Le manifeste de sauvegarde valide l'arborescence ancienne retenue (sommes de fichiers vérifiées). Le retour arrière web reste donc disponible sans toucher au serveur, à la configuration, au TLS ou aux données.

### Smoke tests lecture seule

- HTTPS strict avec le certificat de confiance : page web `200`, API datée `200`, alias API v1 `200`.
- HTTP : page web et API datée sur le port applicatif `200`. L'auto-sonde depuis le Pi vers l'alias HTTP par défaut a produit `000`; ce seul résultat ne permet pas de conclure à son indisponibilité depuis le LAN, car une règle NAT entrante peut ne pas s'appliquer au trafic local sortant. La vérification LAN indépendante du coordinateur a ensuite confirmé `200` et le build attendu sur cet alias.
- MIME : WASM `application/wasm`, JavaScript `text/javascript; charset=utf-8`.
- Sécurité : `/.env` renvoie `404`; requête API marquée cross-site renvoie `403`.
- Mobile : le document servi contient `viewport-fit=cover`; les fichiers mobiles attendus sont présents.

### Invariants serveur

- Service actif, même PID `5420`, zéro redémarrage.
- Les SHA-256 du binaire, de la configuration, de l'unité et du certificat, ainsi que le SPKI, sont identiques au baseline protégé fourni. Les valeurs sensibles ne sont pas reproduites ici.
- Avertissement/erreur/panic/failure post-déploiement : aucun résultat dans le journal de service borné.

### Limites explicites

La validation couvre les ressources et contrats web déployés. Elle ne valide pas le rendu physique sur iOS ou Android, les gestes/taille d'écran, ni la lecture vidéo réelle. Ces essais restent humains et hors périmètre de cette vérification sans écriture.

## EN

### Identity and integrity

- Verified candidate: commit `98ee3e37e8b19c0d2e8e92991b3fb917eb0a13b8`, build `b1722f00c0a6`.
- The candidate archive retains the expected SHA-256 `281cf6eb6babca616004f425ac9e13332f38ef9a3af59f7793398640c81cc0ed` (6,616,576 bytes).
- All 81 candidate-manifest checksums validate against the installed web tree; 82 web files are present (88 TAR members including directories).
- The initial TAR comparison reported installed UID values and two WASM modes only, not content changes. The SHA-256 manifest check is the accepted content evidence.

### Backup and rollback

- The private backup exists at `/root/chillcenter-rollback/predeploy-mobile-20261017T143553Z` with `0700` permissions.
- Pre-deployment archive SHA-256 is `e340656ef3fdae44ed80aaca2e10c35a8cbdf8fb26bfade0a67f976ea470bca4`, with 85 members, matching the handoff.
- The backup manifest validates the retained old tree (file checksums verified). Web rollback remains available without changing the server, configuration, TLS, or data.

### Read-only smoke tests

- Strict HTTPS using the trusted certificate: web page `200`, dated API `200`, and v1 API alias `200`.
- HTTP: web page and dated API on the application port return `200`. The Pi-local probe to the optional default HTTP alias produced `000`; that result alone cannot establish LAN unavailability, because an inbound NAT rule may not apply to locally originated traffic. The coordinator's subsequent independent LAN verification confirmed `200` and the expected build on that alias.
- MIME: WASM is `application/wasm`; JavaScript is `text/javascript; charset=utf-8`.
- Security: `/.env` returns `404`; an API request marked cross-site returns `403`.
- Mobile: the served document contains `viewport-fit=cover`, and expected mobile files are present.

### Server invariants

- Service active, unchanged PID `5420`, zero restarts.
- SHA-256 values for the binary, configuration, unit, and certificate, plus SPKI, match the supplied protected baseline. Sensitive values are not repeated here.
- No warning/error/panic/failure result was found in the bounded post-deployment service journal.

### Explicit limitations

This approval covers deployed web assets and contracts only. It does not validate physical iOS or Android rendering, gestures or screen dimensions, nor real video playback. Those human tests remain outside this read-only verification scope.
