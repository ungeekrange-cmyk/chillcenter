# Branding r4 — preuve de livraison locale

2026-10-13 · auteur /root · DEV_FAST, assets locaux seulement. Source figée : `a62218ad7026f178adb27d2d1a436546249d34f0`, branche `codex/branding-spacing-summer`, base `61f6cd68dae7b2562c4e6d535269f0e0fd8bf2c9`. Pas de push, déploiement serveur ou rebuild Mac.

## Demande et méthode

Noël clair : branches visibles de chaque côté du dossier. Espacements du canapé uniformes entre logos. Été plus chaud/lumineux. Après mesure des PNG et retouches ciblées, l'utilisateur autorise explicitement « Oui, créer le gabarit SVG commun ». Source unique vectorisée du défaut retenu, treize SVG autonomes produits par `scripts/branding.rs` ; couleurs et motifs seulement variables. Original approuvé intact, trois anciens PNG r2 conservés physiquement, historique Git et prompts conservés.

## Vérifications exécutées sur la source

- Rust1.99.0 existant : compilation autonome et3 tests PASS (`every_theme_has_identical_geometry`, `palette_and_motif_placement_are_explicit`, `assets_are_static_self_contained_and_font_independent`). Aucun ajout àCargo ou au lock.
- Génération :13 SVG, répétable depuis `vector/master.svg`. Définitions/transformations communes identiques, empreinte XML canonique `09f00d733c19968011268b64f81759bea354f37eae0a742ebdeb604cef0ee3ea`. Sans police externe, image liée, script, animation ou référence distante.
- XML13/13 : identifiants uniques, références locales résolues, viewBox1254×1254.
- Chromium headless avec profil temporaire et réseau bloqué :13 SVG rendus. Transparence alpha0 dans les coins et255 dans la tuile ; branches du sapin vérifiées aux deux côtés dans les deux modes ; tige de citrouille contrastée vérifiée dans les deux modes.
- Galerie réelle `file://`, même source figée :14 images chargées et décodées,0script, requêtes réseau bloquées ; capture entière inspectée. URL de test locale `/private/tmp/chillcenter-branding.OJILXG/repo/icons/branding/gallery.html`.
- Inspection des12 variantes depuis les captures navigateur et comparaison originale/vectorielle. Masque de premier plan vectorisé versus PNG : IoU99,63%, contours lissés/couleurs unies, pas identité pixel par pixel avec l'original.
- `git diff --check`, `shasum -a256 -c MANIFEST.sha256` : PASS. Défaut PNG SHA256 inchangé `45dd0eec618bec968f29c009e9122e74c42081db3bbc462c4bb3176d769b128f`. Aucun changement aux crates, frontend connecté, icône runtime ou .app.

Commandes source : `rustc --edition=2021 --test scripts/branding.rs -o <binaire-test-temporaire>`, exécution du binaire ; `rustc --edition=2021 scripts/branding.rs -o <générateur-temporaire>`, exécution depuis la racine. Environnement installé utilisé : `CARGO_HOME=/private/tmp/video-catalog-cargo`, `RUSTUP_HOME=/private/tmp/video-catalog-rustup`, rustc dans `/private/tmp/video-catalog-cargo/bin`. Rendu/galerie via Playwright fourni dans les dépendances desktop ; Chromium a nécessité une escalade ciblée après refus de port Mach du sandbox. Python3.12.14/Pillow12.3.0 fournis utilisés pour la vérification XML/pixels, aucune nouvelle dépendance runtime. La vérification ne réexécute pas les124 tests applicatifs, non affectés.

Rapports consultables : `icons/branding/vector/provenance.json`, `icons/branding/geometry-audit-r3.json` (PNG historiques), BUG-0010. Vérifications ci-dessus réalisées par l'auteur, pas une approbation indépendante.

Revue indépendante `/root/svg_review` : [BRANDING-SVG-REVIEW](../reviews/BRANDING-SVG-REVIEW.md), APPROVED pour le SHA source exact `a62218ad7026f178adb27d2d1a436546249d34f0`, aucun finding actionnable. Regénération identique, tests/format/warnings, sécurité statique des SVG et préservation du défaut revérifiés ; preuves Chromium de l'auteur réutilisées explicitement, pas un second parcours navigateur. Le commit porteur de cette preuve ne change pas les sources SVG ou le générateur revus.

## Limites et rollback

### Suite r5 — base validée et exports uniformisés

« nickel, utilise le vectorisé comme base pour refaire les autres » valide la base vectorielle. Base du lot d'exports : `f1452fd009d5c58ec34ee5d53d5ff8b66c03b76d`. Générateur Rust relancé, SVG/générateur conservés à l'identique ;13 PNG1254×1254 exportés dans Chromium avec profil isolé, réseau bloqué, aucune génération IA. Les12 PNG canoniques d'ambiance sont remplacés par ces exports, leurs versions antérieures étant copiées dans `raster-r4/` et tracées dans le manifeste. `default.png` original reste intact ; export courant du défaut sous `vector/default.png`, défaut courant SVG sous `vector/default.svg`. Hashes et détails dans `vector/exports-r5.json` ; statut du rapport r4 ci-dessus historique, la base est maintenant validée.

Rollback r5 : reprendre la branche/les fichiers du SHA de base ou copier les PNG de `raster-r4/` vers les chemins canoniques, dans une branche isolée puis régénérer le manifeste racine. Aucune suppression de donnée/média ou modification du runtime. Pas de revue de code supplémentaire revendiquée puisque SVG/générateur revus sont inchangés ; le nouveau lot est une opération d'exports et de métadonnées.

Méthode SVG approuvée ; rendu visuel encore PROPOSED. Pas d'audit quantitatif de contraste ou d'icône16px. Aucun raccordement du calendrier/des logos au runtime. Les anciens PNG conservent leurs petits écarts ; la garantie d'espacements identiques concerne la géométrie partagée des SVG.

Remise locale par fast-forward sur branche `codex/`, uniquement si le dossier cible est propre et reste au SHA de base attendu. Retour arrière : sélectionner la branche précédente `codex/branding-refinements` sur checkout propre, ou réappliquer les fichiers de base dans une branche isolée ; les snapshots et nouvelles sources restent récupérables dans Git. Aucun média ni donnée supprimé.
