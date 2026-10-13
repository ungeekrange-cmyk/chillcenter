# US-033 — Choisir ou désactiver une ambiance

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-009 — Apparence.
- Priorité : Must Have, périmètre validé.
- Complexité : 2 points ; Persistance et indépendance du mode clair/sombre.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-032.
- Blocks : ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **choisir une ambiance manuelle ou les désactiver**, afin de **personnaliser mon Mac indépendamment du calendrier**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-033-AC-01 — Manuel

Trace : F-20, F-21.

```gherkin
Given les ambiances automatiques sont actives
When je choisis une ambiance manuelle
Then elle reste sélectionnée après réouverture et changement de saison ou événement jusqu'à mon retour volontaire en automatique
```

### US-033-AC-02 — Désactiver

Trace : F-20, F-21.

```gherkin
Given une ambiance est active
When je désactive les ambiances
Then les couleurs de base du mode choisi restent appliquées, sans perdre mon choix clair/sombre/automatique
```

### US-033-AC-03 — Retour automatique

Trace : F-20, F-21.

```gherkin
Given une ambiance manuelle ou désactivée
When je réactive Automatique
Then l'ambiance de la date locale courante est appliquée
```

### US-033-AC-04 — Indépendance

Trace : F-20, F-21.

```gherkin
Given le même profil est actif sur deux Macs
When je choisis une ambiance ou change clair/sombre sur l'un
Then le réglage d'ambiance de l'autre ne change pas ; la variante du Mac modifié suit son mode d'affichage
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-20, F-21 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

