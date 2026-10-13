# US-003 — Choisir et retrouver son profil

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-001 — Accès et profils.
- Priorité : Must Have, périmètre validé.
- Complexité : 3 points ; Persistance par appareil/serveur et changement pendant lecture.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-002, ENABLER-005.
- Blocks : US-004, US-007, US-016, US-018, US-031, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **membre du foyer**, je veux **choisir mon profil puis retrouver le dernier utilisé sur mon Mac**, afin de **ouvrir directement ma vidéothèque personnelle**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-003-AC-01 — Première ouverture

Trace : F-05, F-06.

```gherkin
Given un serveur connecté mais aucun profil précédemment choisi sur ce Mac
When j'ouvre l'application
Then le choix des profils s'affiche avant la consultation des données personnelles
```

### US-003-AC-02 — Ouverture suivante

Trace : F-05, F-06.

```gherkin
Given j'ai choisi un profil existant puis fermé l'application
When je la rouvre sur le même appareil et serveur
Then ce profil est actif et le sélecteur se trouve en haut à droite
```

### US-003-AC-03 — Changer pendant lecture

Trace : F-05, F-06.

```gherkin
Given une vidéo en lecture avec le profil A
When je choisis B dans le sélecteur
Then la lecture de A est mise en pause et sa reprise sauvegardée ; seules les données de B sont ensuite affichées
```

### US-003-AC-04 — Choix indépendant ou profil supprimé

Trace : F-05, F-06.

```gherkin
Given deux Macs utilisent des profils différents, ou mon dernier profil a été supprimé
When un autre Mac change de profil ou je rouvre mon Mac
Then le choix de l'autre Mac ne change pas le mien ; un profil supprimé provoque un nouveau choix explicite
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-05, F-06 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

