# État d'implémentation — remise acceptée avec réserves

Remise locale effective : nouveau bundle9277ac4 et source/API/documentation dans ChillCenter. **Pi non mis à jour** : protection d'exécution exige nouvelle confirmation informée avant stop/backup/replace/restart ; [REL-UX-API-001](../releases/REL-UX-API-001.md). Ancien serveur R7 actif ; ne pas activer nouveau client contre lui avant bascule. Stage binaire prêt/hash validé, données du foyer et NAS préservés. Source/softwaregatesAPPROVED, livegate du nouveau Pi non acquis.

Dernier lot14octobre : interface CC-UX-002 et API2026-10-14 intégrées et approuvées sur9277ac4, [gate](../gates/G-DEV-CC-UX-API-001.md), 289Rust/47Node/clippy/OpenAPI et deux vrais parcours natifs frais PASS. Nouveau bundle75Mio/48dylibs et serveurARM64 reconstruits ; mise à jour Pi/publication puis vérification distincte en cours, pas encore une remise vérifiée dans ce paragraphe. Catalogue continu sans pages visibles, navigation séries et création sagas/listes, clé TMDB UI/worker borné, conseil filtré en lecture seule, vraie API datée. Les paragraphes suivants sont historiques, non une liste actuelle de fonctions absentes. Fournisseurs et NFR matériels demeurent à qualifier.

Mise à jourPi14octobre : serveur ARM64 actif sur192.168.1.42:8443, TLS/confinement/NASRO validés indépendamment, scan2496conservé. Essaisréels2HD30min/3fluxDune+72030min/2Dune10minterminés : **Dune2à3échoueprogression<80%, à2passeavecstallsdébut** ; capacitéfluide3nonvalidée, pasdiagnosticdecausesupposé. RSSPimax24,36Mio/swap0/PIDstable. QA3profilsretirés, vidéosconservées. [REL-PI-001](../releases/REL-PI-001.md) et [preuvesmédias](../releases/REL-PI-001-media-evidence.md) gouvernent les derniers gates/limites ; les paragraphes ci-dessous conservent leurs checkpoints historiques, pas une promesse133ACtousPASS.

ClientMac624f244 : nouveau [tamponconfigurable](../releases/REL-MAC-CACHE-001.md), Administration16–512Mioavant/0–256arrière,défaut128/64,cibleavance60s/retourvariable≈30s. StockageparMac, effetprochainlancement,passerveur/profil ; revuescode/tests/ADR/sécuritéAPPROVED,259Rust/27Node/clippyPASSetbundle75Mio/signature/loader/API2.5PASS. Ancienbundle126conservé ; Pi source126/R7inchangé. Testsgraphiqueshumains/RSSmax/backseeknorefetchnonexécutés ; matérialitéNAS/Wi-Fi ne se déduit pas de ces succès logiciels.

## Situation actuelle (2026-10-14)

La suite du document est le checkpoint historique du13 octobre, pas la liste actuelle des fonctions manquantes. Source logicielle figée `126da0af167ee62588d92d90297f2dfc51962676`, carrier documentaire sans changement de source, remise locale par fast-forward gardé. [COMPLETION-EVIDENCE](COMPLETION-EVIDENCE.md) décrit la remise et les réserves ; [COMPLETION-PLAN](COMPLETION-PLAN.md) gouverne les33 stories, G-03 approuvé, sans nouvelle discovery.

Lecteur natif Tauri visible, pistes/contrôles, reprise/crédit réel/EOF, transfert entre deux vrais processus Tauri et nouvelle saison/vitesse ont passé les parcours de fixtures. Catalogue groupé48/page, administration/TMDB/sagas/playlists, ambiances vectorisées et archives/import explicite sont intégrés. Sauvegarde/restauration opérationnelle offline testée avec crash/récupération ; CLI portable Cinéfolio lecture seule implémenté et testé. Réassociation probable/full unique vérifiée dans Tauri, affiches saga raccordées. IA native + aperçu/confirmation/recherche/isolation vérifiés avec CLI synthétique : [preuves](NATIVE-ADMIN-AI-EVIDENCE.md). Les revues historiques ne valent pas approval de ces nouveaux commits.

Vérifications finales :255 tests Rust,24Node,7packaging PASS ; strictclippy PASS. Les deux LOW de durabilité sont corrigés avec injection d'erreur (BUG-0031), non effacés des rapports historiques. Bundle final75MiB libmpv/FFmpeg autonomearm64 signéad-hoc etloaderPASS ; codeAPPROVABLE/testsAPPROVABLE_SCOPED/sécuritéAPPROVABLE au SHAexact126da0a.0037/38etlangues0039fermés parRED/GREEN etTauri. Inventaire133AC lié aux preuves, pas133PASS. Pi/LinuxNASprêts maisChillnoninstallé ; SSHadminrépond puisagentsignfailed, inspectionenattentedusocketcontrôle. NOT_RUN LinuxARM/systemd/NAS/Wi-Fi/veilleOS/fournisseurs/son/accessibilité humains. G-04 demandevalidationlogicielleconditionnelle, aucune performance Pi ou installation foyer revendiquée.

## Checkpoint historique conservé

2026-10-13 ; G-03 approuvé, G-04 non remis. Source du lot interface : `c2a9d72d634cc7282eb8a20ca9fa138ed92e2fec`, base `fe85cea00da8a7e1f35abf0a5f057907eb235b47`. Les exigences restent CC-STORIES-001/133 AC. La documentation de remise ne modifie pas le code source revu.

## Utilisable dans ce lot

- Application Tauri Mac Silicon avec façade statique sans framework dans `ui/connected/`. Les anciennes sources Cinéfolio dans `ui/` sont préservées.
- Choix DNS/IP/port HTTPS modifiable, première approbation ou refus SPKI, reconnexion avec dernier profil par serveur/Mac, changement en haut à droite. Le remplacement exceptionnel après expiration est signalé par un bandeau persistant ; une connexion échouée ne mémorise pas de confiance provisoire.
- Création, renommage CAS et suppression confirmée de profils ; administration accessible avec zéro profil. Changement de profil avec rollback sur erreur ; notes, vu et préférences audio/sous-titres/enchaînement individuels. Langues initiales : anglais.
- Dossier Linux configurable dans Administration ; scan serveur unique en sous-processus borné. Catalogue et identités stables sont validés atomiquement sur un writer SQLite distinct. Les requêtes interactives annulées ou expirées ne sont pas appliquées tardivement. Aucun montage NAS côté Mac.
- Catalogue paginé (50 éléments, plafond API100), recherche par titre/catégorie, grille/liste/détails relatifs, notes et vu manuels. **Scan uniquement dans Administration** ; Actualiser recharge la liste sans parcourir le NAS. Affiches provisoires à initiale tant que les routes artwork ne sont pas raccordées.
- Paquet de développement `artifacts/macos/Chill Center.app` : arm64, signature ad-hoc, aucune dylib Homebrew nécessaire à ce lot sans lecteur. Ce n'est pas une distribution notarized ni le bundle final du lecteur. Reproduction : `bash scripts/package-macos.sh /chemin/absolu/nouveau.app` ; le script refuse d'écraser un paquet existant.

## Fondations conservées

Lot complémentaire CC-PLAY-001, source `b4100a37f985943fa1932bd98a2ec99d70b6c044` : API baux30s/heartbeat, flux Range sans transcodage, progression/reprise par profil et helpers client Rust. Capacité bornée, idle/révocation indépendante de la consommation du flux. 146 tests/build/clippy PASS et vraie MP4 HTTPS/libmpv headless jusqu'à EOF, reprise second appareil simulé ; trois revues indépendantes r2 APPROVED. Détails [PLAYBACK-EVIDENCE](PLAYBACK-EVIDENCE.md). **Pas encore de lecteur visible dans l'application Tauri** ; paquet .app précédent inchangé.

Catalogue/parseurs/TMDB/affiches/IA Cinéfolio extraits ; store SQLite personnel (progression, vitesses par série, playlists et réassociation explicitement confirmée). Transport Rust JSON/Range lié session-média-génération. Adaptateur libmpv : preuve headless et rendu AppKit/NSOpenGL/FBO isolé avec vidéo générée à3062ffe ; pas encore de surface native Tauri raccordée.

## Vérification du lot

Suite locale124/124 (catalogue65, client18, domaine11, lecteur10, serveur20), clippy sans warnings, syntaxe JS/scripts, builds debug/release, plist/signature/dépendances du paquet ; trois vrais parcours webview → Rust → HTTPS loopback. Les rapports indépendants r3 sont liés au SHA c2a9d72 dans `docs/reviews/`. Les rapports r1/r2 défavorables ou limités sont conservés.

Détail, commandes et limites : [INTERFACE-EVIDENCE](INTERFACE-EVIDENCE.md). Aucun de ces résultats n'approuve l'application entière ou les133 AC.

## Restes / pas prouvé

- Lecteur Tauri, fullscreen/redimensionnement, routes sous-titres externes, transfert confirmé/SSE, progression/heartbeat automatiques et échéance locale/enchaînement : à raccorder. Routes médias et reprise serveur livrées dans CC-PLAY-001, sans UI lecteur. La fixture textuelle historique du lot interface reste sans valeur pour la lecture ; le nouveau smoke utilise une vraie MP4. Matrice MKV/AVI/pistes externes et bundle libmpv final non livrés.
- Métadonnées/TMDB/affiches/édition/groupements/sagas/playlists et UI correspondante : à raccorder. Les méthodes héritées ne prouvent pas ce nouveau parcours.
- Export ZIP/affiches streaming et migration/import Cinéfolio complet : à faire. Import mutatif désactivé jusqu'à un plan de conflits atomique ; JSON de compatibilité1MiB préliminaire.
- IA client avec aperçu/validation/révisions, petit bouton et sans dictée ; ambiances auto/manuelles/désactivables : à faire.
- Build/déploiement Linux ARM/systemd/Pi/NAS/Wi-Fi/2–3 flux/RSS/CPU : NOT_RUN. Pi toujours non prêt ; IP192.168.1.42 et montage/srv/volumes/video restent des exemples configurables. Aucun SSH supposé ou déploiement sur le réseau du foyer.
- LOW ouverts : UISEC-002 (durabilité fsync parent des réglages client), SRV-SEC-005 (réconciliation serveur mémoire/disque après fsync en échec). Propriétaire /root ; à traiter pour la remise finale, pas présentés comme corrigés. L'export de compatibilité borné seulement après allocation reste un risque de ressources.

## Reprise et retour

Lot enregistré localement dans ChillCenter, branche `codex/interface-server`, origin GitHub inchangé ; pas de push. Source Cinéfolio, données et NAS intacts. `MANIFEST.sha256` régénéré sur les fichiers suivis, manifestes historiques conservés ; bundle ignoré accompagné de son hash dans les preuves.

Prochaine implémentation autorisée : surface lecteur Tauri et commandes UI sur les nouvelles routes, heartbeat/progression automatiques/échéance locale, puis transfert/SSE/sous-titres/enchaînement et autres stories selon le DAG. Lire contexte, rapports, bugs et contrats avant reprise. Ne pas recommencer discovery ni demander G-04 avant la livraison complète. Ce checkpoint n'annonce pas de travail en arrière-plan.

Rollback réversible : repartir du checkpoint fe85cea dans une nouvelle branche, relancer le binaire précédent ; sauvegarder base/réglages avant future migration. Ne pas reset/supprimer des changements utilisateur. Aucun format de base irréversible introduit par ce lot.
