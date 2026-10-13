# Preuves CC-UI-001 — interface Mac raccordée au serveur

2026-10-13 ; source figée `c2a9d72d634cc7282eb8a20ca9fa138ed92e2fec`, base `fe85cea00da8a7e1f35abf0a5f057907eb235b47`, branche `codex/interface-server`. Auteur `/root`. Lot DEV_STANDARD autorisé par « continue l'implémentation […] raccorde l'interface au serveur ». Ce lot ne clôture pas G-04/133 AC.

## Résultat et configuration

Application Tauri Mac Apple Silicon → commandes allowlist Rust → HTTPS/SPKI → serveur Rust/SQLite. Adresse DNS/IP/port editable ; première confiance ou refus, dernier profil par serveur et par Mac ; CRUD/CAS/confirmation des profils, administration zéro profil ; dossier Linux editable et scan **exclusivement dans Administration**, catalogue paginé avec filtres/grille/liste, notes/vu manuels isolés et préférences individuelles. « Actualiser » recharge seulement le catalogue, pas le NAS. Cette séparation confirme la demande ajoutée par l'utilisateur le 2026-10-13, illustrée par la barre Cinéfolio.

Racine `/srv/volumes/video` et IP `192.168.1.42` restent des exemples, pas des constantes. Aucun accès NAS côté Mac ; aucune vidéo/fichier source Cinéfolio modifié. Les chemins absolus médias restent au serveur ; le catalogue expose des chemins relatifs. Les profils n'offrent pas de confidentialité/authentification, conformément au réseau familial approuvé.

## Exécution locale

Environnement : macOS27.0.1/arm64, Rust1.99.0, Tauri2.12.3, reqwest0.13.5, rustls0.23.45. Build isolé `/private/tmp/chillcenter-interface.dbGN1v/repo`, cible `/private/tmp/chillcenter-implementation.6HSJzC/repo/target`. Variables : `CARGO_HOME=/private/tmp/video-catalog-cargo`, `RUSTUP_HOME=/private/tmp/video-catalog-rustup`, `PATH=/private/tmp/video-catalog-cargo/bin:/opt/homebrew/bin:/usr/bin:/bin`. Tous les Rust checks utilisent `--locked --offline` ; reviewers avec cible distincte.

- `cargo test --workspace --all-targets --offline --quiet` : PASS124/124,0ignored (catalogue65,client18,domaine11,lecteur10,serveur20) avant gel du candidat, miseàjour mécanique du lock pour la dépendance de test rusqlite déjà présente. Le reviewer revalide avec `--locked` au SHA exact ; résultat dans rapport r3.
- `cargo test --locked --offline -p chill-server --lib --quiet` au SHA final : PASS20/20. Filtrage discriminant51films/2épisodes, filtre croisé vide, UUID média employé commeprofil et inverse, profil supprimé refusé pour catalogue/préférences/vu/note.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` : PASS ; JavaScript `node --check ui/connected/app.js` et `qa.js`, `bash -n scripts/package-macos.sh`, `git diff --check` : PASS. Revérifications reviewer exact-SHA dans rapports r3.
- `cargo build --locked --offline -p chillcenter-desktop -p chill-server` : PASS au SHA final.
- Regressions BUG-0004 type TLS, BUG-0005 rollback sélection sur GET/persistance, BUG-0006 tâches annulées/expirées avant et après démarrage et lecture WAL pendant transaction2000vidéos, BUG-0007 rollback snapshot entier par trigger d'identité, BUG-0008 lecture SQL/JSONdéfaillante distinguée de médiaabsent sur HTTPnotes : PASS. Échec de connexion initiale ne conserve aucune confiance provisoire en mémoire/disque. Les bugs étaient identifiés par expérience ou revue statique, distinctions conservées dans chaque fiche.

## Vraie webview / HTTPS — pas de mock browser

Serveur : `target/debug/chill-server --config /private/tmp/chillcenter-ui-delivery.9F69kK/server.json`, adresse loopback `127.0.0.1:22443`, données/vidéos/client uniquement dans cette fixture neuve. Toutes les requêtes ont utilisé le vrai vérificateur TLS, sans option de bypass ni curl insecure.

Commande de base littérale :

```sh
CHILL_UI_QA_URL=https://127.0.0.1:22443 CHILL_UI_QA_DATA=/private/tmp/chillcenter-ui-delivery.9F69kK/client CHILL_UI_QA_ROOT=/private/tmp/chillcenter-ui-delivery.9F69kK/videos /private/tmp/chillcenter-implementation.6HSJzC/repo/target/debug/chillcenter-desktop
```

1. Sans mode : sortie `UI_QA_RESULT true connect-consent-profiles-library-scan-personal-state`, exit0 (session68416). Actions DOM : refus/acceptation du certificat, bootstrapprofil, chemin, scan, catalogue, vu/note5/préférencesfr/autoplayoff A, renommage CAS, Binitialen/nonvu/sansnote.
2. Même commande avec `CHILL_UI_QA_MODE=reopen_delete` : `UI_QA_RESULT true reopen_delete`, exit0. DernierBrestauré, annulation suppression, suppressionA puis dernierB, administration accessible zéroprofil, créationC/catalogue commun conservé.
3. Même commande avec `CHILL_UI_QA_MODE=renewal_notice` : `UI_QA_RESULT true renewal_notice`, exit0. **Simulation dans les seuls réglages de test** : ancien pin modifié d'un byte et date d'expiration connue miseà1. Le handshake avec le vrai certificat courant émet l'événement d'exception ; bandeau séparé visible après chargement catalogue et Refresh administration. Ce n'est pas une attente réelle de18mois ni une rotation physique de certificat serveur.

Le `.mp4` est une fixture textuelle pour discovery/parser uniquement : **aucune preuve de lecture vidéo n'est revendiquée par ces trois parcours**. QA n'est déclenchée qu'en debug avec variables explicites et répertoireisolé obligatoire ; hook natif absent de release, pas de JSQAchargé parindexHTML.

## Paquet Mac de développement

```sh
CARGO_HOME=/private/tmp/video-catalog-cargo RUSTUP_HOME=/private/tmp/video-catalog-rustup CARGO_TARGET_DIR=/private/tmp/chillcenter-implementation.6HSJzC/repo/target CARGO_NET_OFFLINE=true PATH=/private/tmp/video-catalog-cargo/bin:/opt/homebrew/bin:/usr/bin:/bin bash scripts/package-macos.sh '/private/tmp/chillcenter-interface.dbGN1v/Chill Center-r3.app'
```

PASS, release2m48s ; plistlintOK, signaturead-hoc `codesign --verify --strict --verbose=2` valide, Mach-Oarm64,9.8MiB ; `otool -L` uniquement frameworks/bibliothèquesmacOS, aucune dépendance Homebrew/libmpv pour ce lot sanslecteur. `strings` : marqueur natif `UI_QA_RESULT` absent. SHA256 du binaire signé : `6aad62ca783248d365f84c7df8322de44f0b2d36bfce35e49ebae0617b0f1102`. Signature de développement, **pas notarized**, bundlelibmpvfinal encore à livrer. Pas de parcours GUI release indépendant exécuté ; les trois preuves headed portent sur la même source debug.

## Limites et suivi

Les trois revues r3 sont **APPROVED pour le lot borné** au SHA `c2a9d72d634cc7282eb8a20ca9fa138ed92e2fec` : [code](../reviews/INTERFACE-CODE-REVIEW-r3.md), [tests](../reviews/INTERFACE-TEST-REVIEW-r3.md), [sécurité](../reviews/INTERFACE-SECURITY-REVIEW-r3.md). Vérifications indépendantes : suite124/124, clippy strict, build locked/offline, paquet/signature. Les rapports r1 à b8737bb et r2 à d361eba sont conservés, jamais réécrits. Code r2 CHANGES_REQUESTED (BUG-0008), test/sécurité r2 limités à leur SHA ; pas des approbations de la version finale.

LOW restants : UISEC-002 (fsync parent des réglages clients), SRV-SEC-005 (réconciliation mémoire/disque après fsync serveur). Propriétaire `/root`, à traiter avant remise globale. UISEC-003 (rollback de confiance) corrigé et vérifié dans r3. Les processus serveur/app de test ont été arrêtés après preuve ; leurs fixtures privées sont conservées pour audit.

Remise locale : dossier `/Users/ghillaumesoubigou/Documents/Dev/Perso/ChillCenter`, branche `codex/interface-server`. Le commit de documentation final porte uniquement docs/README/manifeste, descendant direct du SHA source revu. Le bundle ignoré est copié dans `artifacts/macos/Chill Center.app`, puis son hash et sa signature sont vérifiés. La remise est conditionnée à ces contrôles, sans modification des sources Cinéfolio ni écrasement de travaux existants. Aucun push GitHub.

NOT_READY : lecteur Tauri/HTTPSmédias/sous-titres/baux/progression/enchaînement, métadonnées/affiches/édition/collections raccordées, IA, import/export complet, ambiances. NOT_RUN : LinuxARMrelease/systemd/Pi/NAS/Wi-Fi/2–3flux/budgetsCPU/RSS. Aucun déploiement physique, push GitHub ou action sur réseaufoyer. Pi toujours déclaré non prêt.

Retour : ancien checkpoint `fe85cea` et ancien binaire, sauvegarde base/configuration avant future migration. Ce lot ne migre/détruit aucune vidéo ; il ajoute des écritures de profils/configuration selon les actions explicites. Script paquet refuse d'écraser unbundleexistant. Données de preuve isolées conservées pour audit ; processus de test arrêtés après exécution.
