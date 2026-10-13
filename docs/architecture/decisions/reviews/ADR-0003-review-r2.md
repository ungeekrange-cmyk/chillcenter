# Independent ADR Delta Review — R2

- **ADR**: `ADR-0003` at `CC-DOC-001`, corrected delta
- **Branch / Base Commit / Reviewed Commit**: `NOT_CREATED` / Cinéfolio `90c149b797b45cba8a56dca2b74eed06c4225060` / `NOT_CREATED`
- **Artifact Hash / Reviewed At UTC**: `b334456227bddae0f52d6a98d3961ea0462e021e9f6e39e1abd179e21d0b60dc` / `2026-10-13T15:07:17Z`
- **Related Hashes**: `ARCHITECTURE.md` `931b93441f39b131dc0a7623454c44d80fa8a2faeaad4abde02c505be3b2e251`; `CONTRACTS.md` `727de8ae7def76f9f1c753b786ff1647618e2826f5ef11f9efb0e2868b7f5299`; `VERIFICATION.md` `d241301b0bba8579d425dcbd01c77c5eb887d6fb2e6853845c371883fb92293b`
- **Author Instance**: `/root`
- **Reviewer Instance**: `/root/architecture_review`
- **Prior Review**: `ADR-0003-review.md`, `CHANGES_REQUESTED`, SHA-256 `b7928938cec38590b1a59675995b460344a73bcf4972d4b042b2566d5722f16c`
- **Verdict**: `APPROVED`

## Delta disposition

| Initial finding | Resolution evidence | Status |
| :--- | :--- | :--- |
| `ADR3-F-001` | Le bail porte désormais `media_id` et `media_revision`; chaque acquisition/vidéo obtient une nouvelle génération et invalide atomiquement les flux/événements précédents. `ended` libère le bail avant que l'autoplay tente une nouvelle acquisition. `V-04` couvre le rejet des anciens événements/flux. | `RESOLVED` |
| `ADR3-F-002` | L'entrée en `TRANSFERRING` fige `transfer_deadline`, refuse les renouvellements de A et n'accepte qu'un ACK final unique, lié au média/génération et reçu avant la deadline. Les heartbeats retardés ne peuvent plus prolonger l'attente. | `RESOLVED` |

Les courses autoplay/autre appareil sont également closes : l'acquisition concurrente annule l'autoplay et exige une confirmation de transfert, sans transfert forcé. Aucun nouveau finding. L'approbation est limitée au hash ci-dessus ; l'ADR reste `PROPOSED` jusqu'au gate utilisateur de l'étape 2.
