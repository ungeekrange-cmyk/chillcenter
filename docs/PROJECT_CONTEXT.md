# Contexte du projet

## Remise locale effective — Pi mis à jour

Interface/API source9277ac4 promue localement, bundle `artifacts/macos/Chill Center-9277ac4.app` et archive Linux disponibles ; [release](releases/REL-UX-API-001.md). Après confirmation explicite renouvelée, déployeur distinct et revue opérateur V4 APPROVED, Pi mis à jour le14octobre à19:12:40UTC : binaire1eb24… actifPID5145. Sauvegarde privée offline et rollbackR7 conservés surPi ; NAS/configuration/unité/TLS/profil foyer inchangés. [Vérification indépendante post-bascule APPROVED](releases/REL-UX-API-001-pi-verification.md), [gate](gates/G-POST-CC-UX-API-001.md) : processus/hash stable/NRestarts0, sauvegarde cohérente, TLS strict/API datée/v1/OpenAPI et propre sonde réelle du même clientRust queTauri connexion/catalogue48/reconnexion PASS. Sans écriture serveur ni accès aux réglages réels du Mac ; pas une validation humaine de la fenêtre/son ni une nouvelle lecture NAS. [BUG0051](debug/BUG-0051-pi-update-output-quoting.md) conserve la première tentative opérateur échouée et le redémarrage sûr deR7.

## Branche parallèle — client web PWA (`codex/web-pwa`)

14 octobre 2026, demande utilisateur : reproduire le client Mac en web app/PWA servie par le Pi, sans authentification, calcul vidéo côté navigateur, serveur inchangé hors delta minimal. Travail isolé dans un worktree distinct pour ne pas gêner le client Mac. [ADR-0012](architecture/decisions/ADR-0012-web-client-pwa.md) PROPOSED, [architecture web](architecture/WEB-CLIENT.md). Aucun déploiement, aucun push. Dépôt distant : `https://github.com/ungeekrange-cmyk/chillcenter.git`.

## Point actuel — interface/API approuvées, livraison coordonnée en cours

14 octobre2026 : interface Cinéfolio CC-UX-002 intégrée au vrai client Tauri, Administration regroupée, conseil filtré, catalogue continu virtualisé, fiches/séries/sagas/listes et configuration/enrichissement TMDB raccordés. API HTTPS canonique `/api/2026-10-14`, alias `/api/v1`, [guide tiers](api/INTEGRATION-FR.md) et OpenAPI public. Source exacte9277ac4 ; [triple revue indépendante APPROVED](gates/G-DEV-CC-UX-API-001.md), 289Rust/47Node/clippy et deux parcours lecteur natif visibles frais PASS. Les erreurs antérieures sont conservées, pas requalifiées.

Bundle ARM64 autonome75Mio et serveur Linux ARM64 compilés et livrés. Les anciens checkpoints R7 ci-dessous restent historiques, pas l'état du serveur après19:12:40UTC. Profil foyer et NAS préservés, pas de fournisseur réel/clé réelle ni de nouveau benchmark Wi-Fi. Reprendre les [preuves](implementation/UX-API-EVIDENCE.md) et gate, pas la découverte historique.

## Point de reprise actuel — 14 octobre 2026

Maquette r2 enrichie ensuite à la demande utilisateur : stats séries/saisons/épisodes non vus, navigation saisons→épisodes→fiche, badge Indisponible seulement, création et ajout dans sagas/listes. `docs/design/prototype-collections.js` est un module de démonstration **pas le runtime connecté** ; saga commune et liste personnelle, fixture fictive2saisons/5épisodes, créations mémoire. Playwright PASS et captures inspectées, filtres du conseil inclus ; gate r2 PENDING, aucun bundle ou service touché. Ne pas confondre ces parcours avec validation de la vraie lecture, vraie IA, ni du NAS.

Conseil r2 complété à la demande humaine avec filtres Tous/Vus/Non vus et Films et séries/Films/Séries. Ils sont indépendants du catalogue, se combinent et utilisent les états du profil actif dans l'application prévue. Maquette : exemples fixes filtrés, résultats retirés au changement de filtre, état vide explicite ; pas de vrai IA. Tests ciblés maquette ajoutés, gate visuel r2 toujours PENDING, client installé/Pi inchangés.

Retour visuel utilisateur : « Oui bien, mais » ; déplacer Recharger/Compléter les fiches/Organiser avec l'IA en Administration, sidebar « Demande de conseil » pour suggestions selon envies ; retirer slogan/nom à côté du logo et agrandir ce dernier. [Maquette CC-UX-002](design/prototype.html) mise à jour, logo96px bureau/48px étroit, parcours conseil en lecture seule distinct de l'organisation. [Spec](design/DESIGN_SYSTEM.md), [gate r2 PENDING](gates/G-UI-PROJECT-CC-UX-002.md), r1 STALE avec spec historique conservée. Accord de principe n'est pas validation après présentation de r2. Checks maquette PASS, vrais IA/vidéo NOT_RUN ; runtime Mac/Pi inchangé. Pas de modification du catalogue depuis conseil ; pour implémenter, réutiliser mode recherche IA existant et invalider résultats anciens après changement de profil/serveur.

Nouvelle demande UI : sidebar et recherche Cinéfolio, administration basse/serveur à l'intérieur, pas de pages, grandes fiches, clé TMDB/automatisme, commandes vidéo standard et accès IA visible. Diagnostic [BUG0045](debug/BUG-0045-client-ui-port.md) ; [maquette CC-UX-001](design/prototype.html) et [spec](design/DESIGN_SYSTEM.md), [gate visuel PENDING](gates/G-UI-PROJECT-CC-UX-001.md). Pas de correction applicative, bundle ou déploiement dans ce checkpoint. Continuer après confirmation visuelle avec un builder distinct du diagnostic lecteur ; ne pas annoncer une preuve DOM comme preuve AppKit native. Worktree isolé `/private/tmp/chillcenter-ui-rework.4oZp3D/repo`, branche `codex/cinefolio-ui-rework`, base8e83922. Serveur et bundle précédents restent inchangés.

G-01/02/03 et G-04 sont approuvés. L'utilisateur a accepté la remise logicielle avec réserves, puis autorisé explicitement l'installation sur le Pi et les tests NAS ; il a aussi approuvé le compte système dans le groupe existant marvin-admin984 après avertissement. Ne pas relancer discovery ou ajouter de l'authentification, un serveur Mac, du cloud ou du transcodage.

Chill Center est désormais installé et actif sur **192.168.1.42:8443**, Pi3B+Rev1.3/Debian13.7 ARM64/905MiB/Wi-Fi. Source fonctionnelle126da0a, binaire13MiB9a17ffe…36a6 ; archive immuable R7 et overlay WorkingDirectory corrigent le démarrage, compte997:983+984/nologin, configurationroot0640 et données0700. NAS //192.168.1.92/Video monté RO sur /srv/volumes/video, inchangé. SSH/Cockpit préservés. L'accès SSH se fait par le master privé /private/tmp/chillcenter-ssh.UAjtlb/control établi par l'utilisateur ; pas de clé ou mot de passe à copier, ne pas contourner un échec d'agent.

Revues indépendantes code/test/sécurité des scripts et overlay APPROVED, vérification post-installation distincte APPROVED_SCOPED (TLS strict, identité, confinement, service stable, repos≈11MiB, pas d'encodeur). [REL-PI-001](releases/REL-PI-001.md) lie chaque hash, étape, rollback sûr et bug0040–0044 ; la première installation a nécessité des corrections opérateur, pas de changement du CoreRust. Garder les rapports/versions antérieurs, pas de reset/suppression de NAS.

Scan complet réel2496vidéos/10,14s, premières lectures AVI/MKV/MP4 et reprise par second identifiant d'appareil PASS ; dimensions HD et SRT externe vérifiés sur flux HTTPS par FFprobe **Mac uniquement**. [Médias](releases/REL-PI-001-media-evidence.md) : deuxHD30min terminés avecstallsMKV ; troisfluxDune1/Dune2/720p30min **Dune2échoue progression<80%/bufferingimportant**, RSSPi24,23Mio/CPUmax22,95%d'uncœur/swap0. Exact2Dune10min supplémentaires PASSreprise/grant etprogression, Dune1buffer0/Dune2buffer184(≈20sdeficitdepuisdébut), paszérointerruption ni30minDuneàdeux. Pascausedémontrée/supposéeWi-Fi. TroisprofilsQA9be1667e supprimésà12:52UTCaprèsfins, aucunprofilpersonnel, SQL2496médiaconservés ; serveurPID3016/NRestarts0active. Pasdeprocessussonde laissé ni reboot/démontageNAS/panne physique autorisé implicitement.

Clone d'intégration /private/tmp/chillcenter-native-ui.KZRHlE/repo sur codex/pi-install ; cible utilisateur codex/player-streaming propre àc58da7eavantcarrierfinal. Préserver les changements avant toutepromotionfast-forward ; aucunpush/mainreset. **Clientsource624f244**, bundleMacnouveau`artifacts/macos/Chill Center-624f244.app`,75Mio48dylibsarm64/signaturead-hoc/loaderAPI2.5PASS. Demandeutilisateur60savant/30sretour puis réglageMio local128/64par défaut,16–512/0–256, [ADR0010](architecture/decisions/ADR-0010-client-playback-cache.md) acceptéparrevueindépendante624. Duréeretour variable/plafondsapproximatifs, pasguarantee60splein ; saveprochainlancement, jamaispréférencesprofil/serveur. 259Rust/27Node/clippyPASSaprèstentativeENOSPCconservée, code/tests/ADR/sécurité624APPROVED et postbundle/PiAPPROVED_SCOPED. [Preuve](releases/REL-MAC-CACHE-001.md)/[gate](gates/G-DEV-CC-CACHE-002.md). **Serveur126da0a/R7resteinchangé**, ancienbundle126conservé, pasredéploiementPi ; rollbackancienclientpeutperdrechampcachelocalàréappliquer. Harnais standaloneutilisedéfauts, nevalidepasformulairegraphique624. Historiquecidessousne décritpasfonctionsmanquantesactuelles.

Prochaines limites à traiter sanspromesseallUS/133ACDONE : fluidité3fluxnonvalidée, réseau/latenceNASàdiagnostiquer (unessaiEthernetseraunecomparaison, pascorrectiondéjàprouvée), tests humainsformulaireMac/son/rendu/veille/HEVC/multiplesaudio/backseek30sansrefetch/RSSmax768/backuprestorephysique/dataset3000. Aucun changementréseau/NAS/clock/restoredestructif implicite. Ne pas considérer le catalogue neuf comme une migration des fiches Cinéfolio ; import réel nécessitechoixprofil/racines parutilisateur. G04acceptéavec réserves, pasnouvellediscovery.

## Historique conservé

- Révision documentaire : `CC-DOC-001`, 2026-10-13, auteur `/root`.
- Produit : Chill Center ; dépôt cible `https://github.com/ungeekrange-cmyk/chillcenter.git`.
- Source : `/Users/ghillaumesoubigou/Documents/Dev/Perso/Video catalog/`, Cinéfolio 0.5.0.
- Baseline historique vérifiée : `90c149b797b45cba8a56dca2b74eed06c4225060` dans `Cinefolio-source.bundle`. Les fichiers de travail restent distincts d'un checkout Git : leur égalité complète au commit n'est pas revendiquée.
- Destination confirmée : `/Users/ghillaumesoubigou/Documents/Dev/Perso/ChillCenter/`. Preuve : « erreur de ma part si j'ai dit ChilleCenter, c'est ChillCenter ». Aucun dossier existant n'a été renommé.
- Étape 1 : `APPROVED_BY_USER`, périmètre `CC-FONC-002`. Preuve : « vise directement un serveur sur linux, on va zapper la version mac, le reste ok ». Cette correction retire le serveur macOS ; le client Mac Apple Silicon validé reste prévu.
- Étape 2 : `APPROVED_BY_USER`. Preuve : « oui étape suivante », consignée dans `docs/gates/G-02-CC-DOC-001.md` ; sept ADR et contrats acceptés sans modification de conception.
- Étape 3 : `APPROVED_BY_USER`, dossier `CC-STORIES-001` ; preuve « validé, go implémentation » dans G-03 après revue indépendante APPROVED r2.
- Étape 4 : `IN_PROGRESS`. Lot interface Mac/serveur HTTPS implémenté et revu à `c2a9d72d634cc7282eb8a20ca9fa138ed92e2fec`, 124 tests et trois parcours GUI ; pas de remise finale ni validation G-04 demandée. Voir `docs/implementation/STATUS.md` avant reprise.

## Contraintes

Serveur Linux uniquement, matériel de référence Raspberry Pi 3 B (éventuellement B+), 1 Go de RAM ; NAS sur réseau 1 Gb/s, Pi probablement en Wi-Fi. Deux à trois profils simultanés, surtout 1080p ou moins. Client Mac Apple Silicon d'abord, web plus tard. Usage local au foyer, sans authentification. Maximum de logique en Rust, aucun transcodage serveur, stockage vidéo lu uniquement par le serveur.

## État des capacités

Lecture des sources et archive Git vérifiées ; documentation primaire consultée le 2026-10-13. Accès GitHub initial par connecteur : 404, puis CLI autorisée : succès, dépôt vide à l'intake. Implémentation sur branches isolées `codex/`, source Cinéfolio préservée. Lot actuel : 124 tests, revues indépendantes code/test/sécurité r3 approuvées pour son SHA, vraie webview/HTTPS loopback et paquet de développement arm64 vérifiés ; détails dans INTERFACE-EVIDENCE. Aucun push distant, accès NAS ou déploiement Pi effectué. Pi déclaré non prêt, IP prévue `192.168.1.42` et montage `/srv/volumes/video` : exemples configurables, jamais des constantes imposées.

Correction UI utilisateur du 2026-10-13 (capture de la barre Cinéfolio) : scan réservé à Administration. Implémenté dans `ui/connected` ; le bouton Actualiser du catalogue ne fait aucun scan. Aucun nouvel examen de périmètre nécessaire.

Les anciens `docs/PROJECT_CONTEXT.md` et `scripts/agent-context.py` ne sont pas présents dans le dossier Cinéfolio déplacé. Le présent fichier devient le point d'entrée durable du nouveau projet.

## Prochaine action autorisée

### Branding — choix du 2026-10-13

Référence actuelle r5 : l'utilisateur valide le vectorisé par « nickel, utilise le vectorisé comme base pour refaire les autres ». `icons/branding/vector/default.svg` est désormais la base par défaut, son PNG est `vector/default.png`. Les12 PNG d'ambiance sont réexportés depuis leurs SVG, issus du gabarit commun et du générateur Rust, sans nouvelle génération IA. Ancien PNG original `icons/branding/default.png` inchangé ; anciens rendus conservés sous `raster-r4/`. Manifeste de marque format2, preuves/hashs dans `vector/exports-r5.json`. Aucune intégration runtime par cette opération d'assets.

L'utilisateur retient comme logo par défaut le **premier des deux derniers rendus**, avant la seconde translation vers le bas : image source `exec-df820cb4-e6c6-4ce6-8266-d301ab7fe79d.png`, copie `icons/branding/default.png`. Preuve : « oui on garde celui-ci par défaut » puis « quand je dis celui-ci, le premier des deux ». Douze propositions statiques pour printemps/été/automne/hiver/Noël/Halloween × clair/sombre sont dans `icons/branding/`, avec galerie, hashes et prompts. Leur validation visuelle et raccordement runtime restent distincts de cette génération ; ne pas prétendre que l'application change déjà automatiquement de logo.

### Implémentation

Checkpoint CC-PLAY-001 du 2026-10-13 : source `b4100a37f985943fa1932bd98a2ec99d70b6c044`, baux/Range/progression/reprise serveur et helpers Rust disponibles, trois revues r2 approuvées,146 tests/build/clippy et vraie vidéo HTTPS headless jusqu'à EOF PASS. Voir PLAYBACK-EVIDENCE/API dans docs/implementation. Branding Halloween sombre r6 corrigé (orange distinct, tige/contour clair), BUG-0011. Interface vidéo Tauri, transfert/SSE, sous-titres externes, enchaînement et Pi restent non livrés ; prochain lot = surface native/commandes UI + progression/heartbeat automatiques. Pas de push distant ou NAS touché ; G-04 toujours non remis.

Branding r4 : l'utilisateur autorise explicitement « Oui, créer le gabarit SVG commun » pour supprimer les différences d'espacement entre ambiances. Source `icons/branding/vector/master.svg`,13 SVG générés par `scripts/branding.rs` (Rust, sans dépendance supplémentaire), galerie mise à jour. Original PNG approuvé intact ; anciens PNG conservés. Sapin commun derrière les lettres/dossier avec débordements latéraux ; été plus chaud et lumineux ; citrouille commune sur l'assise. Preuves dans `vector/provenance.json`, BUG-0010 ; SVG proposés pour validation visuelle, runtime toujours non raccordé.

Poursuivre l'implémentation autorisée : setup/profils/admin/catalogue Tauri raccordés ; intégrer maintenant surface lecteur Tauri, routes vidéo/sous-titres, baux/progression/enchaînement puis autres stories. Demander G-04 uniquement à la remise complète. Ne pas recommencer discovery/architecture/stories ni affirmer que 124 tests couvrent 133 AC ou le Pi/Wi-Fi. Les LOW et fonctionnalités non terminées restent listés dans STATUS ; ce checkpoint n'annonce pas de travail en arrière-plan.
