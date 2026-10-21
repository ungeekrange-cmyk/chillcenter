# Contexte du projet

## 21 octobre 2026 — intégration main et maintenance OS hebdomadaire

Demande humaine distincte : commit/fusion main et cron de mise à jour des
paquets Linux suivi d'un redémarrage, dimanche 03:00 heure locale du Pi.
Source de maintenance `6abfc71`, tests finaux `1811c93`, trois revues R3
indépendantes APPROVED. Les refus R1/R2 restent conservés : assertions renforcées,
puis attente/cleanup de processus du test TERM corrigés; 9/9 tests finaux PASS.
BUG0128 conserve ses revues exactes sur `a6f0fc0` et ses preuves VM historiques.

Installation des seuls trois fichiers de maintenance OS effectuée le
21 octobre vers15:26UTC, après sauvegarde privée cron/APT et empreintes.
Syntaxe cron et systemd réels PASS, service de maintenance inactif/jamais lancé,
cron actif; premier passage prévu dimanche25octobre03:00Europe/Dublin(GMT).
Binaire/configuration/build web, PID du serveur et inventaire des paquets
inchangés. Aucune mise à jour de paquet, aucun redémarrage immédiat ni accès NAS.
Les mises à jour de sécurité quotidiennes existantes sont conservées.
Voir [le dossier FR/EN](releases/REL-OS-PACKAGES-20261021.md) et la
[vérification indépendante APPROVED_SCOPED](releases/REL-OS-PACKAGES-20261021-verification.md).
Première exécution APT réelle, redémarrage et contrôle après boot NOT_RUN.
Fusion main fast-forward0122c22→c1104eb publiée et relue sur GitHub;
porteur documentaire suivant sans changement des octets source approuvés.

**Ne pas confondre maintenance Linux et mises à jour Chill Center :**
serveur/web restent0.7.2, aucune nouvelle release ni paquet signé publié.
Le helper/planificateur applicatif de l'installation legacy reste non activé;
la fusion main seule ne l'installe pas. BUG0128 corrige les outils opérateurs
du dépôt, qui ne sont pas déployés implicitement par cette maintenance OS.

Nettoyage local supplémentaire demandé : huit caches Cargo externes et debug
courant supprimés, **40,96Go libres supplémentaires observés immédiatement**;
worktree38,25→4,60Go, sources/Git/worktrees/release/artifacts conservés.
[Mesures et limites](storage/STORAGE-CLEANUP-2026-10-21-02.md). Les checkpoints
suivants sont historiques et conservent leur périmètre d'autorisation initial.

## BUG0128 — correctif local,21octobre2026

Branche `codex/bug0128-durable-operator`, base `0122c22`. Verrou noyau hérité
par l'opérateur et ses sauvegardes, garde de récupération interne, lancement
détaché avec UUID, journal privé et reçu atomique durable. PID/boot/démarrage et
incertitude après perte du superviseur explicités. Voir [BUG0128](debug/BUG-0128-operator-transport-and-recovery-overlap.md)
et [le guide FR/EN](operations/LEGACY-DEPLOYMENT-JOBS.md). Tests d'interruption
Linux et vraie coupure SSH uniquement en VM locale isolée :20 tests Linux PASS,
preuve SSH réelle PASS,23 tests docs PASS. Source`a6f0fc0`, revues indépendantes
code/tests/sécurité APPROVED. Correctif VERIFIED_ISOLATED, pas déployé.
Aucune connexion Pi/NAS, aucune version publiée, tag, main ou configuration réelle
modifiée. Le checkpoint publié ci-dessous reste historique et effectif au foyer.

Nettoyage distinct explicitement confirmé : quatre caches Cargo externes
supprimés après revalidation, **15,47Go d'espace libre supplémentaire mesuré**.
Worktree courant/cache, sources/branches/historique, autres worktrees, versions
livrées et sauvegardes conservés ; [audit et reçu](storage/STORAGE-AUDIT-2026-10-21.md).

## v0.7.2 publiée — 21 octobre 2026

[BUG0127](debug/BUG-0127-component-check-helper-coupling.md), source exacte
`a38d4b1`, branche `codex/update-check-controls`. Recherche Mac/web des composants
serveur/web/thèmes découplée du helper root ; nouveau worker non privilégié pour
les installations legacy, métadonnées/signatures seulement, aucune installation
ou archive téléchargée. API datée inchangée, capacité additive `check_available`.
Bouton Installer masqué sans version stable strictement plus récente détectée.
Mac/serveur/web0.7.2/séquence3 ; thèmes conservés0.7.1/séquence2.

147 tests serveur PASS/1NFR ignoré,198Node PASS,23docs PASS, Clippy et DOM partagé
Mac/web PASS ; vraie QA HTTPS locale avec GitHub public concluant, serveur QA
arrêté. Revue sécurité indépendante a38d4b1 APPROVED. Bundle/DMG Mac0.7.2 build11
construits/vérifiés, macOS27+ ; les empreintes et limites figurent dans le BUG.
Publication v0.7.2 Latest à12:38:03UTC, quatre releases Mac/serveur/web/récapitulatif,
tags exacts a38d4b1. Main contient la livraison et ses revues (`531bbb7`).
Contrôle indépendant des22 téléchargements publics, sommes, signatures et
provenance PASS ; CodeQL cinq langages et garde dépendances livrées PASS.
Voir [le dossier de livraison](releases/REL-0.7.2.md) et
[la vérification indépendante](releases/REL-0.7.2-verification.md).
Serveur Pi0.7.2 exactSHA`cdde3ed0…b4cadea`, web0.7.2/build`ad12a307191b`
actifs après sauvegarde privée cohérente. Vérification indépendante APPROVED :
TLS/API/catalogue, empreintes protégées, base schéma3/2496 médias, sauvegarde1924
entrées, ancienne paire de rollback, trois recherches serveur/web/thèmes
`NO_UPDATE`, répétition en cache et installation refusée503 sans helper PASS.
Comparaison SQLite cohérente avec WAL :29tables identiques à la sauvegarde,
service stable et aucun nouvel échec après le démarrage effectif.
NAS, réglages, TLS, unités/drop-ins et absence de helper/planification conservés.
Incident de supervision/récupération concurrente tracé en
[BUG0128](debug/BUG-0128-operator-transport-and-recovery-overlap.md) : sauvegarde
toujours active malgré reçu SSH absent, reprise prématurée du service puis
convergence vers la paire autorisée. Ne pas réutiliser cet opérateur sans suivi
distant durable et exclusion mutuelle. État final établi par contrôle indépendant,
pas par un code de sortie SSH reconstruit. Aucun nouveau test du lecteur/GUI natif.
Le Mac doit être installé via le nouveau DMG pour bénéficier du correctif UI.

## v0.7.1 — contrôles finaux/publication,21octobre2026

**Checkpoint actualisé :** source des paquets `c603f29`, revues indépendantes
favorables, artefacts signés vérifiés. Serveur Pi 0.7.1 installé après sauvegarde
privée et exception AF_NETLINK approuvée ; vérification indépendante
`APPROVED_SCOPED` (TLS/API, données/réglages, web inchangé, mDNS CLI réel).
Client web au foyer préservé en 0.7.0 ; aucun NAS, réglage applicatif ou TLS modifié.
Mac GUI/consentement réseau local non retesté. Les cinq releases GitHub sont
publiées ; v0.7.1 Latest à10:45:54UTC, 22 pièces jointes récapitulatives.
Téléchargements publics vérifiés indépendamment : 22 fichiers identiques à la
livraison signée, tags exacts, sommes/signatures/provenance PASS, gate APPROVED.

Amélioration sécurité demandée : commit `61da27f`, CI/outillage/docs seulement,
sans delta runtime/lockfile. Dependabot Cargo/Actions hebdomadaire, garde ciblé
des deux graphes livrés, actions épinglées, lecture seule ; revue indépendante,
27 tests locaux et run GitHub39586395359 PASS. Avis glib moyen réel laissé ouvert,
absent Mac/serveur ; client lourd Linux non distribué bloqué avant correctif.
CodeQL39586394861 PASS dans les cinq langages, dont Rust ; cinq alertes de
permissions conservées ouvertes selon choix humain, aucun nouveau constat ouvert.
Voir le [dossier courant](releases/REL-0.7.1-delivery.md). La préparation décrite
ci-dessous est historique, pas l'état effectif actuel.

Autorisation humaine reçue pour la seule exception AF_NETLINK, sauvegarde privée
et mise à jour serveur. SSH renouvelé ; précontrôle Pi en lecture seule PASS,
aucune interruption ou modification réelle encore effectuée. Web installé0.7.0
compatible, ses fichiers sont exclus de l'opération ; nouveaux paquets0.7.1
Mac/serveur/web/thèmes préparés pour GitHub, sans remplacement du web au foyer.
Ambiances Pâques/Saint-Valentin et silhouettes unies approuvées puis intégrées.
Rendu2 accepte les anciens packs1 ; nouveau pack signé exige rendu2.
BUG0123/0124 corrigés ; contrôle visuel intégré PASS,337Node PASS, suite Rust
complète PASS après correction fixture, deux NFR historiques ignorés.
GitHub public : secrets/push protection/Dependabot activés, CodeQL étendu cinq
langages dont Rust, premier scan terminé. Alertes qualifiées dans le rapport
sécurité ; clôtures et nouveau scan source restent à vérifier. Pas de visibilité
ou branche par défaut changée, ni clé privée ajoutée au Git.
Les états du20octobre ci-dessous sont historiques. Le dossier de livraison
consignera séparément publication, installation et vérification réelles.

## v0.7.1 en préparation — 20 octobre 2026

**Point d’arrêt actuel : autorisation attendue pour ADR0032 / BUG0119.**
Les revues R4 sont favorables et le DMG est vérifié ; upload privé et preflight
Pi seulement. Aucune sauvegarde fraîche, modification d’unité, interruption ou
installation0.7.1 n’a été exécutée. Le filtre réseau existant interdit AF_NETLINK,
nécessaire à la lecture des interfaces ; les deux probes éphémères confirment
errno97 puis succès en ajoutant cette seule famille. L’unité réelle et le PID
restent inchangés. Voir [le dossier courant](releases/REL-0.7.1-delivery.md).

Demande explicite : découverte mDNS au lancement Mac, saisie manuelle masquée
derrière un lien, API unique2026-10-14 avec suppression de l’aliasv1. Mac/serveur
0.7.1 construits ; aucun remplacement Pi de ce lot encore exécuté à ce stade.
Le web0.7.0 reste compatible (requêtes datées, tests182/182), donc ses octets
n’ont pas à être remplacés. NAS, réglages et TLS restent protégés.
Runtime figé `fdb80b5`, Linux SHA256 `8cf14421630cfd552e3af4ed340de107c0570046cc5ecb2784936ca8c552491c`.
DMG Mac0.7.1 SHA256 `a752700170d40b137d74597beb073f29bb4f5aebcd157c6f9ee244c9ac908bfd`.
L’opérateur de déploiement séparé poursuit ses revues ; ne pas confondre un
build validé avec un déploiement ou une découverte LAN réellement vérifiée.
Les rapports R1–R3 conservent leurs refus historiques et leurs périmètres exacts.

Pâques/Saint-Valentin : fonds approuvés, animations proposées seulement dans
[la maquette](design/concepts/2026-10-20/events.html), aucun pack runtime modifié.
Pâques occidentales calculées chaque année,J−30 ; Saint-Valentin,J−14.
Retour à la saison le lendemain de l’événement. ADR0030/0031 décrivent ce lot.

## Pi : serveur et web v0.7 installés — 20 octobre 2026

Demande humaine explicite : sauvegarder puis mettre à jour les deux composants,
sans NAS ni réglages existants. Paquets publiés inchangés, source `3a043334` :
serveur `0.7.0` SHA256 `68d526cd3f3af7cee9b07b06c62f46260656dda7cccc4c12994dee85431771e5`,
web `0.7.0` build `a1c12ba4a582`. Seconde tentative `deployment=PASS`, commencée
à `2026-10-20T20:26:34Z`. Sauvegarde privée cohérente
`4cb8e869-7990-4e35-8311-b2ded8d4bd74`, anciennes versions conservées. Revue
indépendante de l'opérateur corrigé `815b523` APPROVED, sept tests ciblés et
preuve réelle Linux de nettoyage des processus PASS.

Le premier essai a été restauré automatiquement : [BUG0118](debug/BUG-0118-pi-unit-runtime-fingerprint.md),
PID/heure systemd comparés à tort comme configuration. La restauration a été
vérifiée indépendamment ; ne pas qualifier cet essai de succès. Référence
logique correcte : 2 496 vidéos avec WAL, et non 2 497 issues du premier relevé
sans WAL. Contrôle indépendant final **APPROVED_SCOPED**, gate **VERIFIED** au
dernier relevé `2026-10-20T20:37:32Z` : binaire actif exact, 99 fichiers installés,
97 fichiers servis et deux notices MIME-deny attendues, HTTPS strict 443/8443,
API datée/v1, OpenAPI, catalogue/filtres et 29 empreintes sémantiques inchangées.
PID stable, zéro redémarrage inattendu et zéro avertissement/erreur. Le GET de
versions autorisé a seulement initialisé un verrou vide privé de mise à jour ;
aucune politique existante ni préférence modifiée.
[Dossier FR/EN](releases/REL-PI-V0.7-deployment.md),
[vérification indépendante](releases/REL-PI-V0.7-verification.md).

Aucun Mac, paquet/tag GitHub, unité, compte/groupe, réglage, TLS ou NAS modifié.
L'installateur initial n'a pas été utilisé. Le helper privilégié et le timer de
mise à jour ne sont pas installés/activés sur cette installation legacy ; ne pas
les ajouter sans un lot distinct. Les secrets de sauvegarde restent sur le Pi.

English: the unchanged published server/web 0.7.0 payloads are installed after a
fresh private backup. Automatic rollback of the first false-positive attempt was
independently verified; that failure remains recorded. Existing data/settings,
NAS, Mac and GitHub releases are preserved. Independent post-deployment
verification is `APPROVED_SCOPED`, gate `VERIFIED`; API, strict TLS, exact web
assets, data/settings, backup and stability passed. Physical Safari/PWA install
and playback were not retested. The root updater/timer is deliberately not
activated.

## v0.7 publiée et vérifiée sur GitHub — 20 octobre 2026

Publication effective, pas un déploiement Pi. Source immuable des cinq tags et
manifestes : `3a043334c5c3c676251ab34bf8bab2e4bfb8df4f`. Le delta depuis la source
revue `3e174b9` ne contient que README/docs/rapports/preuves ; Mac reconstruit
pour embarquer le SHA réel du porteur. main a été avancée sans reset/force.
Visibilité PUBLIC et branche par défaut main conservées, v0.6 intacte.

Releases stables : `mac-v0.7.0`, `server-v0.7.0`, `web-v0.7.0`, `themes-v0.7.0`
et **v0.7 Latest**. IDs : 418723980/418724262/418724452/418724554/418725680.
Pièces jointes : 5/14/4/4/22 ; tailles, états uploaded et SHA256 calculés par
GitHub tous comparés aux fichiers locaux avant publication. Une interruption
réseau avant création du brouillon récapitulatif a été vérifiée puis reprise
sans remplacer les quatre brouillons complets. Notes FR/EN et DMG en évidence.

Quatre archives de distribution, manifestes des composants, provenance et
SHA256SUMS signés avec la clé18FD790B48C202F8 hors Git. Aucun secret/certificat
privé/IP du foyer ajouté. Paquets définitifs dans
`/private/tmp/chillcenter-v07-final.E670In/delivery`, Mac DMG SHA256
`7efd143d832b4882c1fb97d79fd9b234e04c863440151c0dabbc4ab22a2ded75`.
Serveur ARM64 glibc2.31+, web builda1c12ba4a582, thèmes0.7.0, Mac0.7.0/build9
macOS27+. Signatures ad hoc/Minisign, pas de notarisation Apple.

Les vérifications locales finales sont PASS : DMG réel monté en lecture seule,
app exacte/signée, TAR Mac sans AppleDouble, ZIP données et archives complètes
serveur/web/thèmes, tous manifestes et octets signés. Vérification indépendante
des téléchargements publics **APPROVED**, gate de release **VERIFIED**, par
`/root/v07_publication_verifier` à18:32:23UTC :
[rapport indépendant FR/EN](releases/REL-V0.7-verification.md). Les cinq releases,
signatures, octets publics et DMG réel sont conformes ; le téléchargement
partiel initial a été rejeté puis refait dans un nouveau dossier complet.
Le rapport constate main à3a043334 à cette heure ; le porteur documentaire
suivant ajoute seulement ce compte rendu, sans changer tags/paquets/runtime.
Ne pas relancer le lecteur ni réécrire les assets stables : une correction
ultérieure exige une nouvelle version/séquence signée.
Serveur HTTP loopback QA arrêté après identification ; anciens fichiers et
montage DMG utilisateur conservés. **Aucun Pi, SSH, NAS ou web déployé contacté.**
Les checkpoints « non publié » ci-dessous sont historiques.

## v0.7 prête à publier — 20 octobre 2026

Les derniers gates sont clos sur `3e174b9e6ed89e08f67e763c3b30028c15767080` :
code APPROVED_SCOPED, sécurité APPROVED, tests APPROVED. Annulation native
0.7.2 réellement réussie, confirmée humainement, sortie0 ; un GET de manifeste
supplémentaire, aucun paquet0.7.2, arbre/exécutable/marqueur/signature inchangés.
Les trois anciennes acceptations restent dans le journal, non requalifiées.
435Rust/359Node PASS, deux NFR existants ignorés, packagingPython19 PASS.
Les checkpoints bloqués ci-dessous sont historiques, ne pas les relancer.

Publication explicitement demandée, sans Pi. Dossier de livraison neuf
`/private/tmp/chillcenter-v07-final.E670In`, build web a1c12ba4a582/98fichiers.
Finaliser le commit porteur des seules docs/rapports, reconstruire le Mac pour
son SHA réel, signer tous les flux avant création des tags/releases indépendants,
puis publier v0.7 avec DMG, serveurARM64, web, thèmes, provenance et sommes signées.
Le script local `/private/tmp/chillcenter-v07-final-artifacts.mjs` prévoit le DMG.
Conserver main comme défaut, visibilité PUBLIC existante et release/tagv0.6.
Ni SSH, ni Pi, ni NAS, ni web déployé. Aucun test supplémentaire du lecteur requis.

## DMG Mac demandé — 20 octobre 2026

Installation manuelle désormais préparée au format standard DMG, application
et raccourci Applications, instructions FR/EN ; updater interne `.app.tar.gz`
inchangé. Générateur `scripts/package-macos-dmg.py`, cinq tests ciblés PASS,
refus d'écrasement atomique et contrôle de signature avant création. Aucun
lancement, remplacement ou installation d'application par le générateur.

Candidat local :
`/private/tmp/chillcenter-v07-dmg-ac52533/Chill-Center-0.7.0-macos-arm64.dmg`,
46 881 214 octets, SHA256
`7c352bdb708b5d53a6f8a332e7ff33a77c54017043c90a0427d28e64902c47a5`.
App embarquée issue de `ac52533`, version0.7.0/build9, macOS27 minimum.
`hdiutil verify` PASS ; montage réel lecture seule, raccourci vers /Applications,
signature stricte et arborescence complète identique à l'app source vérifiés,
puis volume éjecté. Hash de l'arbre applicatif
`86a82895b4faceaa4edb06c16282714cb47a1062cca9667a503d09f9a9a088fd`.
Cet essai de packaging ne clôt pas le gate d'annulation native ci-dessous.
Pas de publication, fusion main, signature de production ou contact Pi.
README/guides FR/EN et assemblage local de release prévoient le DMG en évidence.

## Validation native v0.7 — 20 octobre 2026

Source runtime/packaging figée `ac5253328da3bd2cb6a65e9913c0bbe5b8d3d33b` :
revues indépendantes code APPROVED_SCOPED et sécurité APPROVED ; 435 tests Rust
PASS, zéro échec, deux NFR existants ignorés ; Node359 PASS ; Python packaging14
PASS. Helper natif commun privé extrait sans changer l'ordre confirmation,
annulation et deuxième garde lecteur. Bundle neuf local
`/private/tmp/chillcenter-v07-minos-ac52533/Chill Center.app`, signature stricte,
48 dylibs et chargement libmpv vérifiés. Minimum réel **macOS 27+**, calculé sur
49 Mach-O avant signature (BUG0117). Ne pas livrer les anciens candidats macOS13.

Vrai SDK/AppKit : installation signée et redémarrage isolés réussis. Garde de
lecture avant toute requête et lecture commençant pendant le prompt réussies,
sans téléchargement ni changement du bundle. Nouveau consentement 0.7.2 bien
affiché, mais trois callbacks d'acceptation ont été reçus alors que le scénario
attend Annuler : le harness a refusé avant tout téléchargement/installation 0.7.2.
La question humaine sur le bouton réellement utilisé est en attente : ne pas
conclure à une erreur de clic ou à un bug du SDK sans cette précision.
La revue tests conserve uniquement l'oracle d'annulation 0.7.2 comme gate.
Aucune fenêtre QA encore ouverte ; ne pas relancer en boucle ni refaire le lecteur.

QA séparée : source `7efb31c1e196c53cfa231dbcf043bf6ff1bc75ef`, dossier
`/private/tmp/chillcenter-qa-native-updates.YNFPqL`, lanceur
`launch-isolated-qa.sh second_cancel`, journal `qa-native-updates.log`, compteur
`network.count`. Attendre la ligne PASS d'annulation, bundle inchangé et aucun
GET d'archive 0.7.2, puis verdict indépendant final. Le QA se ferme après son
redémarrage vérifié : absence de catalogue attendue, pas une panne de lancement.
Le défaut AppleDouble du TAR QA est corrigé (BUG0116), échec conservé.

**Aucune publication v0.7, signature avec clé de production, fusion main ou
installation Pi à ce stade.** README/guides de distribution et rapports sont
encore des modifications locales à intégrer après fermeture du gate ; préserver
leur état. Script d'assemblage local préparé hors Git :
`/private/tmp/chillcenter-v07-final-artifacts.mjs`. Produire ensuite les paquets
depuis la source propre finale, vérifier, signer puis publier les quatre flux
et v0.7 sans déplacer v0.6. Aucun contact Pi/NAS dans cette reprise.

## Correctifs v0.7 — checkpoint local, 19 octobre 2026

Source corrigée `3913267` (moteur revu `6041f4d`, delta configuration native) :
huit constats sécurité et quatre défauts fonctionnels traités, vraie VM Linux
ARM64/systemd utilisée sans contacter le Pi. Matrice TERM/KILL/occupation/reprise
et deux tests root PASS ; suite Rust433 PASS/2 NFR ignorés, Node359 PASS,
Clippy PASS. [Preuves et limites FR/EN](implementation/COMPONENT-UPDATES-FIXES-20261019.md).
Revues indépendantes `3913267` : code **APPROVED_SCOPED**, sécurité **APPROVED**,
huit HIGH clos. Revue tests **CHANGES_REQUESTED**, seul CU88-TEST-003 reste ouvert :
parcours natif AppKit consentement/installation/redémarrage. Le delta natif passe
8/8 tests SDK/crypto ; le vrai premier lancement QA avait révélé BUG0115, corrigé.
Le bundle Mac candidat 0.7.0/build9 est construit, signé ad hoc et vérifié
(48 dylibs arm64, libmpv API2.5). Paquets locaux non signés pour publication dans
`/private/tmp/chillcenter-v07-packages.aeUI7U`, source runtime `3913267`.
Le test QA isolé attend le clic « Annuler » ; lanceur
`/private/tmp/chillcenter-qa-native-updates.LiX6KA/launch-isolated-qa.sh`, journal
voisin `qa-native-updates.log`, état `PROMPT native consent queued`. Ce chemin
local sert uniquement à reprendre la validation ; ne pas le présenter comme livraison.
**Ne pas publier/annoncer v0.7 livrée avant ces gates.** Pi/NAS et web déployé
intacts ; versions v0.6 et tous les anciens livrables conservés.

## Mises à jour par composant — implémentation en cours, 19 octobre 2026

Demande utilisateur : thèmes indépendants, versions visibles Mac/web/serveur,
installateurs et releases GitHub, vérification/installation configurables,
serveur automatique à 02 h locale et Mac toujours « Installer maintenant ».
Le dépôt a été constaté PUBLIC (choix humain), main reste la branche par défaut.
La suppression de la référence au projet d'origine des README FR/EN est publiée
sur main à `ac85794`. Aucun runtime ni déploiement modifié.

[ADR0029 R2](architecture/decisions/ADR-0029-component-releases-updates.md) est
**ACCEPTED**, après validation humaine consignée dans le [gate R2](gates/G-ADR-0029-R2.md).
Sa [revue indépendante R2](architecture/decisions/reviews/ADR-0029-review-r2.md)
approuve la conception : nouvelle clé de
signature de releases (privée locale hors Git, publique embarquée), installateur
privilégié séparé, politique automatique 02 h, risques résiduels LAN sans auth.
Interruption forcée réservée à l'administrateur local ; installation normale
différée pendant lectures/travaux. Premier rejet R1 conservé ; ne pas l'effacer.
Contrat de calendrier/format dans [themes/README.md](../themes/README.md), avec
version anglaise. Packs raccordés avec repli embarqué, versions et réglages
indépendants en cours d'intégration sur branche isolée. Clé privée créée hors
Git après le gate ; seule la clé publique est embarquée. Tests Rust hors GUI
PASS, premiers tests des packs et de leur affichage PASS. Revue préparatoire de
sécurité : plusieurs P1 updater/installateur à fermer avant publication. Aucun
accord de livraison n'est implicite dans les tests de développement. Aucun tag
0.7, paquet public, nouveau bundle livré ou déploiement Pi à ce stade. Préserver
main, les anciens livrables et les travaux web indépendants.

Checkpoint source figée **22a38ff50879837c32fc8d5ac7c7200f35c8eaa1** :
**RELEASE_BLOCKED / CHANGES_REQUESTED**, huit HIGH Linux et quatre P1 fonctionnels
confirmés par revues indépendantes. [Rapport de travail FR/EN](implementation/COMPONENT-UPDATES-20261019.md),
[BUG0113](debug/BUG-0113-component-updater-release-gate.md) et
[revue sécurité](reviews/component-updates-security-22a38ff.md).
357/357 tests JS/docs et suite Rust sérialisée PASS (deux ignorés existants),
Clippy/format PASS. Deux échecs de timing lecteur lors de runs antérieurs restent
tracés dans [BUG0114](debug/BUG-0114-subtitle-timing-test-instability.md), pas corrigés
par assouplissement du test. Build web 6dd3d04c9203, 98 fichiers, non déployé.
Ne pas publier/installer le helper avant fermeture des défauts et preuves Linux
root/systemd ; les tests unitaires ne constituent pas cette preuve.
Bundle Mac de revue locale 0.7.0 build9 construit et vérifié (48 dylibs arm64,
signature ad hoc) ; **non approuvé pour livraison, non publié**. Voir le rapport
pour la provenance et les limites. Tous les anciens bundles restent conservés.

## Correctif web mobile R4 déployé — 17 octobre 2026

Source **fa13d44**, build **67773f7877d3**, assets seuls livrés **20:50:15UTC**
après sauvegarde privée vérifiée ; ancien arbre conservé. Bandeau plein écran
limité à2,5s sans réarmement sur poll, autre chemin WebKit dans le geste, safe
areas quatre côtés et correction d'arrondi de grille fractionnaire. **315/315
tests PASS**, QA mobile/desktop web PASS. Aucun moteur audio modifié, aucune
vidéo NAS relue. PID5420, zéro redémarrage, hashes binaire/config/unité/cert/SPKI
inchangés. URL `/?v=67773f7877d3` vérifiée depuis le LAN ; fermer l'ancien onglet.
**DEVICE_RETEST_PENDING** : Safari réel/encoches, canvas toujours potentiellement
refusé par le plein écran natif. Question apps iOS/Android sans store traitée en
faisabilité seulement : aucun nouveau client natif, certificat ou signature créé.
[Livraison FR/EN et preuves](releases/REL-WEB-MOBILE-004.md).
Post-vérification indépendante **APPROVED_SCOPED à20:51:07UTC**, manifeste81/81
et rollback validés. Transfert temporaire seul retiré ; archives conservées.

## Correctif web mobile R3 déployé — 17 octobre 2026

Source **acd7aeb**, build **a15795b5c4cd**. Repli AVI MP3 vers AAC dans le Worker
FFmpeg web, source/compiler épinglés ; aucun transcodage serveur. Actions des
vignettes catalogue masquées sur mobile. Plein écran dans le geste avec repli
vidéo iPhone ; limite canvas explicite, pas de faux succès. Recherche/catégories
déjà conservées, signalement non reproduit : test E2E de résultats ajouté.
**311/311 tests PASS**, trois QA Chromium PASS, dont vrai AVI local avec MP3
natif désactivé. Pas une preuve Safari physique ; **DEVICE_RETEST_PENDING**.
Archive immuable approuvée ; assets web seuls **déployés à19:31:14UTC** après
sauvegarde privée vérifiée, ancien arbre conservé. PID5420 inchangé, zéro
redémarrage ; binaire/configuration/unité/certificat/SPKI identiques au baseline.
Point d'entrée HTTP et URL `/?v=a15795b5c4cd` vérifiés depuis le LAN ; fermer
l'ancien onglet pour éviter une page en cache. Post-vérification indépendante
**APPROVED_SCOPED à19:31:50UTC**, 81/81 sommes et rollback vérifiés ; les
checkpoints ci-dessous sont historiques.
Mac, serveur, API, configuration et NAS inchangés ; aucun push GitHub.
[Livraison FR/EN et limites](releases/REL-WEB-MOBILE-003.md).

## Correctifs web mobile R2 déployés — 17 octobre 2026

Source **8b95033**, build **95ebee4fbe0a** : recherche/filtres derrière une loupe,
catalogue immédiat, rail gauche en paysage téléphone, outils série/saison repliés
après les épisodes. Repli ManagedMediaSource pour iPhone et reprise tactile du
lecteur préparé, sans nouveau bail, avec respect des pauses serveur.
**304/304 tests**, trois QA Chromium (interface, desktop, clips AVI/MKV réels
synthétiques) PASS. Ces tests ne prouvent pas WebKit natif : **DEVICE_RETEST_PENDING**,
en particulier le MP3 des AVI selon Safari ; aucune nouvelle conversion MP3/AAC.
Source et archive revues indépendamment. Assets seuls installés à18:07:25UTC
après sauvegarde privée ; post-vérification indépendante **APPROVED_SCOPED**.
PID5420 inchangé, zéro redémarrage ; binaire/configuration/unité/certificat/SPKI
inchangés, aucun accès aux vidéos NAS ni modification du client Mac.
Sauvegarde, ancien arbre et archive locale conservés ; transfert temporaire supprimé.
Ouvrir `/?v=95ebee4fbe0a` pour éviter une ancienne page en cache.
[Livraison FR/EN et preuves](releases/REL-WEB-MOBILE-002.md).
Les checkpoints ci-dessous sont historiques ; aucun push GitHub.

## Client web smartphone déployé — 17 octobre 2026

Concept R1 validé puis intégration exclusivement web, source **98ee3e3**,
build **b1722f00c0a6**. Navigation basse, grille virtuelle et fiches tactiles ;
plein écran réel demandé avant attente réseau, paysage selon capacités, repli
explicite. Desktop web conservé, aucun fichier commun Mac/serveur modifié.
**283/283 tests**, QA Chromium mobile et desktop PASS, documentation **24/24**.
Revue source/archive et vérification post-déploiement **APPROVED_SCOPED**.
Assets seuls remplacés à14:36:13UTC après sauvegarde privée complète ; PID5420
et0redémarrage, hashes binaire/configuration/unité/certificat/SPKI inchangés.
Archive locale et ancien arbre conservés, seul transfert temporaire supprimé.
HTTP sans port et applicatif répondent depuis le Mac ; sonde locale du Pi sur
le port80 distincte. Essais physiques Safari/iPhone, Android et vidéo restants.
[Livraison FR/EN, preuves et retour arrière](releases/REL-WEB-MOBILE-001.md).
Les versions et worktrees antérieurs sont conservés ; aucun push GitHub.

## Web seul déployé sur Pi — 17 octobre 2026

Autorisation explicite après la remise Mac/web : sauvegarder puis remplacer
uniquement les fichiers web. Build **1cce6dd6cd9d**, source **a13902e**, archive
SHA32176d8f…c885e4, opérateur revu indépendamment SHA b0baea0b…743996.
Installation12:56:36 Europe/Dublin, rc0, sauvegarde privée préalable et ancien
dossier conservés. Vérification indépendante **APPROVED_SCOPED**,79fichiers,
78sommes,56fichiers de backup identiques, API datée/v1 HTTP/HTTPS PASS.
PID5420 inchangé,0redémarrage ; binaire serveur6fec/configuration/unité/SPKI
identiques. Aucun accès aux vidéos ou écriture NAS. SHA certificat complet et
état de montage avant non relevés, pas d'invariance inventée sur ces éléments.
[Preuves FR/EN et retour arrière](releases/REL-WEB-PARITY-001-web-deployment.md).
Bundle Mac, tag/releasev0.6 et chantier web indépendant conservés ; aucun push.
Le checkpoint « non déployé » ci-dessous reste l'état antérieur de la compilation.

## Candidat suivant — bugs 82–84 et parité web

17 octobre2026 : demande de corriger BUG0082/0083/0084, reconstruire Mac et
adapter le web ; clarification explicite : **pas d'IA web**. Branche isolée
`codex/fix-82-84-web-parity`, bases Mac3b2e28b et webe6e23db intégrées par merge,
sans écraser l'autre worktree. Candidat0.6.1/build8, release/tagv0.6 préservés.
[Rapport FR/EN et limites](implementation/WEB-PARITY-20261017.md),
[remise](releases/REL-WEB-PARITY-001.md), ADR0025. Aucun accès/déploiement Pi/NAS.
Les checkpoints ci-dessous conservent les livraisons antérieures.

## Documentation de référence FR/EN — complément après v0.6

17 octobre 2026 : demande utilisateur « doc en anglais et en francais. ensuite merge ».
L'[index français](README.md) et l'[English index](README.en.md) donnent accès
aux neuf paires : README, contribution, sécurité, index, synthèse d'architecture,
guide API, notes v0.6 et deux guides de publication. Les journaux, ADR acceptés
et preuves historiques restent dans leur langue d'origine, sans réécriture.

Le guide API corrige dans les deux langues deux descriptions périmées, à partir
du code existant : origine web autorisée sur la même instance (y compris la
variante HTTP facultative) et bail conservé pendant la pause Mac (ADR0022).
Ces corrections sont documentaires, pas des changements de contrat ou de runtime.
Les 13 contrôles documentaires vérifient liens croisés, exemples API identiques,
endpoints, provenance/hashes v0.6 et garde-fous de confidentialité. Leur succès
ne remplace pas une revue sémantique de traduction ou un audit d'historique.

Branche `codex/docs-fr-en`, base `69da608`, intégration locale dans
`codex/player-streaming`. Le tag `v0.6` reste à `69da608` et le bundle signé
de source `bac22ca` est conservé. Aucun accès Pi/NAS, modification web, rebuild,
push ou publication n'est inclus.

## Référence actuelle — v0.6

17 octobre 2026 : validation utilisateur « validé commit en v0.6 et merge avec doc propre ».
Client Mac **0.6.0 / build 7**, bundle de référence
`artifacts/macos/Chill Center-v0.6.app`, tag local `v0.6`.
La [fiche de livraison](releases/REL-V0.6.md) fait autorité pour cette version,
ses preuves, ses limites et le retour arrière. Elle consolide le candidat validé
`73cbc8c` (source QA `095631a`) sans nouvelle modification fonctionnelle.
Source compilée `bac22ca`, 130/130 tests Node PASS, signature profonde et
48 bibliothèques ARM64 vérifiées ; SHA256 de l'exécutable
`7eb9dd979f473f1dc09309bdc314fe93c9890d8eabf6a42203c7a5232787e3f7`.

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

14 octobre 2026, demande utilisateur : reproduire le client Mac en web app/PWA servie par le Pi, sans authentification, calcul vidéo côté navigateur, serveur inchangé hors delta minimal. Travail isolé dans un worktree distinct pour ne pas gêner le client Mac. [ADR-0014 (anciennement ADR-0012)](architecture/decisions/ADR-0014-web-client-pwa.md) PROPOSED, [architecture web](architecture/WEB-CLIENT.md). Aucun déploiement, aucun push. Dépôt distant : `https://github.com/ungeekrange-cmyk/chillcenter.git`.

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
