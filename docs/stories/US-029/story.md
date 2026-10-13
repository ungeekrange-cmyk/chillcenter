# US-029 — Restaurer la bibliothèque depuis une sauvegarde

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-007 — Conservation et administration.
- Priorité : Must Have, périmètre validé.
- Complexité : 5 points ; Restauration distincte d'import, arrêt des lectures et retour.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-027, US-028, MIG-002.
- Blocks : ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **gestionnaire du foyer**, je veux **restaurer une sauvegarde cohérente de la bibliothèque**, afin de **récupérer les informations du foyer après une panne**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-029-AC-01 — Restaurer

Trace : F-16, F-18.

```gherkin
Given une sauvegarde opérationnelle cohérente et une restauration explicitement confirmée
When la procédure arrête les lectures et restaure les données
Then catalogue, profils, affiches, notes, reprises et playlists de cette sauvegarde sont retrouvés
```

### US-029-AC-02 — Avertir

Trace : F-16, F-18.

```gherkin
Given la destination contient des données créées depuis la sauvegarde
When je prépare une restauration remplaçante
Then la perte éventuelle de ces données est annoncée et une sauvegarde de la destination est réalisée avant remplacement
```

### US-029-AC-03 — Échec

Trace : F-16, F-18.

```gherkin
Given la sauvegarde est incompatible, corrompue ou l'application de la restauration échoue
When je lance la procédure
Then une erreur est affichée et le retour à la sauvegarde préalable est documenté, sans toucher aux vidéos
```

### US-029-AC-04 — Reconnexion

Trace : F-16, F-18.

```gherkin
Given une restauration ou un redémarrage vient de se terminer
When les Macs se reconnectent
Then les anciennes lectures ne reprennent pas automatiquement comme encore actives et chacun retrouve un profil existant ou un choix explicite
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-16, F-18 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

