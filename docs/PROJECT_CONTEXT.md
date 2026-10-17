# Contexte du projet

## Référence actuelle — v0.6

17 octobre 2026 : validation utilisateur « validé commit en v0.6 et merge avec doc propre ».
Client Mac **0.6.0 / build 7**, bundle de référence
`artifacts/macos/Chill Center-v0.6.app`, tag local `v0.6`.
La [fiche de livraison](releases/REL-V0.6.md) fait autorité pour cette version,
ses preuves, ses limites et le retour arrière. Elle consolide le candidat validé
`73cbc8c` (source QA `095631a`) sans nouvelle modification fonctionnelle.

Branche de préparation `codex/release-v0.6`, destination d'intégration
`codex/player-streaming`. Aucun accès ou déploiement Pi/NAS, aucune modification
du chantier `codex/web-pwa`, aucun push ou changement de visibilité GitHub.
Les sections ci-dessous sont des checkpoints historiques : leurs anciennes
mentions « à préparer », « encore ouvert » ou « bundle à tester » ne remplacent
pas la fiche de livraison actuelle. Elles sont conservées comme preuves datées.

## Contrats QA — bundle Mac reconstruit

17 octobre2026 : branche `codex/qa-player-contracts`, basec071b42. Les deux anciens
échecs sont des assertions périmées (compteur global et recherche textuelle du garde
de modes), pas des pannes du lecteur. Test Node seul corrigé, contre-tests et readiness
VM ajoutés : **7/7ciblés et130/130Node PASS**. Aucun fichier runtime, Pi, API ou client
web modifié, aucune lecture réelle rejouée. [BUG0106](debug/BUG-0106-stale-player-qa-assertions.md),
[remise et limites](implementation/QA-CONTRACTS-20261017.md).
Source **095631ad55d1502480c874016a24f3b79968ff28**, bundle à tester
**`artifacts/macos/Chill Center-test-095631a.app`**, SHAexécutable
`2865690fef80cc01ec00e8c7475f34cc81b01c5c2017f811376f966f4f915ab1`.
Signature profonde/48dylibsARM64/libmpvAPI2.5 PASS, copie canonique identique ;
ancien15c2d63 conservé et hash inchangé. Pas de lancement natif ou déploiement Pi/web.

## Lot client livré pour test — sous-titres, retour grille, français/anglais

16 octobre2026 : source **15c2d63c2bca5058456a16c8ac7d0320d440ad4c**, branche `codex/playback-locale`, base0c49a8c. Corrections BUG0104/0105
et préférence locale `auto/en/fr`, anglais de repli, français système. Vérifications
locales ciblées exécutées, validations humaines natives restantes ; serveur/API/web/NAS
inchangés. [Preuves](implementation/PLAYBACK-LANGUAGE-20261016.md).
Huit sorties de compilation nettoyées sur autorisation explicite, **32,81Gio de disponible
volume supplémentaires observés avant reconstruction**, sources/worktrees/historiques/
versions livrées/web conservés : [mesure](performance/STORAGE-CLEANUP-20261016.md).
Bundle **`artifacts/macos/Chill Center-test-15c2d63.app`**, exécutableSHA
`d26d53034aa895a443fb8918a4e14e0294a1286f515859babbb678754a4a75b0`.
Signature profonde/48dylibsARM64/libmpvAPI2.5 PASS, copie canonique vérifiée identique,
ancien bundle conservé ; pas de lancement de la nouvelle app ni déploiement Pi.

## Ressources — livraison courante, sans changement du client web

16 octobre2026 : demande explicite d'appliquer les pistes RAM/stockage, mesurer, reconstruire Mac et mettre à jour Pi. Source **f9803557a452f59cbbe3cbf5b1c4bf1b48a9aa67**, branche `codex/resource-optimizations`, base228c4c6. Client : publication par déplacement sans copie complète ; serveur : Arc partagé reset/checkpoint complet, protocole/admission/TTL inchangés ; affiches : CAS privé par serveur et liens physiques vers les anciens chemins, budget logique/hors-ligne conservés. Aucun UI/web/API/dépendance/SQLschema/tampon changé. [ADR0024](architecture/decisions/ADR-0024-artwork-content-deduplication.md) accepté après revue indépendante, rollback ancien bundle compatible.

Mesures **release** sur fixtures identiques10000médias, trois processus par version : RAM client90,98→65,80Mio(−27,68%), serveur238,91→211,98Mio(−11,27%). RSS de bancs locaux Mac, pas de l'app Tauri complète ni du Pi. Cache réel fermé/migré : allocation unique189,55→47,55Mio(−142Mio),3926/3926références d'images,3930fichiers historiques et états intacts ; limite APFS/volume libre documentée. [Méthode/chiffres bruts](performance/RESOURCE-OPTIMIZATION-20261016.md). Pas de VACUUM/purge de rétention ou réduction du tampon.

Bundle à tester **`artifacts/macos/Chill Center-test-f980355.app`**,0.5.1/build6 ; exécutableSHA **ad699344f983eee9ba3e204b57d925009152a0688cec8e3677e794a2d3f135ee**, signature48dylibsARM64/libmpvAPI2.5 PASS. Pi livré22:03:11UTC, binaire installé/processusSHA **6fecabc69258e372d71730a23451f5ff47565f60cbbc352552c12d8ac39424ea**, service stable, backup exact1924fichiers/SQLite/rollback conservés. Web installé/en chantier, configuration/unité/TLS/montage NAS inchangés ; six sondes API2026-10-14/v1/web200. [Livraison](releases/REL-RESOURCE-OPTIMIZATION-001.md), [revues](reviews/resource-optimization-f980355.md), [vérification indépendante APPROVED_SCOPED](releases/REL-RESOURCE-OPTIMIZATION-001-verification.md). Source initiale9fb rejetée localement pour succès partiel de migration et Clippy, corrigée avant toute mutation réelle : BUG0103, preuve défavorable conservée.

Les sections suivantes sont historiques. Pas de nouvelle lecture vidéo, fournisseur, scan ou benchmarkNAS ; le RSSPi observé au redémarrage ne prouve pas un gain de pourcentage. Sources/worktrees/historiques et versions livrées conservés ; aucune publication distante ou modification de branche active du chantier web.

## Configuration / matching / tampon — livraison courante

16 octobre2026, **21:14:22UTC** : source **b36c916f0ea24fe40fe90df61971fd3e8e6044a0**, branche `codex/configuration-matching`. Pi mis à jour, binaire installé/processusSHA **dbda76e377a0f8ce84262396e6c339e44474846434351f9b4ec5bb5db4f5f100** ; sauvegarde privée exacte/rollback conservés. Client web installé, chantier web, NAS, configuration/unité/TLS et réglages de bibliothèque inchangés. [Livraison](releases/REL-CONFIGURATION-MATCHING-001.md), [revues indépendantes finales](reviews/configuration-b36c916.md), [vérification post-bascule indépendante APPROVED_SCOPED](releases/REL-CONFIGURATION-MATCHING-001-verification.md). Premier essai opérateur échoué avant remplacement puis restauration vérifiée, scripts corrigés/revus avant seconde tentative : BUG0102, historique conservé.

Bundle à tester **`artifacts/macos/Chill Center-test-b36c916.app`**,0.5.1/build6,81Mio, exécutableSHA **1c1f4bbe1b957ab593601041e5ea7807b7bc1263b27923cae82cde229305d344** ; signature48dylibs/libmpvAPI2.5 PASS. Feedback TMDB immédiat, secours suffixes, séries typographiques sans fusion d'homonymes, Configuration catégorisée/recherche/multi-emplacements, détection ciblée fichier absent, noir entre vidéos et tampon maintenu en pause/contrôles au repos. Suites intégrées82catalogue/41client/74domaine/136serveur PASS, tests ciblés navigateur et libmpv synthétique PASS ; rendu humain Tauri, fournisseur réel et débit NAS/Wi-Fi restent distincts, pas de lecteur général rejoué.

Cas réel Avatar fourni :503MEDIA_FILE_MISSING sans bail, puis disponible=false/révision2 persistée ; API datée2026-10-14/v1 et web contrôlés compatibles. [Audit RAM/stockage](performance/RESOURCE-AUDIT-20261016.md) en lecture seule : cache client≈175Mio, au moins≈59,5Mio de copies d'images dans le sous-ensemble vérifié, deux copies de catalogue à éviter ; aucune optimisation d'exécution, suppression ou compaction appliquée. RSS serveur ponctuel avant/après non comparables, aucun gain prétendu. Les sections suivantes restent historiques, leurs mentions « à préparer/non déployé » ne gouvernent pas cette livraison.

## Recommandations — fiches et demande de conseil

16octobre2026 : précision utilisateur « en fait les deux ». Source **fa40e080edae6dd9d36180e00db0969e4252829e**, branche `codex/recommendation-rails`, bundle de test **`artifacts/macos/Chill Center-test-fa40e08.app`**, SHA exécutabledaa2ab80ba2a4814610419e2bfbf8d56330632ce70a56a1839611bfa1a56873a. Rails horizontaux communs, films/séries regroupées sans épisodes, identités opaques conservées ; relations locales sur genres/réalisation/distribution, conseil IA explicite sur contexte local filtré enrichi. Organisation existante et API datée/v1 inchangées, Pi/web/NAS non touchés.

[Preuves/limites/retour arrière](implementation/RECOMMENDATION-RAILS-20261016.md), [ADR0020](architecture/decisions/ADR-0020-local-title-recommendations.md), [revue indépendante APPROVED_SCOPED](reviews/recommendation-rails-fa40e08.md).70domaine/40client/8pontIA/6assistant/44Node et vrai DOM connecté synthétique ciblé PASS, Clippy/API76/signature48dylibs/libmpv2.5 PASS, scan des nouveaux commits sans fuite détectée. Première revue779e299 défavorable corrigée : affiches de conseil retirées lorsque le panneau est masqué, réponses tardives rejetées, sélection conservée au retour. Aucun vrai fournisseur ni lecteur rejoué ; rendu humain Tauri et pertinence d'un conseil réel restent à valider. Anciennes versions et tags préservés, interne0.5.1/build6 inchangé ; choisir le nom de bundle exact plutôt que ce numéro partagé.

## Transition suivante — correctif local, validation humaine précédente enregistrée

Remise : source **8be19f461712c1260db2b21c3d8aba1cb59c3064**, bundle **`artifacts/macos/Chill Center-test-8be19f4.app`**, SHA exécutabled94fa7adcf95edb4e483ff88bd7d338b1b730b077029097dc84d1be87e819716, interne0.5.1/build6, signature/48dylibs/API2.5 PASS. [Preuves et limites](implementation/NEXT-TRANSITION-20261016.md), [revue indépendante APPROVED_SCOPED](reviews/next-transition-8be19f4.md).32Node/24web et deux contrôles de rendu ciblés PASS, aucun nouveau lecteur réel ou déploiement ; transition Tauri à confirmer humainement. Le « bundle à préparer » ci-dessous conserve le checkpoint pré-compilation.

16octobre2026 : utilisateur confirme **78,79/80,85/86/87 et88 corrigés** : sept fiches clôturées sur ce retour, sans rejouer leurs tests natifs. Il demande ensuite de corriger **91/92/93**, remplaçant sa consigne antérieure de référencement seul. Contraste local du bouton, séparation lancement/annulation et fond d'affiche floutée préparé8s avant EOF intégrés ;32Node ciblés et rendu DOM connecté synthétique PASS, bundle à préparer. Aucun Rust, API, serveurPi, NAS, asset web installé ni chantier web modifié ; pas de nouvelle lecture vidéo réelle.0081–0084 toujours ouverts.

## Retours humains sur la proposition suivante — référencement seulement

16 octobre2026 : utilisateur demande explicitement de **ne pas traiter** trois nouveaux points : contraste du bouton « Ne pas lancer » ([BUG0091](debug/BUG-0091-next-cancel-button-contrast.md)), message d'annulation à zéro suivi malgré tout du lancement ([BUG0092](debug/BUG-0092-next-countdown-false-cancellation.md)), fond blanc à remplacer par une transition aux tons de la série, éventuellement floutée et préparée quelques secondes avant EOF ([BUG0093](debug/BUG-0093-next-transition-white-background.md)). OPEN, sans diagnostic ni test/code/bundle/déploiement ; version de l'application ouverte non mesurée. La preuve API001→002 ci-dessous reste valide dans son périmètre, pas une validation de ces comportements visuels.

## Pi mis à jour — serveur seul df2870f, 16 octobre14:44UTC

Connexion partagée rétablie et préflight conforme : bascule depuis d77e/f3 vers **14d84a0478b9f394bfe59b3b88588edb4a9305dfedfc0fb0b66449b39d034169**, source **df2870f**. Opérateur revu terminé **rc0**, sauvegarde privée exacte9b975f1e… et ancien binaire conservés ; service redémarré PID2326. Web installé, configuration/unité/TLS/montage NAS inchangés. [Exécution et rollback](releases/REL-NEXT-POSTERS-SCROLL-MATCHING-001.md). [Vérification indépendante APPROVED_SCOPED](releases/REL-NEXT-POSTERS-SCROLL-MATCHING-001-verification.md) : service stable, sauvegarde exacte, API datée/v1 et cas réel001→002 PASS ; pas de nouvelle lecture vidéo ni test humain Tauri revendiqués. Bundle à utiliser **`artifacts/macos/Chill Center-test-df2870f.app`**. Le blocage SSH décrit ci-dessous est historique, désormais levé.

## Suite numérotée / affiches / scroll / matching — candidat df2870f

16 octobre 2026 : source **df2870f5776d20213bcb8c6c88a5bcb967091e97**, branche `codex/fix-next-posters-scroll-matching`. Le test humain du candidat94495c1 a invalidé l'enchaînement réel : 150 épisodes sans numéros structurés mais avec préfixes de fichiers uniques. Ordre plat explicite désormais reconnu sans modifier la base ; ordre inconnu distinct d'une fin. Affiche de fiche retirée avant chargement et anciens observers neutralisés ; cartes/images virtualisées conservées au scroll ; 19 titres à parenthèses orphelines nettoyés pour le matching automatique sur 2496 entrées parcourues en lecture seule.

[Remise et limites](implementation/NEXT-POSTERS-SCROLL-MATCHING-20261016.md), [revue indépendante favorable](reviews/next-posters-scroll-matching-df2870f.md). Nouveau bundle `artifacts/macos/Chill Center-test-df2870f.app`, interne0.5.1/build6, SHA exécutable20316cb655d45327c283184fab3c1675c3c0f9d99a6fb2a3b423c7e61e47d182 ; signature/48dylibs/API2.5 PASS. **Pi non mis à jour : socket SSH absent**, dernier état vérifié f3fdac5 historique, pas une nouvelle sonde. Web installé, NAS et configuration non touchés ; API datée et v1 préservées. Tests ciblés et vrai DOM synthétique PASS, validation native humaine restante. Les paragraphes suivants restent des checkpoints historiques, non l'état de ces correctifs.

## Séries / EOF / vidéos courtes — candidat local

16 octobre 2026 : source **94495c1**, branche `codex/series-season-grid`. Première saison non vue ouverte, épisodes en grille ; affiche/synopsis/compteurs/note réunis, outils communs en bas. Retour à la série/saga/liste/fiche après dernier élément ; contexte conservé depuis la fiche d'un membre ; une synchro personnelle détachée par EOF confirmé, retours tardifs gardés. Exception <100s explicitement approuvée « Adapter la règle aux vidéos courtes » : fin après couverture réelle du dernier dixième dans le bail courant, seuil long inchangé. Source serveur modifiée mais **Pi/web/NAS non déployés ni touchés** ; bundle seul ne peut activer cette règle sur l'ancien serveur.

[Remise et tests](implementation/SERIES-COMPLETION-20261016.md), [revues](reviews/series-completion-94495c1.md), BUG0085–0087. 43Node/65domaine/27serveur/1horloge/24web et vrai DOM connecté synthétique PASS ; revue finale indépendante APPROVED_SCOPED, deux révisions défavorables conservées. Pas de nouvelle lecture vidéo réelle ni de validation humaine native revendiquée. Bundle créé `artifacts/macos/Chill Center-test-94495c1.app`, interne0.5.1/build6, SHA **d642758aa39c12cca7aa8d6ed887a8b059897b956569c2f93e0b46dd65a343ba**, signature/48dylibs/API2.5 PASS, non lancé. Tags et anciens bundles figés conservés ; consulter la remise pour les limites et la suite serveur nécessaire.

## Blocage décor / vidéo — candidat de test, non déployé serveur

16 octobre 2026 : source `bdce3a4`, deux lignes CSS désactivent le pseudo-fond Noël/Halloween pendant `body.player-open`. RED/GREEN sur le vrai DOM connecté avec pont synthétique, huit combinaisons et retour du décor ; revue indépendante APPROVED_SCOPED. Bundle séparé `artifacts/macos/Chill Center-test-bdce3a4.app`, interne0.5.1/build6, SHA exécutable `6aa696eeff5dcdafbdd9fc9c456f699c1e56ff24e4336fdd974163c72c1f5a5f`, 81Mio/48dylibs/signature/libmpv API2.5 PASS. **Pas une nouvelle release**, anciens bundles/tags inchangés ; rendu Tauri à confirmer humainement, aucune lecture/connexion Pi/NAS rejouée. [Remise](implementation/EVENT-PLAYER-CUTOUT-20261016.md).

Six autres défauts suivis OPEN, non corrigés : ancienne affiche sans match, clipping au scroll, retour de validation TMDB, conflit non actionnable, loupe trop petite, rotation de synchronisation décentrée (BUG0079–0084). L’ancienne preuve BUG0065 n’est pas une validation de la réapparition actuelle.

## Préparation du dépôt GitHub public — non publié

Nettoyage lisible/commenté des modules filtres et ambiances, invariants de synchronisation documentés ; README, licence, contribution, sécurité et formulaire de bug préparés. La priorité demandée est le dépôt GitHub, pas un nouveau site applicatif ; Pages reste facultatif et manuel. Les exemples ne ciblent plus une installation du foyer, les outils exigent leurs adresses à l’exécution, port `8443` conservé. Les adresses des comptes rendus ont été anonymisées sans réécrire l’historique. Les anciens hashes et gates restent historiques, à revalider avant réutilisation.

[Préparer le dépôt](publication/GITHUB-REPOSITORY.md), [option Pages](publication/GITHUB-PAGES.md), [défauts de préparation](debug/BUG-0077-household-addresses-in-public-source.md). Audit local des commits et objets Git : aucun secret réel identifié, alertes TMDB classées fixtures de tests indépendamment. Les anciens commits contiennent encore des informations du foyer : aucun accord de visibilité publique du dépôt complet, aucun push, réécriture d’historique ou déploiement. Pi, NAS, bundles figés et autre chantier web inchangés.

## Correctif Mac0.5.1 — fond du bandeau événementiel

16octobre2026 : [BUG0074](debug/BUG-0074-event-header-opaque-background.md), [release](releases/REL-V0.5.1.md). Source375ec6b, tag localv0.5.1, bundle `artifacts/macos/Chill Center-v0.5.1.app`, version0.5.1/build6/SHA815c2a0c76e70c533fa909fcd6fc9d23fe72e8eb0ca4cd1b5e978ff783e35799. Une seule règle CSS produit : fond de la barre supérieure transparent pour Noël/Halloween, clair/sombre, image déjà pleine hauteur. RED/GREEN sur interface connectée synthétique/recherche visible et fond normal conservé hors événement ; packaging/signature/dépendances PASS, lancéPID36612 à10:27:13UTC. Pas de nouveau test lecteur ni revue indépendante revendiquée. Tag et bundle0.5 préservés, Pi/API/web inchangés. Test visuel natif humain à faire.

## Mac v0.5 — filtres et ambiances intégrés, Pi/web inchangés

16octobre2026 : approbation « validé. Lance la version et on la fige en v0.5 ». Source **1e16ee2**, tag local **v0.5**, application **0.5.0/build5**, bundle à utiliser `artifacts/macos/Chill Center-v0.5.app`, exécutableSHA169ac5126ea4369174db35ad00f12ea5029bf51e686c0cf4ae2283c5fd604a84. Filtres genres OU/années exacte ou intervalle inclusif locaux instantanés, cache de base réutilisé et invalidé ; décor sapin/citrouille rapprochés ancrés à droite, panneaux Noël rouge/Halloween orangé et animations bornées/réduction/pause/off. API2026-10-14, serveur, chantier web et NAS **non modifiés**, aucune nouvelle sonde Pi.

[Release](releases/REL-V0.5.md), [revue indépendante APPROVED_SCOPED](reviews/v0.5-code-1e16ee2.md), [vérification post-publication APPROVED_SCOPED packaging/lancement](releases/REL-V0.5-verification.md).17tests ciblés, vrais parcours DOM de l'interface connectée avec pont synthétique, cargo check, packaging81Mio/48dylibs/signature/libmpvAPI2.5 PASS. Lancement du bundle livré confirméPID33148, identité exécutable/hash/tag vérifiée distinctement. Validation visuelle/ressources Tauri réelle à faire humainement ; pas de nouvelle lecture vidéo. Suite globale74/76 : deux assertions QA lecteur déjà en échec identique sur basef3, non masquées ni corrigées hors périmètre. [BUG0073](debug/BUG-0073-event-animation-pause.md) conserve le RED/GREEN de pause des animations dans le candidat. Les checkpoints ci-dessous restent historiques ; le serveur demeure celui de la livraisonf3 tant qu'une sonde nouvelle ne dit pas autrement.

## Livraison serveur seul f3fdac5 — Pi vérifié, web inchangé

15 octobre 2026 : source `f3fdac503c4b845571981c1fd1bc00bd0b13ec21`, serveur SHA `d77e9e1669287a6d10f279c17f33d7f776efe7e194acc01cd542bcea33511c90`, actif PID3117/NRestarts0 depuis22:14:29Z. Vérification indépendante22:18–22:20Z APPROVED_SCOPED. Web préservé (contenu/inodes/métadonnées), configuration/unité/TLS/montage inchangés ; HTTP8080/HTTPS8443 et accès externes80/443 confirmés. API2026-10-14/v1 conservées,70 chemins/89 opérations hérités présents ; `selector=key` facultatif pour les nouvelles identités de séries, anciens titres inchangés par défaut ([ADR0019](architecture/decisions/ADR-0019-explicit-series-selector.md)). Aucun déploiement web ni modification du chantier web.

Bundle à utiliser : `artifacts/macos/Chill Center-f3fdac5.app` (arm64,48dylibs, signature profonde ad hoc/APIlibmpv2.5 vérifiées).11aa529 est conservé mais reste partiel pour les nouvelles opérations opaques sans paramètre.296Rust/Clippy/API76 PASS, trois revues exactes favorables. Vrai UiClient :2496fiches/sagas/listes, révision de série présente, cache chaud, delta inchangé21ms PASS, sans mutation foyer/fournisseur/vidéo. Sauvegarde privée5dd8df08… exacte1746entrées/SQLitequick_check, rollbackffcc vérifiés.

[Release](releases/REL-SERVER-ONLY-F3FDAC5.md), [exécution](releases/REL-SERVER-ONLY-F3FDAC5-deployment.md), [vérification indépendante](releases/REL-SERVER-ONLY-F3FDAC5-verification.md). [BUG0072](debug/BUG-0072-deployment-session-observation.md) conserve le suivi SSH et le redémarrage prématuré pendant sauvegarde : code terminal opérateur perdu, état final vérifié indépendamment, aucune récupération de données/suppression de verrou. Pas de nouveau test lecteur/GUI, benchmarkNAS/Wi-Fi ou fournisseur. Ne pas installer l'ancien serveur11aa ni supposer queffcc est encore actif. Les checkpoints ci-dessous sont historiques.

## Vérification Pi / Mac11aa529 / web — 15 octobre, aucun redéploiement

Le remplacement concurrent est attribué au chantier web confirmé par l'utilisateur. Au contrôle SSH en lecture seule du15octobre à21:11:21Z : service actif, PID1634/NRestarts0, fichier installé et processus SHA `ffcc634923f637331cac25fec1e5283b062ef5e2943778bcab59460b6269b0f7`, identique à l'artefact du worktree `codex/web-pwa`. HTTPS8443/HTTP8080 conservés ; API datée2026-10-14, package0.1.0. La version0.1.0 seule ne distingue pas les builds.

Compatibilité Mac11aa529 **partielle** : vrai UiClient, TLS épinglé, catalogue2496, fiches/sagas/listes et réouverture chaude PASS ; delta et ajouts série/saison absents du serveur actif. Branche isolée `codex/mac-web-compat-20261015` réunit localement les sources Mac et web, conserve les deux listeners et ajoute une vérification de révision légère pour le navigateur. **Non déployée**, aucun nouveau bundle livré ; ne pas installer l'ancien serveur11aa seul, qui perdrait le web et sa configuration. [Preuves et limites](implementation/MAC-WEB-COMPATIBILITY-20261015.md), [état Pi du chantier web](deployment/PI-LIVE-STATE.md). Les historiques ci-dessous restent des checkpoints datés, pas l'état courant.

## Correctifs catalogue/IA — Mac11aa529 publié, Pi suspendu

15octobre : source `11aa529f2eced060156a3017456bf1d9634f8a56`, bundle `artifacts/macos/Chill Center-11aa529.app`, anciennes versions et `.claude/` préservées. Bouton IA disponible après sélection profil ; synchro détachée/delta, fiches fraîches/images en place, étoiles, matching admin et séries/saisons/homonymes. Revues source finales favorables,281 tests Rust concernés/41Node/Clippy/contrat/navigateur synthétique PASS. CLI IA/TMDB réels et GUI du bundle final non testés. [Release](releases/REL-LIBRARY-UX-SYNC-001.md).

**Ne pas écraser le Pi** : changement de binaire non attribué pendant la préparation. À15:57:04Z, fichier et processusPID9457 SHA `a7ab7b07633c916c358f1656aa8f154b207185d5ac306c374d2a7a52b0c7d527`, actif/HTTPS disponible, config/unité/SPKI inchangées. Ni c610 connu ni notre artefact11aa attendu `de98776d…`. Seconde tentative refusée au préflight, écritures suspendues et utilisateur interrogé. [BUG0068](debug/BUG-0068-unattributed-pi-binary.md), [vérification indépendante BLOCKED](releases/REL-LIBRARY-UX-SYNC-001-verification.md). Aucun rollback vers une version supposée, aucun nouvel essai lecteur à lancer.

## Livraison c610ea9 — Mac publié, Pi déployé et vérifié

15octobre : sourcec610ea9, bundle utilisateur `artifacts/macos/Chill Center-c610ea9.app` (77Mio, signature/48dylibs/API2.5 PASS), archiveLinuxc610 publiée ; anciennes versions préservées et promotion localeff-only, aucunpush. Pi mis à jour à12:43:07UTC, SHAcef54cd0…/PID8460/NRestarts0 ; backupprivé2cbaefcf… vérifié, rollbackCFCf853… conservé, NAS/config/unité/TLS inchangés. Vérification indépendante `/root/verify_filename_release` APPROVED_SCOPED : vraiUiClient TLS/cache/catalogue2496/fiche/sagas/listes/snapshot/réouverturechaude PASS, aucun warning+ ; [rapport](releases/REL-FILENAME-SCROLL-001-verification.md) et [release](releases/REL-FILENAME-SCROLL-001.md). Test humain Tauri isolé sur le correctif : « test concluant sur le scrolling de mon côté ». Pas de nouveau lecteur/vidéo/scan/TMDB réel ni de pilote installé. Variantes expérimentales28b1c64 et chantier série/saison eb73711 exclus et préservés. Matching français/original, fiche périmée et synchronisation bloquante/diff restent distincts/non corrigés dans ce lot. Ne pas relancer les hypothèses du simulateur non concluant.

## Historique : livraison suspendue avant l'essai humain

[Rapport](releases/REL-FILENAME-SCROLL-001.md) : candidat isolé71bd7a0 depuis la base livrée5bebf10, séries incomplètes exclues. Builds/tests ciblés effectués, mais test de molette interne Tauri non concluant après cinq hypothèses ; pas de bundle livré ni de déploiement. Essais expérimentaux préservés à28b1c64 sur `codex/diagnostic-modal-scroll`, hors candidat. Nouvel accord utilisateur/essai manuel requis avant nouvelle investigation. Pi toujoursCFC87de actifPID6808, Mac livré7a0f480 ; données/NAS/configuration/TLS intacts.

## Reprise 15 octobre — noms de fichiers et fiche, candidat local non livré

Branche de livraison `codex/release-filename-scroll`, base livrée `5bebf10`, reprise ciblée de `f74aa09`. [BUG0062](debug/BUG-0062-release-filename-parsing.md) : séparateurs répétés, parenthèses vides, champs/codecs audio/vidéo et VOSTA/VOST/DDP nettoyés ; recherche automatique des anciens titres utilise le même traitement, sans réécriture des métadonnées ou requêtes manuelles. [BUG0063](debug/BUG-0063-media-detail-scroll-and-path.md) : chemin relatif du fichier remis dans la fiche et scroll de fond verrouillé ; Chromium isolé RED/GREEN, confirmation Tauri à faire. Demande explicite de bundle et déploiement reçue. Le chantier séries/saisons incomplet demeure préservé sur `codex/series-tmdb-scopes`/`codex/filename-parsing`, hors de cette livraison. Les défauts distincts de fiche TMDB périmée, comparaison français/original et synchronisation bloquante/diff ne sont pas corrigés ici. Pi CFC87de/Mac7a0f480 inchangés avant bascule.
## Lot corrections catalogue/cache — Pi mis à jour le15octobre

Client7a0f480 et serveur correctifCFC87de revus indépendamment APPROVED ; bundle Mac `artifacts/macos/Chill Center-7a0f480.app` publié. Cache catalogue SQLite/images par serveur/profil, navigation/suggestions locales, échec de synchronisation sans répétition à chaque clic, scan concurrent refusé de façon bornée, purge immédiate après suppression confirmée. Sagas/listes/menu profil/Administration et suite20s : [release](releases/REL-CATALOG-CACHE-001.md), [preuves](implementation/CATALOG-CACHE-EVIDENCE.md), ADR0013 accepté. Après confirmation « Non déploie la nouvelle version », premier073 installé mais vérification CHANGES_REQUESTED pour révisions de sagas manquantes422 et auxiliaires vides dans backup : BUG0059/0060 conservés. Correctif cibléCFC déployé15octobre2026 à02:31:51UTC, hashf853… actifPID6808/NRestarts0. Sauvegarde privée8bbb9079…/rollback073 vérifiés, ancienne sauvegardec889… corrigée par quarantaine réversible de deux fichiers temporaires ; aucun fichier supprimé. NAS/configuration/unité/TLS inchangés. [Vérification indépendante finale APPROVED_SCOPED](releases/REL-CATALOG-CACHE-001-pi-verification.md) : UiClient réel2496fiches/sagas/listes/réouverture cache/TLS PASS,9,16s, sans nouvelle lecture vidéo/GUI/fournisseur. Nouveau Mac peut être lancé ; limites NFR/visuelles historiques maintenues.

## Dernier client — corrections visuelles574d255

Lot14octobre livré localement : [REL-MAC-UI-PANELS-001](releases/REL-MAC-UI-PANELS-001.md), [CC-UX-003](design/CC-UX-003.md). Nouveau bundle `artifacts/macos/Chill Center-574d255.app`, source574d255 ; plein écran initial configurable parMac (défautoui), mode fenêtre sur toute la zone droite, retour/titre/fondu/contrôles, collections et ajout par pages à vignettes. 290Rust/55Node/clippy, navigateur synthétique avec images chargées et deux parcours Tauri réels sur basesHTTPSlocales neuves PASS. Pi/NAS/réglages et profils réels inchangés ; anciens bundles préservés. Validation visuelle utilisateur en attente, pas de revue indépendante nouvelle revendiquée. HTML seul = aperçu non connecté, pas clientweb ; l'utilisateur a été explicitement prévenu. ADR0012 et BUG0052–0054 conservent les décisions et RED/GREEN.

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

Chill Center est désormais installé et actif sur **chillcenter.local:8443**, Pi3B+Rev1.3/Debian13.7 ARM64/905MiB/Wi-Fi. Source fonctionnelle126da0a, binaire13MiB9a17ffe…36a6 ; archive immuable R7 et overlay WorkingDirectory corrigent le démarrage, compte997:983+984/nologin, configurationroot0640 et données0700. NAS //nas.local/Video monté RO sur /srv/volumes/video, inchangé. SSH/Cockpit préservés. L'accès SSH se fait par le master privé /private/tmp/chillcenter-ssh.UAjtlb/control établi par l'utilisateur ; pas de clé ou mot de passe à copier, ne pas contourner un échec d'agent.

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

Lecture des sources et archive Git vérifiées ; documentation primaire consultée le 2026-10-13. Accès GitHub initial par connecteur : 404, puis CLI autorisée : succès, dépôt vide à l'intake. Implémentation sur branches isolées `codex/`, source Cinéfolio préservée. Lot actuel : 124 tests, revues indépendantes code/test/sécurité r3 approuvées pour son SHA, vraie webview/HTTPS loopback et paquet de développement arm64 vérifiés ; détails dans INTERFACE-EVIDENCE. Aucun push distant, accès NAS ou déploiement Pi effectué. Pi déclaré non prêt, IP prévue `chillcenter.local` et montage `/srv/volumes/video` : exemples configurables, jamais des constantes imposées.

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

**État réel du Raspberry Pi (à lire avant tout déploiement, même depuis une autre branche)** : [docs/deployment/PI-LIVE-STATE.md](deployment/PI-LIVE-STATE.md). Le Pi exécute depuis le 15 octobre 2026 un binaire construit sur `codex/web-pwa` (client web, redirection du port 443) : un déploiement construit sans ces changements casserait le client web, et refuserait la configuration si elle contient `http_listen`.
