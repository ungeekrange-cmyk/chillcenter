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

Palette : sauge/vert pour printemps, sable/ocre pour été, cuivre/bordeaux pour automne, bleu givré/bleu nuit pour hiver. Noël : crème/rouge ou sapin/champagne, avec un sapin sauge derrière le canapé et le lettrage ; Halloween : crème/citrouille ou aubergine/citrouille, avec une citrouille posée sur l'assise. Motifs uniquement pour les événements. Tous gardent le canapé et le lettrage gourmand en arc.

### Retouches r2 demandées par l'utilisateur

Été clair : trace fantôme au centre du disque supprimée. Automne sombre : marque fantôme dans le fond extérieur supprimée. Les défauts étaient visibles directement dans les PNG, indépendamment du navigateur : [BUG-0009](../../docs/debug/BUG-0009-branding-artifacts.md).

Noël clair/sombre : houx remplacé par un sapin qui dépasse du dossier et passe derrière le « C » de Chill Center. Premier essai clair écarté car le sapin s'arrêtait sous les lettres ; une retouche de hauteur conserve la palette claire. Halloween clair/sombre : chauve-souris retirée, citrouille avec tige et nervures posée sur l'assise, devant le dossier. Les six autres déclinaisons et `default.png` restent identiques octet par octet.

Les six fichiers conservent leurs noms ; la galerie utilise `?v=2` sur leurs références pour éviter le cache de la version précédente après rechargement. Anciennes versions et prompts conservés dans la révision `branding-r2` du manifeste et le commit Git `135d78c029cde269434186e4a0b8432962739665`. Les propositions r2 restent à valider visuellement par l'utilisateur.

[Galerie de comparaison](gallery.html) — page locale sans JavaScript, animation, réseau ou service externe.

## Fichiers et provenance

Treize PNG statiques (logo retenu +12 propositions),1254×1254, canal alpha. `manifest.json` référence tous les fichiers, hashes, palettes **demandées** (pas des mesures des pixels), prompts complets et historique des retouches. Les rendus non retenus restent dans l'historique de génération, identifiés dans le manifeste ; les fichiers livrés sont les versions corrigées. Les sources générées ne sont pas supprimées.

Vérification : identité binaire du défaut avec le choix utilisateur ; lecture PNG, dimensions/alpha, hashes et présence des six paires. Inspection visuelle : texte, formes, équilibre clair/sombre, correction de l'outline Noël et marques de fond. Pas de mesure quantitative de contraste, audit d'icône à16px ou vectorisation finale ; ce sont des propositions raster de branding.

## Intégration et limites

Le logo retenu est la référence de marque par défaut ; les déclinaisons attendent validation visuelle. **Ce lot livre les assets, pas le sélecteur d'ambiances de l'application.** L'ancien `icons/icon.png`, le frontend et le paquet .app ne sont pas remplacés ici. Pas de build/déploiement ou push GitHub. Charger seulement la variante active, prévoir les exports d'icône optimisés au raccordement ; ne pas précharger13 images dans la webview.

Règles déjà approuvées, inchangées : saisons Nord aux1mars/juin/septembre/décembre ; Halloween1–31octobre ; Noël25novembre–25décembre ; événements prioritaires, retour saison le lendemain, modes clair/sombre/système indépendants du choix auto/manuel/désactivé. Voir F-20/F-21 et US-032/033. Aucune animation ni règle de calendrier implémentée par une image.

Changement local réversible d'assets, pas de nouvel ADR ou nouvelle dépendance runtime. Les approvals code/test/sécurité du lot interface restent liés à leur SHA ; elles n'approuvent pas ces nouvelles propositions visuelles.
