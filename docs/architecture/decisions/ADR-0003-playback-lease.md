# ADR-0003 — Exclusivité de lecture et reprise ordonnée

- Statut : `ACCEPTED` ; révision `CC-DOC-001` ; 2026-10-13. Validation : [G-02](../../gates/G-02-CC-DOC-001.md), proposition originale/hash conservés dans le manifeste historique.
- Propriétaire/auteur : `/root` ; reviewer : voir rapport indépendant.
- Baseline : `90c149b797b45cba8a56dca2b74eed06c4225060` ; commit Chill Center : `NOT_CREATED`.
- Supersedes/superseded_by : `NONE` ; exigences F-06–08, F-12–13.

## Contexte et options

L'utilisateur a choisi une lecture par profil transférable sur confirmation. Des événements retardés et des octets déjà en tampon rendent un simple verrou HTTP insuffisant.

| Option | Bénéfices | Risques/coûts | Réversibilité |
| --- | --- | --- | --- |
| Bail avec génération, ACK de pause et échéance locale | Ordre explicite, panne récupérable, ancien client neutralisé | Protocole et tests de course | Moyenne, contrat versionné |
| Verrou sans expiration | Simple en fonctionnement normal | Client crashé bloque durablement le profil | Élevée |
| Dernière écriture gagne sans exclusivité | Peu de coordination | Contredit le choix utilisateur, reprise recule | Élevée mais rejetée |

## Décision proposée

Bail serveur 30 s, renouvelé toutes les 5 s ; liaison explicite au média/révision, session/génération/seq et epoch de boot. Toute nouvelle vidéo reçoit une génération qui invalide atomiquement flux/événements du média précédent. `TRANSFERRING` fige l'échéance ancienne et refuse son renouvellement ; accepter un seul ACK final lié au média avant cette échéance, sinon attendre l'expiration avant activation du nouveau. Le client se met en pause à échéance même avec tampon. Progression toutes les 5 s et sur transitions. Compteur final : secondes réelles cumulées dans le dernier dixième ; interprétation à valider avec la documentation.

## Conséquences et validation

Positif : vieux paquets refusés, reprise stable, deux transferts concurrents arbitrés. Coût : attendre jusqu'à l'échéance si ancien Mac inaccessible ; dernière position peut perdre environ 5 s après panne brutale. Limite : pas de contrôle DRM d'un client malveillant ni preuve d'attention humaine. Logs d'états et de conflits sans noms de profils. Pas de consommation cloud ; heartbeat léger, écritures bornées.

V-04/V-05/V-06/V-12 : coupures, suspension, buffers, seuil et redémarrage. Migrer la reprise vers états personnels ; baux jamais importés/restaurés. Rollback : arrêter clients incompatibles, restaurer schéma compatible, tous baux invalidés. Changer un délai/protocole nécessite revue des contrats et courses temporelles.

Revue initiale : [ADR-0003-review](reviews/ADR-0003-review.md), `CHANGES_REQUESTED`. Revue du delta : [ADR-0003-review-r2](reviews/ADR-0003-review-r2.md). Gate utilisateur : [TRACEABILITY](../../TRACEABILITY.md), étape 2 `APPROVED`, preuve [G-02](../../gates/G-02-CC-DOC-001.md).
