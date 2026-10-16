# US-021 — Reprendre au dernier moment regardé

## 1. Hiérarchie et contexte fonctionnel

- Epic : EPIC-005 — Reprise, vu et exclusivité.
- Priorité : Must Have, périmètre validé.
- Complexité : 5 points ; Reprise sur transitions, crash et événements retardés.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.

## 2. Dépendances et séquencement

- Depends On : US-016, ENABLER-005.
- Blocks : US-022, US-023, US-026, US-027, ENABLER-007.
- Gate utilisateur G-03 en attente : aucune implémentation commencée.

## 3. Valeur fonctionnelle

> En tant que **spectateur**, je veux **reprendre la dernière position de mon profil sur un autre appareil**, afin de **continuer sans rechercher le passage**.

Cette story décrit le résultat observable. Les détails de bibliothèque, persistance, transport et sécurité restent dans l'architecture acceptée et les travaux techniques liés.

## 4. Critères d'acceptation

### US-021-AC-01 — Pause et autre Mac

Trace : F-06, F-07.

```gherkin
Given j'ai mis une vidéo en pause à une position connue
When je la lance avec le même profil sur un autre appareil
Then elle reprend à la dernière position enregistrée, sans utiliser la position d'un autre profil
```

### US-021-AC-02 — Naviguer et fermer

Trace : F-06, F-07.

```gherkin
Given je choisis volontairement une nouvelle position, même proche du début
When je ferme normalement le lecteur puis le rouvre
Then cette nouvelle position est conservée
```

### US-021-AC-03 — Coupure

Trace : F-06, F-07.

```gherkin
Given la lecture s'interrompt brutalement sans dernier envoi
When je reprends après reconnexion
Then la dernière sauvegarde périodique est utilisée ; l'intervalle nominal de sauvegarde est cinq secondes, sans promesse pendant une panne réseau
```

### US-021-AC-04 — Ancien appareil

Trace : F-06, F-07.

```gherkin
Given une lecture plus récente a été confirmée sur B
When A se ferme tardivement ou se reconnecte avec une ancienne position
Then la reprise de B n'est pas écrasée
```

## 5. Contraintes fonctionnelles de qualité

Amendement16octobre2026, **US-021-AC-05**, ADR0022 : Pause garde le moteur, le bail et le tampon, et continue l'anticipation bornée par le réglage Mio. Pas de crédit vu pendant la pause ; position synchronisée et heartbeat maintenu. Reprise normale sur la même instance ; autre appareil doit confirmer le transfert tant que le lecteur reste ouvert. Cette précision remplace la libération systématique à la pause, pas les protections de fermeture/transfert/échéance.

Interface française, erreurs explicites, commandes accessibles au clavier et confirmations visibles. Aucun état du profil précédent ne doit apparaître après changement. Les budgets ressources et vérifications physiques sont ceux de l'architecture et de VERIFICATION ; aucune performance non mesurée n'est revendiquée.

## 6. Traçabilité et revue

- Exigences : F-06, F-07 ; source CC-FONC-002.
- Architecture : CC-DOC-001, validée dans [G-02](../../gates/G-02-CC-DOC-001.md).
- Auteur : /root ; révision : CC-STORIES-001.
- Revue : [rapport indépendant](../../reviews/STORY_REVIEW_REPORT.md), en attente au moment de rédaction.
- AC stables : les IDs ne seront pas recyclés ; tout changement matériel requiert une nouvelle revue du consommateur.

