# US-027 — Exporter les informations de sa bibliothèque

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-007 — Conservation et administration.
- Priorité : Must Have, périmètre validé.
- Complexité : 3 points ; Export cohérent, exclusion de secrets et erreurs de stockage.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-013, US-015, US-021, ENABLER-004.
- Blocks : US-028, US-029, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **gestionnaire du foyer**, je veux **exporter les informations et affiches depuis Administration**, afin de **les conserver et les transférer sans recopier mes vidéos**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-027-AC-01 — Contenu

Trace : F-17, F-16, F-18.

```gherkin
Given un catalogue avec sagas, profils, notes, reprises et playlists
When je lance l'export depuis Administration
Then une archive contenant les informations versionnées et affiches est produite avec leurs profils et ordres
```

### US-027-AC-02 — Exclusions

Trace : F-17, F-16, F-18.

```gherkin
Given le serveur possède vidéos et configurations secrètes
When j'exporte
Then les vidéos, jetons, identifiants NAS, clés privées et lectures actives ne sont pas inclus
```

### US-027-AC-03 — Cohérence

Trace : F-17, F-16, F-18.

```gherkin
Given des lectures ou modifications ont lieu pendant export
When l'archive est créée
Then elle représente un état cohérent et son récapitulatif indique le périmètre exporté
```

### US-027-AC-04 — Échec ou taille

Trace : F-17, F-16, F-18.

```gherkin
Given l'export manque d'espace ou dépasse les limites d'import annoncées
When je le lance
Then un message explique l'échec ou la limite ; une archive incomplète n'est pas présentée comme restaurable
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-17, F-16, F-18 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

