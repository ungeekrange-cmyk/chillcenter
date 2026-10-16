# CC-CONCEPT-20261016 · Filtres et ambiances événementielles

Statut : **PENDING_USER_APPROVAL**, révision 2, 16 octobre 2026. Maquettes uniquement : aucune intégration Tauri/web, API, base, NAS, serveur ou livraison modifiée. Les documents et approbations précédents restent historiques et intacts. Retour utilisateur intégré : « un style un peu plus simple, un peu plus dessiné » ; les décors photographiques initiaux ont été remplacés par des illustrations aux aplats doux, sans modifier leur composition. Originaux photographiques et dessinés conservés dans le dossier de génération, hors livraison applicative.

## Filtres interactifs

`filters.fragment.html` est le fragment interactif autonome ; `filters-preview.html` son enveloppe de consultation. Catalogue fictif de quatorze films, aucun accès réseau.

- Genres : panneau sous le bouton, sélection multiple, résultats immédiatement recalculés à chaque clic. Proposition à valider : **OU** entre genres ; **ET** avec la période. Un deuxième clic retire un genre.
- Années présentes dans le catalogue, plus récente en premier. Une borne = année exacte ; deux bornes = intervalle inclusif, dans n'importe quel ordre de clic. Les années intermédiaires sont surlignées.
- Proposition ergonomique à valider : une troisième année recommence un intervalle ; cliquer une borne la retire. Réinitialiser efface les filtres. Aucun bouton Appliquer ni requête par clic.
- L'intégration future doit filtrer le miroir local et préserver le défilement/les fiches ; la maquette ne prouve pas cette intégration.

## Ambiances

`events.html` : Noël/Halloween, clair/sombre, largeur large/compacte/étroite, animation activable. Deux images originales générées, conservées sous `assets/`; originaux de génération préservés. Le logo réutilise les SVG événementiels approuvés.

- Noël : salon, cadeaux et sapin au premier plan à droite ; neige discrète dans la barre latérale.
- Halloween : salon décoré et grosses citrouilles aux visages éclairés à droite ; cinq chauves-souris s'envolent lentement dans la barre latérale.
- Image limitée au panneau droit, `object-fit: cover; object-position: 100% 50%`. Le cadrage conserve le bord droit au redimensionnement, sans miroir ni déplacement du décor à gauche ; un recadrage partiel est normal en fenêtre très étroite.
- Illustrations de salon dessinées, formes simples et arrondies, palette crème/bordeaux/ocre. Voile gradué opaque derrière les textes, plus transparent à droite. Décor non interactif, ignoré par les technologies d'assistance. Catalogue et navigation restent au premier plan.
- Ressources : une seule image visible, quatorze particules de neige ou cinq chauves-souris ; animations CSS transform/opacity, aucun canvas, filtre flou animé ou appel serveur. Arrêt si onglet masqué, désactivation manuelle, respect de `prefers-reduced-motion`. Budget réel RAM/GPU/Tauri **non mesuré** dans ce concept.

## Système visuel et accessibilité

Continuité du système approuvé : police système, surfaces crème/vert sombre, accent ocre et bordeaux, logo seul ; aucune nouvelle police ou bibliothèque. Noël sombre : fond `#151b18`, texte `#f0eee4`, accent `#d9c48b`. Clair : fond `#faf5eb`, texte `#302c25`, accent `#795428`. Halloween sombre : fond `#1e181c`, accent `#e1a35b`; clair `#fbf0e3`, accent `#955218`. Rythme 8/12/16/24/32px, rayons 8–16px. Contrastes des compositions photographiques non certifiés ; voile à conserver lors de toute intégration.

Contrôles natifs, état sélectionné annoncé, focus visible, surfaces adaptatives. Les contrôles de largeur et d'ambiance sont des outils de démonstration, pas une décision de les placer dans le catalogue final. La recherche et les liens de navigation de la maquette des ambiances sont illustratifs.

## Validation et suite

Contrôles locaux du 16 octobre : `node docs/design/concepts/2026-10-16/check.cjs` **PASS**, Chrome isolé sans réseau. Quatre rendus événement/mode, ancrage à droite, largeur réelle étroite, absence de débordement 320–1440px, arrêt manuel/reduced-motion/onglet masqué ; genres multiples, années décroissantes, bornes inclusives, troisième année, Escape et panneaux mobiles. Captures inspectées directement par l'auteur, pas une revue indépendante. Les captures et l'enveloppe de prévisualisation sont générées localement et exclues de Git ; images de concept et fragment source suivis.

Une première inspection a révélé un défaut de portée CSS dans le fragment de filtres (`@scope` avec racine répétée) ; corrigé avec `:scope`, puis vérification du style réellement calculé et nouvelle capture concluantes. Le contrôle du format étroit attend maintenant la largeur réelle après la transition, pas seulement une classe. Ces observations concernent la maquette seule, aucun nouveau bug produit revendiqué.

Origines des décors finaux : génération `exec-75f31960-aa53-4f19-9a64-652b44a7edf4.png` (Noël) et `exec-9406616f-8520-4dc1-bb64-67b43baf8125.png` (Halloween), dans le répertoire de génération du chat. Brief : illustration éditoriale simple façon gouache, formes arrondies, aplats crème/bordeaux/ocre, salon calme à gauche, sujets festifs au premier plan à droite, ni texte ni personnes. Les originaux ne sont ni déplacés ni supprimés.

L'utilisateur a demandé de voir avant intégration. Approbation non reçue ; aucun déploiement autorisé par ce lot. Après retour humain seulement : intégrer une tranche bornée aux clients concernés en préservant le travail web parallèle et les contrats API existants, puis tests ciblés sur ces changements. Aucun nouveau test lecteur nécessaire pour cette maquette.
