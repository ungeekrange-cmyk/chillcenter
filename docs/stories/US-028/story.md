# US-028 — Importer les données de Cinéfolio ou Chill Center

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-007 — Conservation et administration.
- Priorité : Must Have, périmètre validé.
- Complexité : 5 points ; Aperçu, profils/racines, idempotence et invalidité.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-027, MIG-001.
- Blocks : US-029, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **gestionnaire du foyer**, je veux **importer une archive et contrôler son affectation**, afin de **retrouver mes informations dans la bonne bibliothèque et les bons profils**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-028-AC-01 — Cinéfolio

Trace : F-17, F-16, F-18.

```gherkin
Given une archive exportée des informations Cinéfolio et un profil de destination
When je choisis les racines correspondantes, vérifie l'aperçu puis confirme
Then métadonnées et sagas sont communes ; notes, vus et playlists hérités vont au profil choisi ; la source et ses vidéos restent intactes
```

### US-028-AC-02 — Chill Center

Trace : F-17, F-16, F-18.

```gherkin
Given une archive valide contenant plusieurs profils
When je confirme leur correspondance et les racines après aperçu
Then les données personnelles restent liées aux profils appropriés et les reprises/ordres sont retrouvés
```

### US-028-AC-03 — Répétition et conflits

Trace : F-17, F-16, F-18.

```gherkin
Given un export déjà importé ou des données de destination plus récentes
When je demande un nouvel import
Then les doublons sont évités et les conflits apparaissent avant application ; aucune fusion personnelle implicite
```

### US-028-AC-04 — Annuler ou invalide

Trace : F-17, F-16, F-18.

```gherkin
Given un aperçu ouvert ou une archive invalide, trop volumineuse, de version inconnue ou dont l'import échoue
When j'annule ou tente de confirmer
Then la destination est conservée sans import partiel et un message permet de corriger
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-17, F-16, F-18 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

