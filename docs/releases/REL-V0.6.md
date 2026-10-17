# Chill Center v0.6

Date : 17 octobre 2026. Autorité : validation utilisateur explicite du candidat,
puis demande de commit et de fusion avec documentation. Version Mac **0.6.0**,
build **7**, tag local **v0.6**.

## Périmètre et provenance

Cette version consolide les corrections validées depuis v0.5.1. Le gel ne change
que les métadonnées du client Mac et la documentation ; il ne refait ni le lecteur
ni les contrats réseau. Base exacte : `73cbc8c9fec3fbddf3f239d00db64ca450a7d9f6`.
Dernier changement de tests : `095631ad55d1502480c874016a24f3b79968ff28`.
Dernier changement fonctionnel Mac : `15c2d63c2bca5058456a16c8ac7d0320d440ad4c`.

Branche de livraison : `codex/release-v0.6`. Fusion locale prévue dans la branche
d'intégration existante `codex/player-streaming`, sans réécrire l'historique.
Le tag est créé après consignation des preuves. La source de compilation et le
SHA256 du binaire sont indiqués dans la section Livraison ci-dessous ; un commit
ultérieur limité à la documentation ne signifie pas une autre source compilée.

## Changements inclus

- Sous-titres externes récupérés sans bloquer la lecture ; sélection précédente
  annulée si elle devient obsolète. Pas de remise à zéro du moteur ou du tampon.
- Retour du lecteur et fermeture de fiche : invalidation des mesures cachées et
  reconstruction de la grille de vignettes, plutôt que des bandes étirées.
- Langue locale `auto`, `en` ou `fr` dans Configuration. Automatique : français
  si la langue système principale est française ; anglais sinon. Les titres,
  métadonnées et données personnelles ne sont pas traduits artificiellement.
- Deux tests QA obsolètes corrigés à leur couche responsable, avec contre-tests
  et contrôle de disponibilité de l'interface sans lecture réelle supplémentaire.
- Correctifs déjà livrés : feedback d'association TMDB, nettoyage des suffixes
  de fichiers, distinction des séries homonymes et regroupement typographique,
  bibliothèques multiples, fichier absent au lancement, tampon conservé en pause,
  recommandations horizontales et transition vers la vidéo suivante.
- Optimisations RAM et déduplication d'affiches déjà livrées, sans réduction du
  tampon ni changement du contrat de cache.

Preuves détaillées, sans prétendre à une nouvelle validation globale :
[langues, sous-titres et grille](../implementation/PLAYBACK-LANGUAGE-20261016.md),
[QA](../implementation/QA-CONTRACTS-20261017.md),
[Configuration et matching](../implementation/CONFIGURATION-MATCHING-20261016.md),
[recommandations](../implementation/RECOMMENDATION-RAILS-20261016.md),
[transition](../implementation/NEXT-TRANSITION-20261016.md),
[mesures de ressources](../performance/RESOURCE-OPTIMIZATION-20261016.md).

## Compatibilité et éléments non modifiés

- API datée `/api/2026-10-14` et alias historique `/api/v1` conservés.
- Aucun nouveau déploiement, accès SSH, changement NAS ou configuration du Pi.
  Dernière livraison serveur vérifiée dans l'historique : source `f980355`,
  [rapport exact](REL-RESOURCE-OPTIMIZATION-001.md). Ce gel ne constitue pas une
  nouvelle vérification en ligne de sa version.
- Aucun changement des fichiers, de la branche ou du déploiement du client web.
  Sa version en chantier reste indépendante ; aucune fusion de `codex/web-pwa`.
- Aucun changement de schéma, dépendance, TLS, authentification ou préférences.
  Les versions serveur/crates et web ne sont pas alignées artificiellement sur
  le numéro de version du client Mac.
- Pas de push, publication GitHub, changement de visibilité ou déploiement Pages.

## Vérification du gel

Contrôles à consigner après exécution : cohérence Cargo/Tauri/plist/lock,
suite Node complète, compilation release verrouillée, signature profonde,
bibliothèques ARM64 et chargement du libmpv embarqué. Aucune nouvelle lecture
générale ou sollicitation des fournisseurs/NAS n'est nécessaire pour ce delta.

Les preuves antérieures restent attribuées à leurs sources : 7/7 tests QA et
130/130 tests Node sur `095631a`, contrôles Rust et DOM ciblés dans les rapports
liés. Elles ne deviennent pas de nouveaux essais physiques par changement du
numéro de version. Pas de nouvelle revue indépendante revendiquée pour ce gel.

## Livraison et retour arrière

Bundle distinct : `artifacts/macos/Chill Center-v0.6.app`. Compilation et
publication exclusive à confirmer avant remise finale. Application Apple Silicon,
macOS 13 minimum ; signature ad hoc, non notariée. Les licences des bibliothèques
embarquées sont incluses. Quitter l'ancienne application avant d'ouvrir ce bundle.

Les anciens bundles et tags sont conservés. Repli : quitter v0.6 et relancer
`Chill Center-test-095631a.app` ou une autre version livrée compatible ; aucune
restauration serveur/base ni intervention NAS n'est nécessaire pour ce gel.

## Limites connues

- BUG0082 (conflits non actionnables), BUG0083 (loupe trop petite) et BUG0084
  (rotation de synchronisation) restent ouverts, pas masqués par cette release.
- Les validations visuelles et physiques Tauri/NAS explicitement restantes dans
  les rapports, notamment changement de sous-titres en lecture et retour à la
  grille sur la collection réelle, ne sont pas déclarées rejouées ici.
- La capacité de plusieurs flux Full HD dépend toujours du NAS et du Wi-Fi :
  les mesures synthétiques RAM ne prouvent pas une nouvelle garantie de débit.
- La préparation d'un dépôt public ne vaut pas audit ni autorisation de publier
  tout l'historique. Les contrôles de ce gel portent sur ses ajouts, sans effacer
  les preuves privées ou réécrire les anciens commits.
