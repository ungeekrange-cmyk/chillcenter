# US-001 — Choisir et connecter un serveur

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-001 — Accès et profils.
- Priorité : Must Have, périmètre validé.
- Complexité : 3 points ; Choix d'adresse, confiance initiale et erreurs de connexion.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : ENABLER-002, ENABLER-003.
- Blocks : US-002, US-005, US-006, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **membre du foyer**, je veux **connecter mon Mac au serveur de mon choix par son nom ou son adresse**, afin de **accéder à la vidéothèque sans monter le NAS**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-001-AC-01 — Connexion initiale

Trace : F-02, F-03, F-18, F-19.

```gherkin
Given un serveur disponible et une adresse DNS ou IP valide
When je me connecte et accepte l'adresse et l'empreinte présentées
Then la connexion est enregistrée et le choix du profil est proposé ; aucune donnée du NAS n'est demandée au Mac
```

### US-001-AC-02 — Refus de confiance

Trace : F-02, F-03, F-18, F-19.

```gherkin
Given une première connexion non encore approuvée
When je refuse le certificat présenté
Then aucun catalogue, profil ou flux vidéo n'est récupéré et je peux modifier l'adresse
```

### US-001-AC-03 — Adresse erronée ou serveur absent

Trace : F-02, F-03, F-18, F-19.

```gherkin
Given une adresse vide, invalide ou un serveur inaccessible
When je demande la connexion
Then un message précis permet de corriger ou réessayer sans effacer les serveurs enregistrés
```

### US-001-AC-04 — Serveur différent

Trace : F-02, F-03, F-18, F-19.

```gherkin
Given un serveur déjà enregistré sur mon Mac
When je sélectionne un autre serveur
Then les profils et le dernier choix affichés appartiennent au serveur choisi, sans mélange des bibliothèques
```

## 5. Contraintes fonctionnelles de qualité

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-02, F-03, F-18, F-19 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

