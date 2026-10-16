# US-007 — Parcourir et rechercher le catalogue

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-002 — Catalogue commun.
- Priorité : Must Have, périmètre validé.
- Complexité : 5 points ; Catégories, recherche, tri et affichage de l'état personnel.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-003, US-006.
- Blocks : US-008, US-009, US-011, US-012, US-013, US-014, US-016, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **parcourir et filtrer les vidéos disponibles**, afin de **trouver rapidement quoi regarder**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-007-AC-01 — Catégories et présentation

Trace : F-14, F-06.

```gherkin
Given un catalogue de films, séries, spectacles et documentaires
When je consulte une catégorie ou Toute la collection
Then les fiches montrent les informations disponibles ; une série apparaît comme une carte regroupée plutôt que ses épisodes à plat
```

### US-007-AC-02 — Recherche et filtres

Trace : F-14, F-06.

```gherkin
Given des vidéos de genres, années et états vus différents
When je combine recherche, genre, année et vu/non vu puis un tri
Then les résultats respectent ces choix et l'état vu du profil actif ; une recherche d'épisode retrouve sa série
```

### US-007-AC-03 — Affichage

Trace : F-14, F-06.

```gherkin
Given des résultats de catalogue
When je choisis grille ou liste et modifie la taille des affiches
Then le catalogue reste consultable, titres longs tronqués visuellement mais complets dans leur fiche
```

### US-007-AC-04 — Fiche et informations absentes

Trace : F-14, F-06.

```gherkin
Given une vidéo sans certaines métadonnées
When j'ouvre sa fiche par le titre ou l'affiche
Then les informations manquantes sont signalées ; aucun lecteur ne démarre et le statut vu ne change pas
```

### US-007-AC-05 — Vidéos en relation

Trace : F-14.

```gherkin
Given une fiche de vidéo et d'autres vidéos disponibles partageant des genres, une saga ou des années proches
When je consulte sa présentation
Then des vidéos en relation du catalogue sont proposées sans inventer une fiche absente, sans démarrer la lecture et sans changer mon état vu
```

### US-007-AC-06 — Recommandations regroupées en rail

Complément demandé le16octobre2026, [ADR0020](../../architecture/decisions/ADR-0020-local-title-recommendations.md). Pour le client Mac, ce critère précise AC05 : une année proche ou une saga seule ne suffit plus.

```gherkin
Given une fiche et un catalogue local avec genres, réalisateurs ou acteurs communs
When je consulte les vidéos en relation
Then les correspondances sont classées et présentées sur une ligne de vignettes défilante gauche/droite
And une série apparaît une seule fois, sans épisode individuel, avec son identité propre même si une autre série porte le même nom
And cliquer ouvre la fiche du film ou la série complète, sans démarrage de lecture ni modification du profil
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-14, F-06 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

