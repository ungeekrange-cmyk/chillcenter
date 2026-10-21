# Journal des bugs, observations et risques

[BUG0127](BUG-0127-component-check-helper-coupling.md) : recherche des mises à
jour couplée à tort au helper d'installation ; capacités séparées, recherche
manuelle non privilégiée et bouton d'installation conditionné à une version
plus récente. Correctif 0.7.2 VERIFIED_LOCAL, revue sécurité APPROVED ; aucun
déploiement ou publication effectué.

[BUG0123](BUG-0123-native-theme-css-data-limit.md) : grands fonds natifs dépassant
la limite CSS ; Blob borné/URLs émises validées, preuve navigateur intégré.
[BUG0124](BUG-0124-native-update-test-fixed-version.md) : fixture de signature
figée à la version installée ; candidat suivant calculé, garde production intacte.
[BUG0125](BUG-0125-renderer-two-external-directory.md) : dossier de décoration
externe refusé ; whitelist/cap corrigés, vrai chargement depuis slot vérifié.
[BUG0126](BUG-0126-bonjour-dropin-writable-parent.md) : opérateur Bonjour refusant
un parent de drop-ins accessible en écriture à un autre utilisateur/groupe.

## Lecture de l'état courant

20octobre2026, lot0.7.1 non déployé : [BUG0119](BUG-0119-mdns-systemd-interface-family.md)
**OPEN / USER_PERMISSION_REQUIRED**, filtre systemd bloquant la découverte Linux.
[BUG0120](BUG-0120-discovery-before-https-bind.md) et
[BUG0121](BUG-0121-discovery-saved-server-race.md) corrigés/revus localement ;
[BUG0122](BUG-0122-private-deployment-operator-guards.md) outillage de déploiement
corrigé,21tests et revues R4 favorables, aucun swap Pi effectué.
Le [dossier0.7.1](../releases/REL-0.7.1-delivery.md) distingue DMG prêt et
déploiement bloqué avant arrêt ; ne pas annoncer le Pi0.7.1.

20 octobre 2026, déploiement Pi v0.7 : [BUG0118 empreinte d'unité systemd](BUG-0118-pi-unit-runtime-fingerprint.md)
**FIXED / VERIFIED_PI_GUARD** dans `815b523`. Premier essai revenu automatiquement
à l'ancienne paire serveur/web ; sauvegarde conservée. Seconde tentative PASS,
empreintes statiques et données confirmées par un vérificateur indépendant.
Le [gate final](../releases/REL-PI-V0.7-verification.md) est VERIFIED dans le
périmètre contrôlé ; ne pas requalifier le premier essai.

20 octobre 2026, clôture : [BUG0113 updater](BUG-0113-component-updater-release-gate.md)
**FIXED / VERIFIED_LOCAL**, huit HIGH et quatre défauts fonctionnels clos,
revues code/sécurité/tests favorables sur `3e174b9`. Dernier contrôle natif
d'annulation réussi sans téléchargement ni mutation. BUG0115/0116/0117 corrigés
et vérifiés ; publication des paquets séparée, aucun Pi/NAS contacté.

20 octobre 2026 : [BUG0116 archive QA AppleDouble](BUG-0116-qa-tar-appledouble.md)
corrigé/vérifié par installation native ; [BUG0117 minimum macOS du bundle](BUG-0117-macos-bundle-minimum-os.md)
corrigé dans le packaging, vérification finale du paquet requise. Le bundle
fourni requiert macOS 27+, pas 13+. Aucun Pi modifié.

19 octobre 2026, candidat corrigé : les huit constats sécurité et quatre défauts
fonctionnels de BUG0113 disposent de corrections et de preuves Linux réelles
sur VM isolée ; revues finales et validation native en cours, **pas de publication
ni de contact Pi**. [Preuves FR/EN](../implementation/COMPONENT-UPDATES-FIXES-20261019.md).
Le premier vrai lancement natif a révélé [BUG0115 configuration updater](BUG-0115-native-updater-bootstrap-config.md),
corrigé dans `3913267`, retest natif restant. Les checkpoints ci-dessous sont historiques.

19 octobre 2026 : [BUG0113 moteur de mise à jour privilégié](BUG-0113-component-updater-release-gate.md), **P1 OPEN / RELEASE_BLOCKED**, huit défauts Linux confirmés par revue indépendante. Aucune publication 0.7 ni installation Pi.
[BUG0114 instabilité du test de timing des sous-titres](BUG-0114-subtitle-timing-test-instability.md), P2 tests, cause non confirmée ; aucun changement du lecteur pour masquer l'échec.

17octobre2026 : [BUG0112 bandeau persistant et zones caméra](BUG-0112-persistent-fullscreen-notice-cutouts.md), correctif web ciblé R4 : bandeau limité à2,5s, autre chemin WebKit et safe areas quatre côtés. Retest physique Safari toujours distinct ; aucune app native lancée sur la base d'une question de faisabilité.

17octobre2026 : [BUG0111 vignettes/plein écran/recherche mobile](BUG-0111-mobile-posters-fullscreen-search.md), correctif web R3 en cours. Extension MP3 AVI de BUG0110 dans le Worker client ; preuves synthétiques et essai physique à distinguer.

17octobre2026, retours iPhone : [BUG0109 encombrement mobile](BUG-0109-mobile-content-obscured.md)
et [BUG0110 lecture iPhone](BUG-0110-iphone-web-playback.md), corrections web
vérifiées avec données synthétiques. Retest des vidéos sur appareil réel requis,
sans prétendre résoudre tous les codecs Safari à partir du seul conteneur.

17octobre2026 : [BUG0108 adaptateur smartphone web](BUG-0108-mobile-web-lifecycle.md),
FIXED/VERIFIED_LOCAL avant déploiement : navigation après lecteur, grille,
ambiance et plein écran tardif. Revue et essais physiques distingués dans le
rapport de livraison mobile ; aucun changement du client Mac.

Version Mac publiée de référence : [v0.6](../releases/REL-V0.6.md), validée le 17 octobre 2026.
Nouveau candidat : [corrections 82–84 et parité web](../implementation/WEB-PARITY-20261017.md).
BUG0082/0083/0084 **FIXED / VERIFIED_LOCAL**, avec limites Tauri humaines explicites.
Le client web exclut l'IA sur demande utilisateur ; aucun déploiement Pi dans ce lot.
[BUG0107 écarts d'adaptateur web](BUG-0107-web-parity-adapter-contracts.md) détectés
avant remise, corrigés et vérifiés localement dans ce candidat.
Ce journal est chronologique, du plus récent au plus ancien : les lignes OPEN
historiques ne rouvrent pas un bug corrigé dans une entrée ultérieure.
Les limites natives restantes ne sont pas converties en validations par le gel
de version. Les corrections après ce gel n'altèrent ni le tag ni le bundle v0.6.

17 octobre2026 : [BUG0106 assertions QA périmées](BUG-0106-stale-player-qa-assertions.md),
FIXED/VERIFIED_LOCAL_QA. Les deux faux échecs antérieurs sont corrigés :7/7tests
ciblés et130/130Node PASS ; aucun lecteur/runtime/Pi/web modifié.

16 octobre2026, lot client `codex/playback-locale` : [BUG0104 sous-titres/lecture](BUG-0104-subtitle-switch-blocks-playback.md)
et [BUG0105 grille après lecteur/fiche](BUG-0105-grid-after-player-detail-close.md), FIXED/VERIFIED_LOCAL.
Preuves et limites : [langues et lecteur](../implementation/PLAYBACK-LANGUAGE-20261016.md).
Test humain Tauri/NAS restant ; aucun serveur ou client web livré modifié.

Lot ressources16octobre2026, source **f980355** : [BUG0103 faux succès de migration partielle](BUG-0103-artwork-migration-partial-success.md), **FIXED/VERIFIED_LOCAL** avant toute mutation du cache réel, garde d'identité/comptages et sortie non-zéro testés. Revue9fb défavorable et deux Clippy corrigés conservés. Migration réelle complète ensuite, nouveau bundle Mac et serveur seul surPi, [vérification indépendante APPROVED_SCOPED](../releases/REL-RESOURCE-OPTIMIZATION-001-verification.md). Aucun nouveau bug utilisateur clôturé par simple benchmark ; [mesures release/cache réel](../performance/RESOURCE-OPTIMIZATION-20261016.md).

Lot Configuration16octobre2026, source **b36c916** : [BUG0081 feedback TMDB](BUG-0081-tmdb-apply-feedback.md), [BUG0094 suffixes de release](BUG-0094-release-suffix-fallback-matching.md), [BUG0095 séries typographiques](BUG-0095-series-spelling-duplicates.md), [BUG0096 fichier absent au lancement](BUG-0096-missing-file-launch-state.md), [BUG0097 ancienne frame](BUG-0097-previous-frame-during-load.md), [BUG0098 pause/cache/reprise](BUG-0098-resume-discards-buffer.md), [BUG0099 contrôles au repos](BUG-0099-player-controls-idle.md), [BUG0100 collision helper Configuration](BUG-0100-configuration-command-global-collision.md), [BUG0101 course de classification](BUG-0101-missing-leaf-classification-race.md) : **FIXED / BUNDLE_DELIVERED**, serveur seul livré21:14:22UTC, revue et vérification indépendante APPROVED_SCOPED. Cas réel Avatar indisponible persisté confirmé ; les rendus Tauri et tests physiques de débit restent à valider humainement. [Preuves et limites](../implementation/CONFIGURATION-MATCHING-20261016.md), [livraison](../releases/REL-CONFIGURATION-MATCHING-001.md). [BUG0102 opérateur privé](BUG-0102-private-deployment-wrapper-syntax.md) : premier échec/restauration et V4 rejetée conservés, V5 revue puis exécutée rc0. Les mentions OPEN anciennes de0081 restent historiques ;0082–0084 hors de ce lot.

Remise16octobre : **BUG0091/0092/0093 FIXED / BUNDLE_DELIVERED**, source8be19f4, nouveau bundle `Chill Center-test-8be19f4.app`, [preuves](../implementation/NEXT-TRANSITION-20261016.md), revue indépendante APPROVED_SCOPED ; validation humaine de cette nouvelle transition restante. **BUG0078/0079/0080/0085/0086/0087/0088 CLOSED sur confirmation utilisateur.** Sans correctif à ce jour :0081–0084. Les checkpoints ci-dessous restent historiques.

État16octobre après nouveau retour humain : **BUG0078/0079/0080/0085/0086/0087/0088 CLOSED**, confirmation explicite de l'utilisateur. Demande suivante de corriger0091–0093 : **FIXED_LOCAL**,32Node ciblés et rendu connecté synthétique PASS, nouveau bundle à préparer et validation humaine Tauri restante. Bugs applicatifs encore sans correctif :0081–0084. Les mentions « ne pas traiter », « validation restante » des sept bugs clôturés et autres anciens états plus bas restent historiques.

Signalement humain16octobre, **à référencer sans traiter** : [BUG0091 contraste « Ne pas lancer »](BUG-0091-next-cancel-button-contrast.md), [BUG0092 faux message d'annulation à zéro puis lancement](BUG-0092-next-countdown-false-cancellation.md), [BUG0093 fond blanc entre épisodes](BUG-0093-next-transition-white-background.md). Tous **OPEN**, cause non investiguée, aucun test/correctif/bundle/déploiement. Décor aux tons de la série, éventuellement flouté, à préparer quelques secondes avant la fin demandé pour0093. Capture et observation dynamique distinguées ; pas de défaut saga/liste déduit sans preuve.

16octobre14:44UTC : serveur source **df2870f** déployé, sauvegarde exacte et rc0 ; blocage SSH levé, [exécution](../releases/REL-NEXT-POSTERS-SCROLL-MATCHING-001.md). [Vérification indépendante APPROVED_SCOPED](../releases/REL-NEXT-POSTERS-SCROLL-MATCHING-001-verification.md), dont vrai001→002 PASS ; tests humains du lecteur court/enchaînement et rendu Tauri encore distincts. Les états « Pi en attente » ci-dessous conservent le checkpoint antérieur.

16 octobre, candidat **df2870f**, Mac livré / Pi en attente SSH : [BUG0088 suite numérotée](BUG-0088-flat-numbered-series-next.md), [BUG0089 parenthèses](BUG-0089-parentheses-matching.md), [BUG0090 affiches au boot rapide](BUG-0090-fast-boot-artwork-hooks.md) FIXED_LOCAL. [BUG0079 affiche sans match](BUG-0079-unmatched-poster-regression.md) et [BUG0080 clipping](BUG-0080-catalog-scroll-clipping.md) désormais FIXED_LOCAL, régressions DOM ciblées PASS, validation humaine Tauri restante. [Preuves et limites actuelles](../implementation/NEXT-POSTERS-SCROLL-MATCHING-20261016.md). Les mentions OPEN antérieures ci-dessous sont historiques pour0079/0080 ;0081–0084 restent ouverts.

Deux compléments visuels du 16 octobre, non bloquants et **OPEN** : [BUG0083 loupe trop petite](BUG-0083-search-magnifier-size.md), [BUG0084 rotation décentrée](BUG-0084-sync-icon-rotation-center.md). Captures utilisateur reçues ; aucun correctif revendiqué dans le lot limité au blocage vidéo.

Signalements du 16 octobre : [BUG0078 décor sur vidéo](BUG-0078-event-artwork-covers-native-video.md), BLOQUANT corrigé dans les sources / VERIFIED_DOM, essai Tauri humain restant. À traiter séparément, statuts OPEN : [BUG0079 affiche sans match](BUG-0079-unmatched-poster-regression.md), [BUG0080 clipping au scroll](BUG-0080-catalog-scroll-clipping.md), [BUG0081 retour de validation TMDB](BUG-0081-tmdb-apply-feedback.md), [BUG0082 récupération des conflits](BUG-0082-conflict-recovery-guidance.md). Aucun de ces quatre défauts n’est annoncé corrigé.

Préparation GitHub publique, non publiée : [BUG0075 page 404](BUG-0075-public-404-relative-assets.md) et [BUG0076 liens physiques](BUG-0076-public-build-hardlink.md), VERIFIED_LOCAL. [BUG0077 adresses du foyer](BUG-0077-household-addresses-in-public-source.md) : version de travail assainie, historique privé conservé ; aucun déploiement ou changement de visibilité. Voir la [stratégie de publication du dépôt](../publication/GITHUB-REPOSITORY.md).

[BUG0074](BUG-0074-event-header-opaque-background.md), FIXED/VERIFIED_LOCAL0.5.1 : bandeau opaque masquant l'image pleine hauteur, règle événementielle transparente ; RED/GREEN Noël/Halloween clair/sombre et hors événement, correctif livré séparément sans modifierv0.5 figée. [Release](../releases/REL-V0.5.1.md).

[BUG0073](BUG-0073-event-animation-pause.md), FIXED/VERIFIED_LOCAL v0.5 : spécificité CSS de pause des décorations corrigée, RED/GREEN dans le vrai DOM connecté synthétique ; pas une preuve de lecture native. Voir [release v0.5](../releases/REL-V0.5.md) pour la source1e16ee2, le bundle et les limites de validation.

[BUG0071](BUG-0071-legacy-series-api-compatibility.md) FIXED/VERIFIED : compatibilité des titres historiques, collisions titre/clé et sélecteur explicite ; sourcef3fdac5 livrée serveur seul, web intact. [BUG0072](BUG-0072-deployment-session-observation.md) : surveillance de commande distante et redémarrage prématuré pendant sauvegarde, incident terminé, état final vérifié indépendamment ; preuve du code retour opérateur indisponible conservée. [Release courante](../releases/REL-SERVER-ONLY-F3FDAC5.md) / [vérification APPROVED_SCOPED](../releases/REL-SERVER-ONLY-F3FDAC5-verification.md). Les mentionsffcc/non-déployé ci-dessous sont les checkpoints antérieurs.

[BUG0068](BUG-0068-unattributed-pi-binary.md), attribution RESOLVED le15octobre21:11Z : chantier web confirmé, Pi ffcc6349 actif ; l'ancienne livraison11aa reste non déployée. [Compatibilité Mac/web](../implementation/MAC-WEB-COMPATIBILITY-20261015.md) partielle sur le Pi, intégration locale non déployée. [BUG0069](BUG-0069-web-bridge-catalog-sync.md) : adaptateur web actualisé pour l'interface commune et invalidation des affiches tardives.

[BUG0067](BUG-0067-ai-button-availability.md), FIXED Mac11aa529 : bouton IA resté désactivé après sélection d’un profil, ordre de restauration/rendu reproduit/corrigé. Fournisseur réel non testé.

[BUG0064](BUG-0064-blocking-catalog-sync.md) synchronisation détachée/delta ; [BUG0065](BUG-0065-stale-detail-and-artwork.md) cycle de fiche/images/étoiles ; [BUG0066](BUG-0066-series-season-provider-scope.md) série/saison et homonymes. Mac11aa529 publié, serveur associé NON déployé/validé : BUG0068.

[BUG0062](BUG-0062-release-filename-parsing.md), 15 octobre : titres à séparateurs répétés, VOSTA/VOST/DDP et recherches automatiques corrigés sans réécrire les fiches. [BUG0063](BUG-0063-media-detail-scroll-and-path.md) : chemin relatif affiché et scroll modal, Chromium RED/GREEN puis vrai essai humain Tauri concluant. LIVRÉS c610ea9, bundleMac publié et Pi mis à jour/sauvegardé, vérification indépendante runtime APPROVED_SCOPED. Essais du simulateur natif non concluants préservés sur branche28b1c64, pas requalifiés en PASS.

Déploiement15octobre : [BUG0059](BUG-0059-lazy-saga-revisions.md) révisions absentes pour sagas TMDB paresseuses,422sagas/snapshot confirmé37/30 puis correctifCFC et sonde réelle2496médias PASS ; [BUG0060](BUG-0060-backup-empty-sidecars.md) deux fichiers auxiliaires SQLite vides mis en quarantaine réversible, sauvegardes exactes/rollback vérifiés. Première vérification indépendante défavorable conservée, addendum final APPROVED_SCOPED. Aucun nouveau test lecteur/GUI/vidéo dans ce lot de livraison.

[BUG0058](BUG-0058-replay-at-eof.md) : reprise à EOF lors d'une relecture, confirmée par vraie fixture native et RED unitaire ; correction locale au FileLoaded, preuve GREEN dans le lot cache.

Lot cache/retours utilisateur14octobre : [BUG0055](BUG-0055-shared-client-slot.md) concurrence du pont ; [BUG0056](BUG-0056-network-catalog-navigation.md) catalogue réseau répété ; [BUG0057](BUG-0057-playlists-profile-next.md) listes/profil/suite. FIXED, preuves natives/Pi et validation humaine distinguées dans le rapport du lot.

Lot de corrections visuelles14octobre : [BUG0052](BUG-0052-admin-navigation-order.md), ouverture Administration fragmentée entre callbacks et courses d'IO ; [BUG0053](BUG-0053-catalog-context-bootstrap.md), baseline de contexte après boot rapide ; [BUG0054](BUG-0054-detached-poster-observers.md), premières affiches désobservées pendant la construction d'une grille horsDOM. Causes confirmées et RED/GREEN conservés ; toutes détectées dans le candidat local avant remise, pas présentées comme pannes du Pi. Résultats et limites dans REL-MAC-UI-PANELS-001.

14octobre, état courant : [BUG0051](BUG-0051-pi-update-output-quoting.md), quoting opérateur confirmé et corrigé par déployeur distinct/V4 revue indépendamment ; premier échec et restaurationR7 conservés. Pi mis à jour source9277 à19:12:40UTC ; connexion/catalogue/reconnexion du même clientRust queTauri PASS, vérification distincte post-bascule APPROVED dans ce périmètre. La connexion bloquée provenait du nouveau client face à l'ancien contrat R7, pas d'un certificat à contourner. BUG0049/0050 corrigés dans le QAdebug seulement, deux parcours natifs finaux frais PASS ; les mentions d'investigation ci-dessous sont historiques. Voir [release courante](../releases/REL-UX-API-001.md).

Diagnostic indépendant [BUG0050](BUG-0050-native-qa-hidden-timers.md) : H5 confirme le blocage du QA masqué et son déblocage par visibilité/activation de sa propre application. Aucun H6 ; aucun deadlock GPU/Metal démontré. Correction du lancement debug à faire par builder distinct, puis deux parcours complets vierges et revues exactes avant livraison ; pas une validation du lecteur complet par l'early-exit diagnostique.

Contrôles natifs du candidat db9455c : [BUG0049](BUG-0049-native-qa-hidden-controls.md), course de l'oracle avec l'auto-masquage confirmée indépendamment et corrigée dans le QA uniquement ; contrôle de clic franchi au nouveau SHA. Le timeout distinct 52212 a été reproduit par le watchdog120s du parcours vierge99088, arrêté à `overlay inactivity`, `document_hidden=true`. Cause de ce timeout encore en investigation : aucune clôture ni livraison déduite du correctif hit-test. Les rapports [test0a](../reviews/ux-api-test-0a7e8f3.md) et [testdb](../reviews/ux-api-test-db9455c.md) restent défavorables historiques.

Intégration CC-UX-002/API2026-10-14 : [BUG0046](BUG-0046-native-fullscreen-geometry.md) oracle plein écran ignorait la safe-area AppKit, diagnostic indépendant H2 confirmé puis vrai QA lecteur GREEN ; [BUG0047](BUG-0047-openapi-dto-conformance.md) DTOs OpenAPI incorrectes, corrigées et testées HTTPS/JSONSchema ; [BUG0048](BUG-0048-tmdb-unbounded-jobs.md) file TMDB non bornée, HIGH bloquant avant correction et revue. Les rapports258 défavorables sont conservés ; aucune clôture globale ou nouvelle validation physique3flux déduite de ces correctifs.

Refonte14octobre : [BUG-0045 écarts UI](BUG-0045-client-ui-port.md), causes statiques confirmées, correction applicative non faite. [CC-UX-002](../design/DESIGN_SYSTEM.md) intègre les corrections humaines de placement/branding et attend validation visuelle ; IA actuelle dans Administration/profil, TMDB présent mais configuration UI/automatisme absents. Pas un bug résolu par la seule maquette.

InstallationPi14octobre : [0040 garde roinitial](BUG-0040-pi-install-ro-first-option.md), [0041 CLIexit1](BUG-0041-pi-help-smoke-exit.md), [0042 cwdNAS](BUG-0042-pi-nas-check-cwd.md), [0043 cwdservice](BUG-0043-systemd-workspace-cwd.md), causesconfirmées/correctionsopérateurrévues. Service désormaisactif, sourceRustinchangée ; [0044 querysonde](BUG-0044-pi-probe-catalog-query.md) corrigé dans le harnais, pas serveur. Toutesrevuesdéfavorablesconservées, [releasePi](../releases/REL-PI-001.md) et [preuvesmédiasréelles](../releases/REL-PI-001-media-evidence.md) gouvernent le dernier état ; pas tout133ACPASS.

Révision `CC-DOC-001`. Ce journal sera relu avant chaque nouvelle intervention. Les risques ne sont pas des bugs démontrés et les incidents historiques ne sont pas de nouveaux défauts Chill Center.

| ID | Type | État | Preuve / effet |
| --- | --- | --- | --- |
| HIST-001 | Incident Cinéfolio historique | ROOT_CAUSE_CONFIRMED, historique | Scan NAS timeout ; accès répertoire bloqué indépendamment du scanner. Source : `Video catalog/docs/debug/BUG-NAS-ACCESS.md`, 2026-10-12, code `1d842015d78e8d3cb7b2503d66309f422cc8fdd8`. Aucune reproduction Chill Center. |
| OBS-001 | Contexte/outillage | RESOLVED | Ancien cwd `Mongoose-Posture/Video catalog` inexistant ; source déplacée dans `Perso/Video catalog`, confirmée par l'utilisateur. Utiliser chemins actuels. |
| OBS-002 | Accès dépôt | RESOLVED_WITH_CLI | Connecteur lié : 404 ; CLI d'abord bloqué réseau du sandbox puis `gh repo view` autorisé : succès, dépôt vide. Ne pas conclure que le dépôt n'existe pas. |
| OBS-003 | Source sans checkout | BASELINED | Dossier source sans `.git`, mais bundle valide, historique complet 0.5.0. Ne pas initialiser/écraser la source ; utiliser copie isolée à l'implémentation et comparer différences de travail. |
| OBS-004 | Nom de destination | RESOLVED | Utilisateur corrige `ChilleCenter` en `ChillCenter`. |
| SEC-001 | Risque accepté utilisateur | ACCEPTED_FUNCTIONAL_RISK | Nouveau certificat accepté automatiquement après expiration peut venir d'un imposteur LAN. F-19 / ADR-0005 ; vérification cryptographique normale conservée dans les autres cas. |
| RISK-001 | Performance | TO_VERIFY | Pi probablement en Wi-Fi, deux passages réseau par flux ; trois vidéos non garanties avant V-02. |
| RISK-002 | Faisabilité lecteur | TO_VERIFY | Rendu libmpv Mac et adaptateur de flux/TLS à prouver dans l'enabler ; V-03/V-09, remplacement par ADR si nécessaire. |
| RISK-003 | Identité fichier | CONTROLLED_BY_DESIGN | Empreinte partielle insuffisante pour preuve absolue ; confirmation ou preuve complète avant réassociation. F-16 / ADR-0002. |
| DESIGN-001 | Lacunes d'architecture, revue indépendante | CORRECTED_PENDING_REREVIEW | ARCH-F-001/002 : bail lié média/révision, génération suivante et deadline de transfert non renouvelable ajoutés. Pas un bug applicatif exécuté. |
| DESIGN-002 | Contraintes bibliothèque, revue indépendante | CORRECTED_PENDING_REREVIEW | ARCH-F-003 : version/ABI, lifetime et annulation stream_cb contractualisés ; enabler encore NOT_RUN. |
| DESIGN-003 | Lacune TLS, revue indépendante | CORRECTED_PENDING_REREVIEW | ARCH-F-004 : déclaration de rotation nécessitant HTTP remplacée par pin SPKI stable vérifiable au handshake ; certificat renouvelé, clé protégée long terme. |

## Format de tout futur BUG

OBS-005, performance physique14octobre2026 : essai30min réel3flux(Dune1/Dune2/720p), Dune2échoue progression<80% etbufferingimportant, malgréPiRSS24,23Mio/CPUmax22,95%d'uncœur/swap0. Causeunique nonconfirmée, pas un défautRust supposé etaucunpatch spéculatif. [Preuves](../releases/REL-PI-001-media-evidence.md), sourcelecteur27bd5b2, Pi126da0a ; essaiadditionnel deuxDune10minsource624terminéPASSprogression/reprise/grantmaisbuffer184débutDune2. `iw` indisponible surPi : aucun changementWi-Fi/montage/routeur. Capacité3profilscode≠fluidité3vidéosgarantie, ENABLER006 reste àvalider.

État courant14 octobre : LOW UISEC-002/SRV-SEC-005 corrigés et testés par injection ([BUG-0031](BUG-0031-config-durability.md)). Les mentions ouvertes ci-dessous décrivent leurs anciens checkpoints. Nouveaux correctifs vérifiés : [0028 garde de bail indépendante](BUG-0028-buffering-deadline-guard.md), [0029 réponses IA tardives](BUG-0029-ai-late-response.md), [0030 pagination/version des empreintes](BUG-0030-fingerprint-pagination-version.md), [0032 références libmpv non épinglées](BUG-0032-mpv-unpinned-references.md), [0033 révisions après scan](BUG-0033-scan-initial-revisions.md), [0034 saga affiche/membres](BUG-0034-saga-artwork-members.md), [0035 tailles ARMv7](BUG-0035-armv7-large-file-fingerprint.md), [0036 empreintes après restart](BUG-0036-fingerprint-restart.md). Les preuves et limites restent dans chaque fiche ; revue finale sur candidat exact, pas approbation transitive des anciens rapports.

Dernière revue et reprise : [0037 sélecteur48/50](BUG-0037-picker-page-gaps.md), [0038 notes séries sans nom](BUG-0038-anonymous-series-rating.md), [0039 langues du lecteur](BUG-0039-native-language-key.md), reproduitsRED puisGREEN. Le rapport code initialCHANGES_REQUESTED est conservé ; relecture au nouveau SHA42050c7 avant remise.

Régressions relevées à la complétion : [BUG0024 métadonnées série/saga](BUG-0024-common-series-metadata-and-saga-cleanup.md), [BUG0025 plafonds legacy](BUG-0025-legacy-fallback-bounds.md), [BUG0026 publication bundle](BUG-0026-bundle-publication-race.md), [BUG0027 substitution source](BUG-0027-legacy-source-replacement.md). Corrections et fixtures exécutées dans le delta d'intégration, revue nouvelle requise. Les anciens rapportsdéfavorables sont conservés.

Complétion du14 octobre : BUG-0015–0019 et leurs preuves restent dans ce dossier. [BUG-0020 variantes serde fermées](BUG-0020-admin-ipc-unit-variants.md), [BUG-0021 ACL Tauri](BUG-0021-tauri-admin-command-permission.md), [BUG-0022 Content-Length ZIP](BUG-0022-archive-download-content-length.md), [BUG-0023 iconutil](BUG-0023-iconutil-iconset.md). Le parcours archives Tauri réel est PASS après0022 ; packaging autonome signé et libmpv chargé, inspection Finder/Dock NOT_RUN. Les revues de slice ne valent pas remise des33 stories.

Lecture HTTPS : [BUG-0012 crédit ×0,5](BUG-0012-slow-playback-final-credit.md), VERIFIED par test RED/GREEN et horloge serveur déterministe ; seuil inchangé de10s réelles.

Revue du slice lecture r1 : [BUG-0013 EOF HTTP](BUG-0013-http-reader-eof.md) et [BUG-0014 capacité des corps non consommés](BUG-0014-unpolled-stream-capacity.md), VERIFIED : RED/GREEN,146 tests, smoke EOF et trois revues r2 APPROVED au SHA b4100a3. Rapports r1 conservés, source et limites dans PLAYBACK-EVIDENCE.

Branding r6 : [BUG-0011 citrouille Halloween sombre](BUG-0011-halloween-pumpkin-contrast.md), VERIFIED : orange distinct, tige/contour clairs ;4 tests et rendu navigateur, géométrie inchangée.

Suivi r5 de BUG-0010 : base vectorisée validée par l'utilisateur,13 PNG réexportés depuis les SVG communs ; les12 anciens rasters canoniques sont archivés dans `icons/branding/raster-r4/`. Les comparaisons raster r3/r4 restent historiques, pas les assets courants. Sources SVG/générateur inchangés, aucun nouveau défaut applicatif revendiqué.

Assets de marque r3/r4 : [BUG-0010 géométrie du canapé](BUG-0010-branding-geometry.md) : VERIFIED pour les SVG communs autorisés par l'utilisateur, mêmes tracés sur13 variantes ; défaut principal Noël clair corrigé, branches latérales vérifiées. Audit des PNG et leurs écarts résiduels conservés dans `icons/branding/geometry-audit-r3.json`. Méthode vectorielle autorisée, propositions visuelles encore à valider.

Assets de marque, 2026-10-13 : [BUG-0009 marques fantômes](BUG-0009-branding-artifacts.md) : VERIFIED par inspection directe des PNG avant/après pour été clair et automne sombre. Pas un défaut de la galerie ou du runtime ; aucun test applicatif revendiqué pour ces retouches. Le logo approuvé et les six variantes non concernées sont inchangés.

Implémentation 2026-10-13 : [BUG-0001 fournisseur TLS](BUG-0001-tls-provider.md) et [BUG-0002 autorité HTTP/2](BUG-0002-http2-authority.md) : corrections vérifiées par tests et smoke réel de configuration ; [BUG-0003 fixture extraite](BUG-0003-extracted-fixture.md) : VERIFIED par 64 tests catalogue. Les défauts de revue du candidat domaine initial ne sont pas effacés ; nouvelles preuves dans `docs/implementation/` et rapports à consulter avant intégration finale.

Lot interface, source `c2a9d72` : [BUG-0004 type TLS reqwest](BUG-0004-reqwest-tls-type.md), [BUG-0005 rollback profil](BUG-0005-profile-context.md), [BUG-0006 mutations tardives](BUG-0006-late-scan-mutations.md), [BUG-0007 snapshot/identités](BUG-0007-scan-identity-atomicity.md), [BUG-0008 lecture SQLite](BUG-0008-api-read-errors.md) : VERIFIED par régressions, 124 tests et revues indépendantes r3. Preuves exactes dans INTERFACE-EVIDENCE ; rapports précédents conservés. Remplacement exceptionnel de clé affiché dans un bandeau persistant, erreurs API sûres/typées ; confiance provisoire annulée après connexion initiale échouée.

LOW ouverts, propriétaire /root : UISEC-002 (fsync du parent des réglages client) et SRV-SEC-005 (réconciliation après fsync serveur en échec). Non bloquants pour ce lot de développement selon revue sécurité r3, mais à traiter avant remise globale ; pas présentés comme corrigés. Pas de nouvelle acceptation de risque.

Créer `BUG-NNNN-titre.md` : date, environnement/matériel/réseau, branche/commit exacts, attendu/réel, reproduction, hypothèses numérotées avec une seule expérience falsifiable à la fois, résultat, cause confirmée ou inconnue, correction et commit, tests de régression, rollback, exigences/ADR concernés. Limite : cinq hypothèses non confirmées puis retour à l'utilisateur. Ne pas appliquer une correction spéculative à une dépendance externe indisponible.

Statuts : OPEN, INVESTIGATING, ROOT_CAUSE_CONFIRMED, FIXED, VERIFIED, CLOSED. « Non reproduit » n'est pas « corrigé ». Les preuves originales ne sont pas effacées ; une correction ajoute un lien. Ajouter ici le résumé consultable, sans enregistrer secrets ou contenu des profils.
## Correctifs série / fin de lecture — 16 octobre 2026

[BUG0085](BUG-0085-series-season-card-layout.md) : saison non vue ouverte, grille et fiche série réorganisée, corrigé localement. [BUG0086](BUG-0086-terminal-playback-return.md) : fin du dernier élément et conservation du contexte de lecture, corrigé localement. [BUG0087](BUG-0087-watched-eof-local-refresh.md) : rafraîchissement personnel à EOF et exception courte approuvée corrigés localement. Preuves synthétiques ciblées, pas de nouveau test vidéo réel ou déploiement Pi/web.
- [BUG-0070](BUG-0070-web-player-stale-title-while-opening.md) — client web : titre de la vidéo précédente pendant l'ouverture de la suivante ; FIXED (branche `codex/web-pwa`).
- [BUG-0071](BUG-0071-web-audio-missing-ac3-eac3.md) — client web : pas de son sur les films AC-3/E-AC-3 ; FIXED (branche `codex/web-pwa`), à vérifier sur le Pi.
