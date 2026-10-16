# Recommandations par vignettes — fiches et conseil

16 octobre2026. Demande explicite puis précision « en fait les deux » : appliquer aux vidéos en relation et à la demande de conseil. Branche `codex/recommendation-rails`, base8e4523a ; [ADR0020](../architecture/decisions/ADR-0020-local-title-recommendations.md), compléments des US007/030. Aucun changement de découverte ou de fournisseur.

## Résultat implémenté

- Rail horizontal commun, flèches gauche/droite, trackpad, focus clavier, mouvement réduit ; toutes les vignettes de la sélection accessibles, pas de grille verticale ni de coupe aux douze premiers éléments. Titres/types/années sous les affiches, raisons réellement communes lorsqu'elles existent. Affiches via le cache/observer existant, retirées à la fermeture/remplacement ; gardes de contexte contre anciennes fiches/profils.
- Rust : regroupement par TMDB ou groupe source, homonymes distincts, aucun épisode individuel ; un UUID sert d'affiche mais la carte ouvre la série entière. Série sans nom exploitable exclue plutôt que transformée en fausse série ; fichiers et métadonnées d'épisodes jamais réécrits.
- Fiche : genres, réalisateurs et acteurs communs distincts, trim/casse normalisés ; classement par nombre de correspondances puis titre/UUID stable, au plus60 résultats disponibles, source et sa série exclus. Ni année ni saga seules ne suffisent. Métadonnées communes de série prioritaires, champs manquants non inventés.
- Conseil : instantané local regroupé et filtré au clic explicite ; filtre vu appliqué à la série entière, profil actif seulement. Titres sans métadonnées pertinentes écartés, plafond3000 titres regroupés/1Mio de prompt,60 propositions maximum ; ordre du modèle conservé. Les acteurs/réalisateurs connus sont réellement inclus dans la whitelist du prompt sous alias temporaires, pas seulement mentionnés dans l'interface. Le modèle doit expliquer les correspondances ; les identifiants/résultats restent validés et toute application est refusée en recherche.
- Aucune route/version HTTP ou DTO AiContext serveur changé. Organisation existante conserve son contexte de révisions ; nouveau booléen IPC facultatif `recommendations` par défautfalse. Pi, configuration, NAS, assets web installé et worktree web non touchés ; pas de build web.

## Preuves locales

- Domaine **70/70**, dont deux tests recommandations : genres/réalisation/distribution, doublons/casse, classement, absence de suggestion par année, séries homonymes, filtres personnels/état vu complet, métadonnées communes. Régressions complémentaires ciblées après ajout : série sans nom rejetée, clé de série publique opaque, exclusion de sa propre série non identifiée, aucune réécriture.
- Client **40/40** : vrai miroir SQLite, serveur volontairement injoignable, navigation/détail/série/recommandations/conseil disponibles sansHTTP. Premier passage39/40 : ancienne fixture attendait une suggestion sur la seule saga et clonait encore saison/épisode sur ses films. Fixture corrigée en vrais films ayant un acteur commun ; assertion de fonctionnement entièrement local conservée, cas sans vrai match testé au domaine. Pas un défaut caché par un repli réseau.
- Pont desktop IA **8/8**, dont aperçu sélectionné/regroupé, aucun chemin privé ou épisode dans sa réponse, aucune application possible ; annulation/délai/profil conservés. Catalogue filtreassistant **6/6**, whitelist enrichie de cast/directors sans chemins/ID sources et alias restaurés localement.
- Node **44/44** ciblés rails/IA/fiches/commandes et web bridge/player inchangés ; contrat API **76 chemins PASS**, diffcheckPASS. Anciens échecs globaux `player-qa` non requalifiés, aucun nouveau parcours vidéo.
- `scripts/test-recommendation-rails.cjs` : **PASS** sur le vrai DOM connecté/pont fictif : deux surfaces,20 cartes par rail, ligne unique, dernier résultat accessible au défilement, épisodes et doublons exclus, homonymes distincts, classement IA préservé et ouverture de la série choisie. Navigation hors conseil : observers/file retirés, réponse d'affiche tardive rejetée, sélection conservée puis affiche manquante reprise au retour sans nouvel appel fournisseur. RéseauHTTP(S) bloqué, aucun fournisseur/serveur/vidéo réel.
- Clippy bibliothèques domaine/client `-D warnings` PASS sur le candidat779e299 incluant le dernier filtre/test ; modifications suivantes limitées au cycle de vie DOM et documentation. Aucun runtime/dépendance ajouté.

## Révision du cycle de vie

La première revue indépendante du candidat779e299 demande une correction : une navigation vers une série masque le conseil sans détacher ses cibles d'affiches. Précision après inspection : le conseil est un panneau, pas la boîte de dialogue d'organisation ; fermer cette dernière ne libère donc pas ses observers. Le cycle de visibilité du panneau retire désormais ces cibles et invalide les réponses en vol, puis reprend les affiches encore manquantes au retour. Les résultats, leur ordre et le défilement restent conservés. Le test navigateur couvre cette course avec une réponse retenue jusqu'après navigation. La première revue défavorable reste historique, pas une approbation de livraison.

La qualité sémantique d'une réponse réelle du CLI et le rendu Tauri humain restent à valider ; une réponse fictive ne prouve ni la connexion fournisseur ni sa pertinence réelle. Aucune métadonnée du foyer envoyée à un fournisseur pendant le développement. Le consentement de conseil mentionne explicitement genres, réalisateurs et acteurs ; aucune nouvelle souscription ou clé.

## Remise et retour arrière

Candidat local à relire et packager. Ancien bundle conservé, aucune migration ; rouvrir le précédent client suffit pour revenir en arrière. Pas de déploiement ou de rollback serveur nécessaire, API2026-10-14/v1 inchangée. Pas de push GitHub ou de publication implicite.
