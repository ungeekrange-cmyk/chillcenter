# BUG0141 — Reprendre sans pourcentage, compteurs ni coche

9 novembre 2026. Correction explicitement demandée après installation de
Mac0.7.7 et serveur/web0.7.6. Corrigé, publié et installé : Mac0.7.8/build18, serveur/web0.7.7,
source `70c8292adeea820155fe47fc92e8ba5a69e8ec70`. Voir [REL-0.7.8](../releases/REL-0.7.8.md).

## Causes et données vérifiées

Les anciennes reprises ont une position, mais aucune durée dans le registre des
observations de lecture. BUG0140 les rend visibles sans pourcentage ; son critère
position>0 inclut aussi les essais de quelques secondes. Les compteurs de saison
et les commandes vu/non vu ne sont pas exposés dans les cartes de l’Accueil.
Le tri ne prouve pas un visionnage récent : les anciennes reprises n’ont pas
d’horodatage de lecture fiable et aucune date n’est inventée.

Lecture de l’API avec la confiance TLS du Mac : dix reprises, toutes sans durée
mesurée ; la série signalée compte réellement4 épisodes vus sur150. Le film que
l’utilisateur indique terminé était toujours non vu. Seule cette correction
personnelle explicite a été appliquée au Pi : vu=true, position conservée et
absence de Reprendre vérifiée. Les autres historiques restent intacts.

Après validation SSH1Password, lecture des seuls en-têtes des dix fichiers :
942 octets au total, formats MKV/MP4/AVI. Le film principal a une durée réelle
de9 382 421 ms et une reprise à3 025 917 ms, soit32,25 %. Les films parasites sont
entre0,2 % et3 %. Le parseur Rust candidat reproduit les dix durées depuis ces
seuls octets, reconstruits dans une fixture creuse immédiatement retirée.
Aucune vidéo téléchargée ou lancée, aucun transcodage, aucun parcours du NAS.
Les identifiants, titres et en-têtes de ce diagnostic restent dans les preuves
privées de la tâche et ne sont pas ajoutés au dépôt.

## Correction

- Enrichissement Home avec la durée déclarée du fichier, sans réécrire la table
  des observations de lecture. Le serveur lit des en-têtes bornés, sous les mêmes
  protections de chemins et de versions que le streaming.
- Essais sous5 % écartés lorsqu’une durée réelle est connue ; positions gardées
  pour la fiche. Une durée indisponible conserve une indication explicite de
  position, sans pourcentage inventé. Aucun recours à une durée TMDB estimée.
- Compteur vus/total de la saison affichée, y compris saison inconnue ; pour la
  série, le pourcentage de la carte compte les épisodes terminés.
- Coche de la carte : film vu, ou uniquement saison affichée vue (choix explicite
  de l’utilisateur). Marquage groupé atomique, révision du catalogue obligatoire,
  position/notes conservées et autres saisons/profils indépendants.
- Après succès serveur, fondu180 ms puis glissement300 ms. Aucun retrait sur
  échec. Navigation/profil et réponses tardives protégés ; mouvements réduits
  respectés et cartes inchangées conservées pendant la synchronisation.

## Vérification et limites

Régressions domaine : seuil, lecture seule, compteurs, conservation des profils,
saisons/notes/positions et rollback complet en cas d’échec SQL intermédiaire.
Régressions HTTP : durée réelle sans bail/écriture, chemins masqués, révision
requise et route historique compatible. Contrats Mac/web et réponses tardives.
Chromium réel : bureau1280px, mobile390px, mouvements réduits, attente serveur,
échec, fondus et déplacement mesuré des cartes. Un test a révélé un décalage de
3px lié au scroll-snap/focus ; marge de défilement et focus sans scroll corrigés.
La compilation du client Mac est vérifiée séparément des tests Chromium.

Les observations compactes de lecture restent distinctes des durées de conteneur.
Format non reconnu/en-tête absent, NAS lent ou occupé : aucune durée supposée,
la position reste consultable. Les essais physiques Safari/iPhone ne sont pas
attestés par les fixtures Chromium. Voir [ADR0042](../architecture/decisions/ADR-0042-home-progress-controls.md).

Validation fonctionnelle initiale :302 tests Rust réussis (55 client,86 domaine,161 serveur ; deux
opt-in ignorés),377 tests JavaScript,27 documentaires, Clippy strict domaine/
serveur/client et compilation `cargo check -p chillcenter-desktop` PASS.
Parcours Home Chromium1280/390px et mouvements réduits PASS, ainsi que les
smokes existants web et mobile320/390/430/736/844/1280px. Captures bureau/mobile
assemblées relues : pourcentages, barres conservées après chargement des images,
compteur de saison et coche visibles. Les cartes conservées gardent leur identité
lorsqu’une vingt-et-unième carte remplit la place libérée.

Commandes : `cargo test --locked --offline -p chill-domain -p chill-server -p
chill-client --lib`, `cargo check --locked --offline -p chillcenter-desktop`,
`cargo clippy --locked --offline -p chill-domain -p chill-server -p chill-client
--lib -- -D warnings`, suites Node partagées/web, `scripts/test-home-resume-ui.cjs`,
`scripts/test-web-parity.cjs` et `scripts/test-web-mobile.cjs`. Le bind TCP du
sandbox a d’abord échoué EPERM ; la même suite avec réseau local autorisé est
PASS. Les durées réelles ont été vérifiées à partir d’en-têtes, pas d’une lecture
simulée ou d’une durée TMDB. Lors de cette validation initiale, les paquets publics et données NAS étaient inchangés.

### Revue de livraison — focus dans une rangée défilée

La revue indépendante a trouvé un retour du focus au début de la rangée après un retrait au milieu. Le bouton désactivé perd aussi le focus pendant la sauvegarde. Le client conserve l’intention de focus, choisit le voisin suivant (ou précédent pour la dernière carte), le garde visible et restaure le focus et le défilement lors du rafraîchissement de révision. Le test navigateur couvre les retraits au milieu et en fin de rangée sur écran de 390 px, avec une nouvelle révision après chaque sauvegarde.

### Vérification après installation

Les manifestes signés et les fichiers HTTP correspondent aux paquets publics. La lecture Home sur le Pi et la GUI Mac confirme les pourcentages et le compteur de saison ; positions, notes et états vu sont inchangés par la livraison. Les essais sous5 % sont exclus, le film déjà terminé reste vu et absent. Les commandes de coche et animations sont vérifiées sur des fixtures isolées ; aucun autre film ou saison réel n’est marqué pour un test.
