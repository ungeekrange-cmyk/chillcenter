# Logos Chill Center

2026-10-13. Génération et retouches avec l'outil intégré image_gen, sans CLI/API à configurer.

## Logo par défaut approuvé

<img src="default.png" width="240" alt="Logo Chill Center par défaut, bordeaux et ocre">

Choix utilisateur : « oui on garde celui-ci par défaut », corrigé par « le premier des deux ». Il s'agit de **la première proposition de la dernière paire**, avant le second décalage vers le bas : source `exec-df820cb4-e6c6-4ce6-8266-d301ab7fe79d.png`. Pas de `exec-29e960ca…`. Copie originale conservée sans modification dans `default.png`, SHA-256 `45dd0eec618bec968f29c009e9122e74c42081db3bbc462c4bb3176d769b128f`.

## Douze déclinaisons proposées

| Ambiance | Claire | Sombre |
| --- | --- | --- |
| Printemps | <img src="spring-light.png" width="180" alt="Printemps clair"> | <img src="spring-dark.png" width="180" alt="Printemps sombre"> |
| Été | <img src="summer-light.png" width="180" alt="Été clair"> | <img src="summer-dark.png" width="180" alt="Été sombre"> |
| Automne | <img src="autumn-light.png" width="180" alt="Automne clair"> | <img src="autumn-dark.png" width="180" alt="Automne sombre"> |
| Hiver | <img src="winter-light.png" width="180" alt="Hiver clair"> | <img src="winter-dark.png" width="180" alt="Hiver sombre"> |
| Noël | <img src="christmas-light.png" width="180" alt="Noël clair"> | <img src="christmas-dark.png" width="180" alt="Noël sombre"> |
| Halloween | <img src="halloween-light.png" width="180" alt="Halloween clair"> | <img src="halloween-dark.png" width="180" alt="Halloween sombre"> |

Palette : sauge/vert pour printemps, sable/ocre pour été, cuivre/bordeaux pour automne, bleu givré/bleu nuit pour hiver. Noël : crème/rouge ou sapin/champagne avec houx ; Halloween : crème/citrouille ou aubergine/citrouille avec chauve-souris. Motifs uniquement pour les événements. Tous gardent le canapé et le lettrage gourmand en arc.

[Galerie de comparaison](gallery.html) — page locale sans JavaScript, animation, réseau ou service externe.

## Fichiers et provenance

Treize PNG statiques (logo retenu +12 propositions),1254×1254, canal alpha, environ14MiB au total. `manifest.json` référence tous les fichiers, hashes, palettes **demandées** (pas des mesures des pixels) et prompts complets. Les deux premiers rendus Noël non retenus restent seulement dans l'historique de génération, identifiés dans le manifeste ; les fichiers livrés sont les versions corrigées. Les sources générées ne sont pas supprimées.

Vérification : identité binaire du défaut avec le choix utilisateur ; lecture PNG, dimensions/alpha, hashes et présence des six paires. Inspection visuelle : texte, formes, équilibre clair/sombre, correction de l'outline Noël et marques de fond. Pas de mesure quantitative de contraste, audit d'icône à16px ou vectorisation finale ; ce sont des propositions raster de branding.

## Intégration et limites

Le logo retenu est la référence de marque par défaut ; les déclinaisons attendent validation visuelle. **Ce lot livre les assets, pas le sélecteur d'ambiances de l'application.** L'ancien `icons/icon.png`, le frontend et le paquet .app ne sont pas remplacés ici. Pas de build/déploiement ou push GitHub. Charger seulement la variante active, prévoir les exports d'icône optimisés au raccordement ; ne pas précharger13 images dans la webview.

Règles déjà approuvées, inchangées : saisons Nord aux1mars/juin/septembre/décembre ; Halloween1–31octobre ; Noël25novembre–25décembre ; événements prioritaires, retour saison le lendemain, modes clair/sombre/système indépendants du choix auto/manuel/désactivé. Voir F-20/F-21 et US-032/033. Aucune animation ni règle de calendrier implémentée par une image.

Changement local réversible d'assets, pas de nouvel ADR ou nouvelle dépendance runtime. Les approvals code/test/sécurité du lot interface restent liés à leur SHA ; elles n'approuvent pas ces nouvelles propositions visuelles.
