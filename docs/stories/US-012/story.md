# US-012 — Gérer les références devenues indisponibles

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-002 — Catalogue commun.
- Priorité : Must Have, périmètre validé.
- Complexité : 3 points ; Distinction indisponibilité, suppression de référence et réapparition.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-006, US-007.
- Blocks : ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **gestionnaire du foyer**, je veux **repérer et retirer les références absentes**, afin de **garder un catalogue compréhensible sans perdre des fichiers**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-012-AC-01 — Repérer

Trace : F-14, F-16.

```gherkin
Given un scan complet a confirmé des références absentes
When j'ouvre Indisponibles
Then les références et leurs chemins relatifs sont présentés ; le catalogue les conserve tant que je ne les retire pas
```

### US-012-AC-02 — Confirmer un retrait

Trace : F-14, F-16.

```gherkin
Given des références réellement indisponibles
When je confirme leur retrait individuel ou groupé
Then leurs fiches et associations locales sont retirées, jamais les fichiers NAS ; l'action avertit de la perte des historiques associés
```

### US-012-AC-03 — Protection

Trace : F-14, F-16.

```gherkin
Given une vidéo est redevenue disponible ou l'opération échoue
When je tente un retrait
Then la vidéo disponible est protégée et aucun retrait partiel non annoncé n'est appliqué
```

### US-012-AC-04 — Réapparition

Trace : F-14, F-16.

```gherkin
Given une vidéo précédemment indisponible est retrouvée
When je relance un scan complet
Then son identité et historique sont réutilisés si sa référence existait encore ; les données volontairement supprimées ne sont pas inventées
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-14, F-16 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

