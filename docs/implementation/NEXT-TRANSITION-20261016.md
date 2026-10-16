# Transition suivante — BUG0091,0092,0093

16 octobre2026. Demande explicite de corriger les trois défauts, remplaçant le précédent référencement seul. Source **8be19f461712c1260db2b21c3d8aba1cb59c3064**, branche `codex/fix-next-transition`, basebd4de81. Sept bugs précédents0078/0079/0080/0085/0086/0087/0088 clôturés sur confirmation explicite de l'utilisateur, sans refaire leurs parcours natifs.

## Diagnostic et correction ciblée

- **0091** : carte suivante toujours sombre, texte du bouton secondaire hérité de la palette claire. Surcharge du seul lecteur pour texte clair/bordure visible et mention « À suivre » lisible ; pas de changement de palette globale.
- **0092** : `launchNext` appelait `cancelNext`, qui écrivait le faux message d'annulation. RED déterministe avec commande retardée : « Lancement automatique annulé. » pendant un lancement réel. Arrêt de minuterie désormais distinct de l'annulation ; message « Lancement de la vidéo suivante… », verrou de proposition en vol et contrôles désactivés pendant cette commande. Une relance différée par le statut garde sa proposition et disparaît à l'annulation, au passage en arrière-plan ou au changement de contexte. Pas de relance de mutation hors ligne.
- **0093** : après fermeture de la surface native à EOF, la découpe DOM transparente laisse apparaître le fond clair. Préparation d'une seule affiche courante dans les8dernières secondes via le chargeur/cache existant, calque toujours transparent avant EOF, puis affiche floutée/assombrie sur toute la surface du lecteur. Repli sombre si pas d'image ou réponse tardive. Retrait et invalidation avant autre média/profil/navigation/fermeture ; aucune image de série précédente conservée. Ni nouvelle image générée, ni transcodage, ni préchargement de la vidéo suivante.

## Preuves exécutées sur la source figée

- `node --test ui/connected/player.dom.test.js` : **32/32 PASS**, dont échéances0/5/10/15/20/30/off, disponibilité, message pendant lancement retardé, double appel, annulation/arrière-plan/proposition remplacée derrière le statut, préparation8s et réinitialisation de l'affiche. La première version du test de slot occupé utilisait un délai0 déjà déclenché à la construction du harnais ; corrigée pour installer le slot avant une échéance5s, pas présentée comme panne applicative supplémentaire.
- `scripts/test-next-transition.cjs` : vrai DOM connecté Chromium/pont fictif, réseauHTTP(S) bloqué, **PASS**. Préparation une seule fois à8s, image chargée mais calque transparent avantEOF, fond flouté àEOF, disparition de l'ancienne affiche et repli sans image. Contraste texte du bouton≥4,5, clair/sombre×fenêtre/plein écran, couverture au redimensionnement. Affiche de fixture de marque, pas une capture de lecture réelle.
- `scripts/test-event-player-cutout.cjs` : **PASS**, Noël/Halloween×clair/sombre×fenêtre/plein écran ; pas de réintroduction du décor devant la surface native.
- `node --test web/test/player.test.js web/test/bridge.test.js` : **24/24 PASS**, fichiers web inchangés. Aucun portage ou déploiement du client web.
- `git diff bd4de81..8be19f4 --check` PASS. Aucun delta dans `crates`, `apps`, `web`, `Cargo.toml` ou `Cargo.lock`, API inchangée ; contrôle restreint des ajouts sans adresse privée ou bloc certificat/clé. Audit historique/publication non rejoué.
- [Revue indépendante APPROVED_SCOPED](../reviews/next-transition-8be19f4.md), reviewer32/32PASS. Les anciens échecs de la suite globale `player-qa` ne sont pas annoncés corrigés.

Commandes navigateur : `CHILL_PREVIEW_NODE_MODULES=/private/tmp/marvin-cockpit.NmYyTL/package-manager/node_modules node scripts/test-next-transition.cjs` et même préfixe pour le contrôle du décor. Aucun lecteur, nouvelle vidéo, profil foyer, fournisseur, Pi ou NAS utilisé.

## Remise

Bundle séparé livré : **`artifacts/macos/Chill Center-test-8be19f4.app`** du dépôt canonique, interne **0.5.1/build6**,81Mio,48dylibs arm64 autonomes. Exécutable SHA256 **d94fa7adcf95edb4e483ff88bd7d338b1b730b077029097dc84d1be87e819716**. Build release hors ligne/packaging, signature ad hoc profonde/stricte et fermeture des dépendances PASS ; `--verify-bundle` du fichier publié confirme libmpvAPI2.5, sans ouverture de fenêtre/lecture. Deux avertissements `dead_code` natifs existants conservés.

Commande : `CARGO_HOME=/private/tmp/video-catalog-cargo CARGO_TARGET_DIR=/private/tmp/chillcenter-series-target.ZXNxst CARGO_NET_OFFLINE=true bash scripts/package-macos.sh <nouvelle-destination-test-8be19f4.app>`. Aucune version figée ni ancien candidat remplacé ; bundle précédentdf2870f toujoursSHA20316cb6… et stable0.5.1 toujoursSHA815c2a0c… . Aucun tag/release stable incrémenté, aucun push GitHub ou changement de branche canonique active/worktree web.

Validation humaine de cette transition dans Tauri reste distincte des preuves DOM synthétiques ; pas de nouvelle validation visuelle réelle revendiquée. Le serveur14d84…/sourcedf2870f reste celui du déploiement précédent, non sondé ni redéployé pendant ce lot. Web installé, worktree web, NAS et configuration inchangés. Retour arrière client : précédent bundle, sans restaurer de base. Pas de migration ou rollback serveur nécessaire.
