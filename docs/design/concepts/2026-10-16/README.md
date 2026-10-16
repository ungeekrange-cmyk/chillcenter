# CC-CONCEPT-20261016 · Filtres et ambiances événementielles

### Révision 5 — Noël moins rose et Halloween orangé (validation en attente)

Retour humain : Noël clair trop rose, sombre pas assez rouge Noël ; ajouter de l'orange à Halloween. Nouvelles barres gauches alignées dans les deux maquettes : Noël clair rouge franc `#ba3028`, textes ivoire `#fff8ed`/`#fff0df`, accent or `#ffe0a3` ; sombre rouge profond `#701019`, textes `#fff5e7`/`#efd3bb`, accent `#e8ca8b`. Halloween clair abricot `#f5d4a7`, textes `#432917`/`#6f492b`, accent `#8b3b0e` ; sombre brun-orangé `#352014`, textes `#fff1df`/`#dcc0a4`, accent `#ffae58`. Les photographies, logos, animations et filtres restent inchangés. Contrôle dédié `node docs/design/concepts/2026-10-16/check-palette.cjs` : rendu des quatre palettes, comparaison des deux maquettes et contraste des textes/icônes sur fond normal ; aucun test lecteur ni accès serveur. R4 ci-dessous conservée comme historique, remplacée seulement pour ces tokens. Toujours aucun changement applicatif ou déploiement.

### Révision 4 — panneau Noël rouge (validation visuelle en attente)

Demande humaine du16octobre : panneau gauche rouge clair en mode clair et rouge très sombre en mode sombre. Maquette complète et aperçu de barre gauche alignés : clair fond `#f2c7c4`, texte `#411b1e`, secondaire `#74484c`, accent `#83343c` ; sombre fond `#240a0f`, texte `#fff0e6`, secondaire `#d6b8b2`, accent `#e3c28c`. Neige, photographies, logos, Halloween et filtres inchangés. Modification de concept seulement ; aucune intégration ou livraison autorisée par ce retour. Contrôles de rendu calculé Noël et non-régression Halloween ajoutés au harnais : PASS, captures inspectées en clair/sombre. Contrastes texte/secondaire/accent sur fond normal : clair9,81/4,95/5,45:1 ; sombre16,76/10,09/10,98:1 (sRGB, pas une certification de tous les états). Les sections r3 ci-dessous conservent le contexte.

Statut : **PENDING_USER_APPROVAL** pour l'ensemble, révision 3, 16 octobre 2026. Maquettes uniquement : aucune intégration Tauri/web, API, base, NAS, serveur ou livraison modifiée. Les documents et approbations précédents restent historiques et intacts. Après la révision dessinée, l'utilisateur préfère finalement le premier rendu photographique, avec citrouille/sapin beaucoup plus proches et gros, sapin moins chargé. Ces deux nouveaux décors reçoivent « nickel », puis demande de voir la barre gauche et les filtres ; cet accord sur les images n'autorise pas encore l'intégration de l'ensemble. Originaux photographiques et dessinés préservés.

## Filtres interactifs

`filters.fragment.html` est le fragment interactif autonome ; `filters-preview.html` son enveloppe de consultation. Catalogue fictif de quatorze films, aucun accès réseau.

- Genres : panneau sous le bouton, sélection multiple, résultats immédiatement recalculés à chaque clic. Proposition à valider : **OU** entre genres ; **ET** avec la période. Un deuxième clic retire un genre.
- Années présentes dans le catalogue, plus récente en premier. Une borne = année exacte ; deux bornes = intervalle inclusif, dans n'importe quel ordre de clic. Les années intermédiaires sont surlignées.
- Proposition ergonomique à valider : une troisième année recommence un intervalle ; cliquer une borne la retire. Réinitialiser efface les filtres. Aucun bouton Appliquer ni requête par clic.
- L'intégration future doit filtrer le miroir local et préserver le défilement/les fiches ; la maquette ne prouve pas cette intégration.

## Ambiances

`events.html` : Noël/Halloween, clair/sombre, largeur large/compacte/étroite, animation activable. Les images photographiques courantes sont `assets/christmas-foreground.png` et `assets/halloween-foreground.png` ; les propositions dessinées antérieures sont conservées séparément. Le logo réutilise les SVG événementiels approuvés. `sidebar.fragment.html` et son enveloppe `sidebar-preview.html` montrent les deux barres gauches côte à côte, clair/sombre, animation désactivable, logos embarqués sans réseau ; il s'agit toujours de navigation illustrative.

- Noël : salon, cadeaux et sapin au premier plan à droite ; neige discrète dans la barre latérale.
- Halloween : salon décoré et grosses citrouilles aux visages éclairés à droite ; cinq chauves-souris s'envolent lentement dans la barre latérale.
- Image limitée au panneau droit, `object-fit: cover; object-position: 100% 50%`. Le cadrage conserve le bord droit au redimensionnement, sans miroir ni déplacement du décor à gauche ; un recadrage partiel est normal en fenêtre très étroite.
- Rendu photographique du salon approuvé, gros sujets au premier plan droit, palette chaude bordeaux/ocre. Voile gradué opaque derrière les textes, plus transparent à droite. Décor non interactif, ignoré par les technologies d'assistance. Catalogue et navigation restent au premier plan.
- Ressources : une seule image visible, quatorze particules de neige ou cinq chauves-souris ; animations CSS transform/opacity, aucun canvas, filtre flou animé ou appel serveur. Arrêt si onglet masqué, désactivation manuelle, respect de `prefers-reduced-motion`. Budget réel RAM/GPU/Tauri **non mesuré** dans ce concept.

## Système visuel et accessibilité

Continuité du système approuvé : police système, surfaces crème/vert sombre, accent ocre et bordeaux, logo seul ; aucune nouvelle police ou bibliothèque. Noël sombre : fond `#151b18`, texte `#f0eee4`, accent `#d9c48b`. Clair : fond `#faf5eb`, texte `#302c25`, accent `#795428`. Halloween sombre : fond `#1e181c`, accent `#e1a35b`; clair `#fbf0e3`, accent `#955218`. Rythme 8/12/16/24/32px, rayons 8–16px. Contrastes des compositions photographiques non certifiés ; voile à conserver lors de toute intégration.

Contrôles natifs, état sélectionné annoncé, focus visible, surfaces adaptatives. Les contrôles de largeur et d'ambiance sont des outils de démonstration, pas une décision de les placer dans le catalogue final. La recherche et les liens de navigation de la maquette des ambiances sont illustratifs.

## Validation et suite

Contrôles locaux du 16 octobre : `node docs/design/concepts/2026-10-16/check.cjs` **PASS**, Chrome isolé sans réseau. Quatre rendus événement/mode, ancrage à droite, largeur réelle étroite, absence de débordement 320–1440px, arrêt manuel/reduced-motion/onglet masqué ; genres multiples, années décroissantes, bornes inclusives, troisième année, Escape et panneaux mobiles. Captures inspectées directement par l'auteur, pas une revue indépendante. Les captures et l'enveloppe de prévisualisation sont générées localement et exclues de Git ; images de concept et fragment source suivis.

Une première inspection a révélé un défaut de portée CSS dans le fragment de filtres (`@scope` avec racine répétée) ; corrigé avec `:scope`, puis vérification du style réellement calculé et nouvelle capture concluantes. Le contrôle du format étroit attend maintenant la largeur réelle après la transition, pas seulement une classe. Ces observations concernent la maquette seule, aucun nouveau bug produit revendiqué.

Origines des décors dessinés r2 : génération `exec-75f31960-aa53-4f19-9a64-652b44a7edf4.png` (Noël) et `exec-9406616f-8520-4dc1-bb64-67b43baf8125.png` (Halloween), dans le répertoire de génération du chat. Brief : illustration éditoriale simple façon gouache, formes arrondies, aplats crème/bordeaux/ocre. Les originaux ne sont ni déplacés ni supprimés.

Décors photographiques rapprochés r3 : `exec-7c94a3ea-bb6b-4257-a64c-4f94b31fa64e.png` (Noël) et `exec-9a9cb539-37e8-43ae-998e-ceae7ae225c3.png` (Halloween), générateur intégré, retouches des premiers décors : sujet festif proche et agrandi à droite, sapin peu décoré, salon/couleurs conservés, sans texte ni personnes. Nouveaux tests PASS sur ces assets ; aperçu sidebar : logos chargés, mouvement CSS réel mesuré, clair/sombre, arrêt manuel/réduction des animations et largeur320px vérifiés ; captures inspectées. Les contrôles attendent le peuplement de l'iframe après chargement et rouvrent une page par largeur pour éviter les courses de redimensionnement du harnais. Aucune revendication de performance native ou d'intégration réelle.

L'utilisateur a demandé de voir avant intégration. Approbation reçue pour la direction des deux images rapprochées uniquement ; validation des animations/filtres en attente, aucun déploiement autorisé par ce lot. Après retour humain seulement : intégrer une tranche bornée aux clients concernés en préservant le travail web parallèle et les contrats API existants, puis tests ciblés sur ces changements. Aucun nouveau test lecteur nécessaire pour cette maquette.
