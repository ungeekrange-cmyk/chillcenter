# US-025 — Contrôler le lancement automatique

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-006 — Enchaînement.
- Priorité : Must Have, périmètre validé.
- Complexité : 5 points ; Délais, disponibilité tardive et conflit d'appareil.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-024, US-023.
- Blocks : ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **régler et interrompre le lancement de la vidéo suivante**, afin de **garder le contrôle de l'enchaînement**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-025-AC-01 — Défaut et commandes

Trace : F-12, F-13, F-06.

```gherkin
Given une vidéo se termine avec une suivante disponible
When la proposition s'affiche
Then vignette, titre, Lancer maintenant et Ne pas lancer sont visibles ; le compte à rebours par défaut est dix secondes et démarre après cet affichage
```

### US-025-AC-02 — Tous les réglages

Trace : F-12, F-13, F-06.

```gherkin
Given les préférences de mon profil
When je choisis zéro, cinq, dix, vingt, trente secondes ou aucun lancement automatique
Then ce choix est conservé sur mes appareils ; zéro lance immédiatement si disponible, désactivation conserve la proposition sans compteur
```

### US-025-AC-03 — Action utilisateur

Trace : F-12, F-13, F-06.

```gherkin
Given une proposition de suite ouverte
When je choisis Lancer maintenant ou Ne pas lancer
Then le lancement se fait immédiatement ou cette proposition est annulée, sans changer la préférence enregistrée
```

### US-025-AC-04 — Indisponible ou nouveau lecteur

Trace : F-12, F-13, F-06.

```gherkin
Given la suivante disparaît avant le lancement, ou un autre appareil acquiert mon profil pendant le compte à rebours
When le délai se termine
Then aucune lecture n'est forcée ; message d'indisponibilité ou proposition de transfert sur confirmation ; une erreur de lecture ou un changement de profil annule aussi l'enchaînement
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-12, F-13, F-06 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

