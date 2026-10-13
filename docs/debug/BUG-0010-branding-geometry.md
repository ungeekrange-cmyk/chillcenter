# BUG-0010 — Espacements du canapé variables selon le logo

2026-10-13 · VERIFIED pour les variantes SVG r4 ; écarts des PNG historiques conservés. Propriétaire /root. Assets seulement, pas un bug runtime.

## Reproduction et attendu/réel

Base `61f6cd68dae7b2562c4e6d535269f0e0fd8bf2c9`, correction `codex/branding-spacing-summer`. macOS, PNG1254×1254 dans galerie locale. Ouvrir chaque PNG directement et comparer avec `default.png` approuvé ; aucun NAS/serveur impliqué.

Attendu : mêmes formes et espaces entre dossier, assise et accoudoirs. Réel : Noël clair possède des séparations élargies (~25/26px ày790 contre18/18 pour le défaut) et un dossier abaissé (y671 contrey648). Son sapin ne déborde pas sur les côtés du dossier. Les autres variantes ont des différences plus petites.

## Hypothèse et mesure

H1 : la géométrie des PNG varie réellement, indépendamment de la galerie. Vérification : inspection directe des13 images, puis classification RGB en lecture seule avec Python3/Pillow11.3.0. Mesurer séparations ày770/790/820, assise àx470/780 et haut central du dossier. Comparer aux mêmes coupes du défaut. Résultat : confirmé ; écart maximal échantillonné de33px pour Noël clair. Origine interne de la génération non instrumentée ; la variation des fichiers est démontrée, pas son mécanisme interne.

Script de diagnostic local : `/private/tmp/chillcenter-branding.OJILXG/spacing-audit.py`, exécution `python3 /private/tmp/chillcenter-branding.OJILXG/spacing-audit.py icons/branding`. Aucune écriture d'image par ce script. Paramètres, méthode, hashes et données complètes avant/après conservés dans `icons/branding/geometry-audit-r3.json`, consultable sans ce répertoire temporaire. L'antialiasing et les gradients peuvent déplacer une frontière échantillonnée d'environ1px. Cette mesure ne compare pas toute la silhouette.

## Retouches et résultat

Imagegen intégré : Noël clair dérivé du sombre comme maître de géométrie, ancien clair comme référence de couleurs uniquement. Branches visibles à gauche et à droite, canapé et texte au premier plan. Haut du dossier y647 après retouche ; séparations20/21px ày790. Écart maximal échantillonné ramené de33 à4px. Été clair/sombre recolorés selon demande additionnelle de chaleur et luminosité, avec référence géométrique au défaut.

Les13 images ont été vérifiées ; rester précis : différences résiduelles1–4px sur1254px, environ1px au plus à320px. **Ne pas annoncer une uniformité exacte.** Source SVG commune proposée pour garantir une géométrie partagée, décision utilisateur en attente. Pas de re-génération des autres neuf variantes ni du défaut sans nécessité démontrée.

Provenance exacte des trois sorties/prompts dans le manifeste `branding-r3`. Le défaut approuvé garde SHA256 `45dd0eec618bec968f29c009e9122e74c42081db3bbc462c4bb3176d769b128f`. Aucun changement au code applicatif, à son icône runtime, au paquet Mac ou aux médias.

## Rollback et exigences

### Résolution définitive entre variantes SVG r4

Après preuve des limites du raster, l'utilisateur autorise « Oui, créer le gabarit SVG commun ». Les contours du défaut retenu sont vectorisés une fois puis partagés dans `vector/master.svg`. Générateur Rust sans dépendance runtime : les définitions et transformations sont identiques dans tous les SVG, palettes et présence de motifs seulement variables. L'égalité des tracés ne dépend plus d'une estimation de pixels colorés.

Preuves :3 tests Rust PASS ;13 fichiers XML avec IDs/références locales vérifiés ; empreinte commune des définitions `09f00d733c19968011268b64f81759bea354f37eae0a742ebdeb604cef0ee3ea` ;13 rendus navigateur sans réseau ; transparence, sapin visible des deux côtés et tige de citrouille contrastée vérifiés. Détails et limites dans `icons/branding/vector/provenance.json`. Le rendu interne ImageMagick ignorait des détails de chemins référencés de la citrouille ; rendu Chromium utilisé pour la vérification finale. Ce n'est pas un défaut applicatif. Le lancement Chromium sous sandbox a refusé un port Mach ; escalade ciblée réussie, profil temporaire uniquement.

Le masque de premier plan vectorisé recouvre le masque du PNG initial avec une IoU99,63% : contours lissés et couleurs unies, pas une copie binaire du logo. La méthode est autorisée ; le rendu visuel SVG reste proposé à l'utilisateur. `default.png` demeure inchangé, les trois anciens PNG r2 sont aussi conservés dans `raster-r2/`. Les écarts des PNG raster historiques ne sont pas présentés comme supprimés ; la galerie utilise désormais les SVG communs.

Retour arrière r4 : remettre la galerie raster depuis la base et laisser les nouvelles sources conservées ; aucun changement de base de données ou média. Aucun raccordement automatique au runtime dans cette intervention.

Restaurer les trois PNG, la galerie, le README et le manifeste de marque depuis le commit de base, dans une branche isolée ; régénérer `MANIFEST.sha256`. Garder ce journal historique. F-20/F-21 et US-032/033 inchangés. Aucun nouvel ADR architectural, coût ou dépendance runtime.
