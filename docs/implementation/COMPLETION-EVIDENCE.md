# Remise logicielle CC-COMPLETE-001

2026-10-14 · périmètre CC-STORIES-001,33 stories/133 critères stables · G-03 approuvé · G-04 prêt à remise après l'artefact final, pas une validation déjà obtenue. Source finale : `126da0af167ee62588d92d90297f2dfc51962676`, base utilisateur `2a085486f4579d4cf27c98b254bd29fea9d73547`. Les commits documentaires ultérieurs ne changent pas les sources revues. Aucun push ni installation Pi/foyer.

## Livrable

Client Apple Silicon Tauri avec lecteur **natif libmpv**, image AppKit/FBO, flux Rust HTTPS épinglé et Range bornés ; aucune URL ou racine NAS libre dans l'IPC. Choix et mémoire du serveur/profil, contrôles/pistes/sous-titres, reprise synchronisée, vitesse par série, crédit vu après10 secondes réelles dans les derniers10%, transfert confirmé entre appareils, suite et compte à rebours. Le serveur ne fait aucun transcodage vidéo.

Le décodage n'est pas réinventé : libmpv embarqué dépend de libavcodec/libavformat/libavutil/libswresample/libswscale de FFmpeg (liens `@loader_path` vérifiés par otool). Le code Rust spécifique est l'adaptateur d'intégration/session/HTTPS/contrôles ; [mpv est conçu pour être embarqué](https://mpv.io/), [FFmpeg fournit les bibliothèques multimédias](https://ffmpeg.org/about.html). Aucun processus ffmpeg serveur.

Catalogue groupé/paginé, détails et métadonnées communes, TMDB avec sélection explicite, affiches personnelles et de saga, notes/vus/playlists individuels et ordre canonique. Administration : scan seul, configuration de racine, retrait des indisponibles, identité et rapprochement partiel confirmé/full unique, archives avec aperçu/conflits/idempotence ; CLI Cinéfolio en lecture seule. IA côté client avec aperçu/confirmation/CAS, annulation, petit bouton sans dictée. Thèmes/ambiances vectorisées automatiques/manuelles/désactivables. Serveur Linux : sources, paquet/build explicite ARMv7/aarch64, unité systemd et runbook ; sauvegarde/restauration offline avec verrou/journal/récupération.

## Traçabilité de preuve par story

[AC-MATRIX.json](AC-MATRIX.json) conserve les133 IDs et leurs chemins d'implémentation. C'est un inventaire de portée, **pas133 tests d'acceptation exécutés**. Le tableau ci-dessous rattache la portée locale et les limites ; les fichiers story approuvés sont conservés comme baseline historique, sans réécrire les Given/When/Then. Une preuve de règle/DOM n'est pas un essai sonore humain ni un benchmark Pi.

| Story | Preuve locale consultable | Limite d'acceptation restante |
| --- | --- | --- |
| US-001 | INTERFACE-EVIDENCE : refus/accord SPKI, connexion webview→Rust→HTTPS ; tests client | DNS/LAN du foyer non testé |
| US-002 | Même parcours : créer/renommer/validation des noms, bootstrap sans profil ; domaine/API CAS | Aucun rôle d'autorisation revendiqué |
| US-003 | Réouverture/dernier profil, isolation, fermeture lecteur lors changement ; bridge | Deux Macs physiques non testés |
| US-004 | Réouverture/suppression confirmée/annulée/zéro profil ; tests erreur et révocation | Suppression pendant suspend OS non testée |
| US-005 | Certificat réel + exception simulée ; tests TLS/signature/horloge/SPKI | Pas18mois réels ; risque imposteur après expiration accepté |
| US-006 | Configuration/scan réel fixture, snapshot transactionnel, scan partiel/annulé et préservation | NAS/Wi-Fi réel non testé |
| US-007 | Tests SQL groupements/filtres/recherche/genres/année/tri + DOM query + catalogue Tauri | Grande bibliothèque réelle non benchmarkée |
| US-008 | Domaine/navigation/compteurs/règles numéros inconnus ; catalogue Tauri | Corpus séries réel non importé |
| US-009 | Métadonnées commune série/vidéo, CAS/conversion/saga ; BUG-0024 | Fusion manuelle foyer non exécutée |
| US-010 | Décodeur borné/affiche immuable/thumbnail/choix explicite et préservation custom ; bridges | TMDB réel/token et sélection de fichier humaine non testés |
| US-011 | SAGA-ARTWORK, contrôleurs membre/source/CAS ; BUG-0034 ouvre affiche avant membres | Validation visuelle humaine de saga à faire |
| US-012 | Indisponibles/protection/retrait atomique et réapparition domaine/API | Déconnexion NAS réelle non testée |
| US-013 | Notes vidéo/groupes/season/personnelles, nouveaux épisodes et divergence ; domaine/Tauri | Pas de confidentialité entre profils (sans auth) |
| US-014 | Création/ajout borné/annulation/isolation ; domaine, contrôleur103médias/48page sans trous et sélection conservée BUG-0037 | Corpus foyer non utilisé |
| US-015 | Ordre canonique/CAS/filtre/vue + tests DOM playlist ; Tauri archives préserve ordre | Déplacement multiappareil pendant suspend non testé |
| US-016 | NATIVE-FORMATS-EVIDENCE : vraie image FBO MKV/MP4/AVI, pistes ; QA player/fullscreen | Son/sous-titres interprétés par humain, HEVC et perte NAS physique non testés |
| US-017 | QA native seek, sept vitesses/contrôles ; bornes et Range unité | Gestuelle trackpad/humaine non testée |
| US-018 | Initialen/en, préférencepersonnelle/isolation/persistance API/UI | Langues du corpus foyer non testées |
| US-019 | Deux audio anglais/français, ASS intégré/externe, SRT externe natif ; ja/Japanese absent et deuxfr ambigus vraisTauri ; testuntaggedavantoriginale BUG-0039 | VTT couvert par serveur/parseur ; externe illisible E2E et rendu humain non testés |
| US-020 | Sept vitesses natif, film reset1x ; nouveau season/série dernière1,5x ; isolation domaine | Autres Macs physiques non testés |
| US-021 | Reprise Tauri + second processus/ACK ; progression génération liée et échéance | Coupure Wi-Fi et suspend/reprise OS réels non testés |
| US-022 | Seuil/cumul/pause/buffering/seek/vitesse déterministes, QA crédit jusqu'à EOF ; BUG-0012 | Aucun seek seul ne vaut preuve de vu ; durée inconnue reste prudente |
| US-023 | Deux vrais processus Tauri : refuser puis confirmer, A pause/SSE/ACK et B reprend ; BUG-0028 | Suspend/réveil OS réel non testé |
| US-024 | Tests directs playlist/saga réordonnée/nextindisponiblenonsauté/standalone/série numérosinconnus/nouvellesaison ; nouvelle saison Tauri, révision/génération | Pas panne NAS réelle |
| US-025 | Vrai contrôleur player DOM :0/5/10/20/30/off/now/cancel/unavailable/background ; parcours natif10s/nouvelle saison | Chaque délai n'est pas une lecture GUI complète distincte |
| US-026 | FINGERPRINT-EVIDENCE + NATIVE-IDENTITY-EVIDENCE : probable confirmé et unique full, UUID/notes/vu/ordre conservés ; BUG-0030/33/35/36 | Déplacement NAS et buildARMv7 réel non testés ; modification entièrement inobservée indétectable |
| US-027 | EXPORT-JOBS/ARCHIVE-API : cohérence snapshot, limites, fichier streaming, ZIP réel Tauri aller-retour | Archive foyer non exportée |
| US-028 | CINEFOLIO-EXPORT/ARCHIVE-API/NATIVE-ADMIN : CLI lecture seule, mapping explicite/conflicts/idempotence/atomicité et annulation | Migration réelle foyer non exécutée ; source vidéo intacte |
| US-029 | OPERATIONS : backup/restore/recover offline, injection crashes/journal/ancien arbre sauvegardé | Panne électrique/systemd/Pi réels non testés |
| US-030 | NATIVE-ADMIN-AI-EVIDENCE + AI-LIFECYCLE-TESTS + vrai contrôleur DOM race/CAS/profile | CLI synthétique ; fournisseur réel/qualité/coût/termination externe non prouvés |
| US-031 | appearance Rust/JS et préférences locales automatic/light/dark | Inspection accessibilité humaine de toutes vues non exécutée |
| US-032 | Calendrier déterministe saisons/Halloween/Noël/limites, SVG commun, horloge locale | Réveil OS traversant une date réelle non testé |
| US-033 | Tests override/disabled/automatic/isolationlocale + contrôles UI | Autre Mac physique non testé |

## Vérification du candidat

Commandes avec Rust1.99.0, macOS arm64, Cargo.lock verrouillé, cache hors-ligne. `CARGO_TARGET_DIR=/private/tmp/chillcenter-root-target` propre à ce clone ; ne jamais réutiliser l'ancienne cible partagée de plusieurs clones. Résultats finaux et revue indépendante à rattacher ici avant promotion :

- `cargo test --workspace --all-targets --offline --locked --quiet` au SHA126da0a :255/255 PASS, aucun ignoré (67catalogue,30client,50domaine,15lecteur,71serveur,2CLI,20desktop), session47660 exit0. La première invocation historique confinée a refusé les deux listeners loopback des tests adversariaux libmpv ; ce refus de sandbox n'est pas compté commePASS. Nouvelle invocation autorisée PASS, sans changement de test ni bypass TLS.
- `cargo clippy --workspace --all-targets --offline --locked -- -D warnings` : PASS sans avertissement.
- `node --test ui/connected/tests/*.test.js ui/connected/*.test.js` :24/24 PASS, aucun ignoré ; depuis42050 aucun contrôleurprodNode changé, seulement harnaisdebugformats contrôlé parnodecheck.
- `python3 scripts/helpers/test_linux_package.py` :4/4 PASS (un test utilise ELF factice, **pas un build Linux**).
- `python3 scripts/helpers/test_publish_macos_bundle.py` :3/3 PASS, dont refus d'écrasement/race.
- Native post BUG-0032 : `UI_QA_RESULT true native-Tauri-MKV-MP4-AVI-integrated-audio-ASS-external-ASS-seven-rates-seek-film-reset` ; client privé `/private/tmp/chillcenter-formats-secure.7QXVCJ`.
- Identité native préparation + finition PASS, client privé `/private/tmp/chillcenter-identity-roundtrip.QHXlTU` ; deux fichiers synthétiques déplacés, notes/vus/playlist conservés.
- Garde native independent-deadline avec renderingoff, slot UI et mutex host retenus PASS (BUG-0028).
- Regressions0034 ouverture saga,0035 bornes>4GiB/32bit,0036 récupération au restart : PASS ciblés.
- Regressions0037 pagination48/50 et0038 notesanonymes : RED→GREEN.0039 validkeyabsente puisnon-normalisableJapanese : RED→GREENproductionhelper etGUIja/Japanese/fr2externes PASS, client `/private/tmp/chillcenter-final-invalid-language.18yFTy`.

La sonde debug de vivacité JS utilisée quand document.hidden est vrai réveille les timers ; elle ne poll ni actionne le lecteur/heartbeat. Absente du release. Cela ne prouve pas un suspend système. Les hooks QA choisissent seulement des fixtures explicites et ne sont pas chargés par l'index livré.

## Artefact, exploitation, retour

Bundle source126da0a construit et vérifié dans `/private/tmp/chillcenter-final-126da0a.11obnd/Chill Center.app` :75MiB,48dylibs arm64/41formules, build release locked/offline PASS1min02 (deux warnings QA-only nonblank/glReadPixels connus), plist/lipo/closure/otool/signaturead-hocPASS. `--verify-bundle` PASS : bibliothèque locale embarquée/API2.5, aucune UI/config/réseau ouverte. Binaire signé SHA256 `865a2210ccb80bd28f943db3102cad290d4c8bbe07de65455d77c2444d384f80` ; libmpv/icône/notices identiques aux hashs ci-dessous. Remise prévue dans `artifacts/macos/Chill Center-126da0a.app`, via stagingprivé et publicationatomiqueNO_REPLACE. Ancien `Chill Center.app` et paquetsded8/42050 conservés, jamais remis comme version corrigée. Pas de notarisation, secondMac sansHomebrew ou GUIrelease non instrumentée revendiqués.

Preuve historique premier bundle complet `ded8b297` dans `/private/tmp/chillcenter-final-bundle.dVzEwZ/Chill Center.app` :75MiB,48dylibs arm64 de41formules, aucune ancienne application écrasée. Cargo release locked/offline PASS (4min, deux warnings dead_code QA-only nonblank/glReadPixels déjà connus). Signature ad hoc `codesign --verify --deep --strict` PASS, plist/lipo/otool fermeture relocalisée PASS, libmpv embarqué API2.5 PASS. Pas DeveloperID/notarisation. Notices/sources/licences tierces présentes ; ce bundle n'est pas déclaré MIT-only ni autorisé juridiquement pour distribution publique. `--verify-bundle` a retourné `BUNDLE_LIBRARY_OK` avec le chemin Contents/Frameworks uniquement puis quitté **avant** Tauri/réglages/réseau ; aucun réglage du foyer lu pour ce smoke.

SHA256 des octets signés : binaire `c008d8b13ff41e046b7c3c66df58d01fd78e1002261ff82fb23d880ebc084d71`, libmpv `a548453541616ee85970fd31da1aeafd7c7bf501a552e63502f6d69abbac31d7`, icône `df2685a364690fa7e5f29017a0fcd00481cf830bda74fe76a76b35a607f075b6`, notices `ca8dfb304d7db7228994d9ffadfcbf24febfd43ffbaed2eb19f0b3afb1efc69f`. Commande : `CARGO_NET_OFFLINE=true CARGO_TARGET_DIR=/private/tmp/chillcenter-root-target bash scripts/package-macos.sh '/private/tmp/chillcenter-final-bundle.dVzEwZ/Chill Center.app'` avec les mêmes cache/PATH que la suite. Second Mac sans Homebrew et parcours graphique du release sans harnais : NOT_RUN.

Linux : [LINUX-PACKAGING](LINUX-PACKAGING.md), exemple `/var/lib/chillcenter/data`, IP/racine configurables. Les cibles Rust/linker ARM ne sont pas disponibles ici, Docker n'a pas de daemon : **buildARMv7/aarch64 et systemd NOT_RUN**. Mise à jour utilisateur14 octobre : Pi/Linux et NAS prêts, Chill Center non installé ; utilisateurSSHadmin. Tentative autorisée lecture seule admin@192.168.1.42 : Pi répond, puis `sign_and_send_pubkey: signing failed` et `Permission denied (publickey)` ; aucune commande distante exécutée, aucune installation. Configuration SSH locale désigne agent1Password ; session de contrôle privée proposée pour accès sans transmettre de secret. Aucune confiance SSH contournée, aucun portLAN/compte/secretNAS/cloud/fournisseur payé créé. Vérifications matérielles restentNOT_RUN tant que connexion/readiness n'est pas prouvée.

Retour réversible : conserver l'ancien bundle et la branche/checkpoint2a085486. Tester un ancien binaire dans une nouvelle branche, **pas de reset des fichiers utilisateur**. Les nouvelles tables privées portent user_version3 ; pour revenir à un binaire incompatible, restaurer une sauvegarde pré-migration cohérente service arrêté/même UID. Le rollback ne supprime jamais les vidéos ; les opérations restaurantes conservent arbres précédents et sauvegarde de sécurité. Les fixtures/tmp et anciens rapports restent récupérables.

Revues indépendantes exactSHA126da0a : codeAPPROVABLE, testsAPPROVABLE_SCOPED, sécuritéAPPROVABLE dans [COMPLETION-FINAL-126da0a](../reviews/COMPLETION-FINAL-126da0a.md). AucunP0/P1/P2 nouveau. Les rapports initiaux CHANGES_REQUESTEDded8/42050 restent consultables. La suite255 finale, qui était encore en cours au verdict test, est désormais réellementPASS. La validation globale des133 AC n'est pas revendiquée ; G-04 demandera acceptation de la remise logicielle avec réserves matérielles explicites, après artefact/promotion.

## Promotion locale et essais matériels restants

Remise effectuée et vérifiée : [DELIVERY-RECEIPT](DELIVERY-RECEIPT.md). Fast-forward local vers484be027, bundle atomique `Chill Center-126da0a.app` publié sans écrasement, signature/loader/hash/manifeste PASS après copie. Les formulations de commande et de conditions ci-dessous restent le protocole de reproduction, pas des actions encore en attente.

Promotion autorisée de ce carrier documentaire (sources identiques126da0a) par fast-forward depuis2a085486 uniquement si le répertoire utilisateur est encore propre et surcodex/player-streaming. Aucun checkout/reset/main/push. Vérifier signature/hash du bundle copié et manifestes après promotion ; arrêter seulement le serveur loopback synthétique identifié. Les artefacts/tests/sourceCinéfolio/données NAS du foyer ne sont jamais effacés. Conserver l'ancien .app pour retour.

Pi prêt d'après l'utilisateur, produit non installé : reste à vérifier OS/architecture32ou64bits et RAM, lecture NAS par UIDservice, buildELFARM correct et dépendances, installation Linux explicite avec configurationvariable/horloge/TLS/firewallLAN/systemd, puis deux/trois flux représentatifs pendant30min (seek/pistes, buffering/débit/RSS/CPU/mémoire/température), Wi-Fi et déconnexion/reconnexionNAS, restart/reboot/backup/restore/récupération sans vidéos supprimées. Aucune de ces opérations n'est déclaréePASS. AccèsSSHread-only en attente de socket partagé par l'utilisateur ; une autorisation d'inspection ne vaut pas installation du service ou modification du montage.
