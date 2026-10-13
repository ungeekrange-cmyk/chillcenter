# US-026 — Retrouver l'historique après déplacement

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-007 — Conservation et administration.
- Priorité : Must Have, périmètre validé.
- Complexité : 5 points ; Identité, preuves et confirmations d'ambiguïté.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-006, US-013, US-014, US-021.
- Blocks : ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **gestionnaire du foyer**, je veux **réassocier une vidéo déplacée ou une racine changée**, afin de **conserver les notes, reprises et playlists du foyer**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-026-AC-01 — Racine changée

Trace : F-16, F-06.

```gherkin
Given les vidéos ont gardé leurs chemins relatifs sous une nouvelle racine
When je confirme la correspondance de racines puis actualise
Then les fiches et données de chaque profil sont conservées sans créer une nouvelle bibliothèque personnelle
```

### US-026-AC-02 — Déplacement identifié

Trace : F-16, F-06.

```gherkin
Given une vidéo déplacée est reconnue de façon certaine
When je lance une actualisation
Then sa localisation change et les références existantes conservent notes, reprise et playlists
```

### US-026-AC-03 — Correspondance probable

Trace : F-16, F-06.

```gherkin
Given une vidéo absente et un nouveau fichier présentent une correspondance seulement probable
When je consulte Administration
Then la proposition de réassociation est présentée sans fusion silencieuse ; la confirmer conserve l'historique
```

### US-026-AC-04 — Ambiguïté ou refus

Trace : F-16, F-06.

```gherkin
Given plusieurs candidats existent ou je refuse une réassociation
When j'actualise
Then l'ancienne référence et ses données restent conservées, les candidats ne sont pas fusionnés par titre
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-16, F-06 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

