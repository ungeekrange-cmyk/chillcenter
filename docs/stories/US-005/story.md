# US-005 — Rester connecté lors du renouvellement du certificat

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-001 — Accès et profils.
- Priorité : Must Have, périmètre validé.
- Complexité : 5 points ; Calendrier, absence prolongée et exception de confiance approuvée.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-001.
- Blocks : ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **membre du foyer**, je veux **retrouver mon serveur après renouvellement de son certificat**, afin de **continuer à consulter et lire mes vidéos sans manipulations annuelles**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-005-AC-01 — Renouvellement normal

Trace : F-19.

```gherkin
Given mon serveur approuvé renouvelle après douze mois un certificat valable dix-huit mois en conservant sa clé
When je me reconnecte, même après une absence pendant le renouvellement
Then le nouveau certificat valide pour l'adresse est reconnu automatiquement et lecture et catalogue restent accessibles
```

### US-005-AC-02 — Clé changée avant expiration

Trace : F-19.

```gherkin
Given la clé présentée diffère et mon ancien certificat connu reste valide
When je me connecte
Then une nouvelle approbation explicite est demandée avant l'accès aux données
```

### US-005-AC-03 — Ancien certificat expiré

Trace : F-19.

```gherkin
Given mon certificat enregistré a expiré et le serveur présente un nouveau certificat valide pour l'adresse
When je me reconnecte
Then le nouveau certificat est accepté automatiquement pour ce serveur enregistré et cette acceptation est signalée
```

### US-005-AC-04 — Certificat invalide

Trace : F-19.

```gherkin
Given le certificat présenté est lui-même expiré, ne correspond pas à l'adresse ou ne permet pas de vérifier la connexion
When je tente un accès au catalogue, à la vidéo ou aux sous-titres
Then la connexion est refusée de façon cohérente ; l'expiration de l'ancien ne supprime pas ces vérifications
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-19 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

