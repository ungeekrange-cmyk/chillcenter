# Independent ADR Review

- **ADR**: `ADR-0003` at `CC-DOC-001`
- **Branch / Base Commit / Reviewed Commit**: `NOT_CREATED` / Cinéfolio `90c149b797b45cba8a56dca2b74eed06c4225060` / `NOT_CREATED`
- **Artifact Hash / Reviewed At UTC**: `fd1501bd5f1dc40c28555f273bbb83cacae66d6b5858ce8bd94cf3b45c26a7e6` / `2026-10-13T15:00:47Z`
- **Author Instance**: `/root`
- **Reviewer Instance**: `/root/architecture_review`
- **Verdict**: `CHANGES_REQUESTED`

- [x] Le problème et les moteurs de décision sont étayés.
- [x] Trois options, dont deux alternatives rejetées, sont évaluées.
- [ ] Le contrat temporel et l'identité du média sont entièrement cohérents.
- [x] Sécurité, coût, observabilité et limites sans DRM sont exposés.
- [x] Migration et rollback sont décrits.
- [ ] Tous les consommateurs/invariants du bail sont suffisamment spécifiés.
- [x] Le gate utilisateur reste en attente ; l'ADR demeure `PROPOSED`.
- [x] Aucun lien de supersession n'est requis.

## Findings and Required Changes

| ID | Evidence | Risk | Required change |
| :--- | :--- | :--- | :--- |
| `ADR3-F-001` | `CONTRACTS.md` définit `playback_lease` sans `media_id` ni révision média, alors que les événements portent un média et que l'enchaînement peut réutiliser la session. | Un événement ou flux retardé de la vidéo précédente peut rester compatible avec session/génération et altérer la reprise ou servir des octets après passage au média suivant. | Lier chaque bail à `media_id` et à une révision/ETag, puis imposer une nouvelle génération ou une invalidation atomique à chaque changement de média ; ajouter le cas à `V-04`/`V-06`. |
| `ADR3-F-002` | Le transfert passe en `TRANSFERRING` et attend ACK ou expiration, mais ne dit pas si les heartbeats résiduels de A renouvellent encore l'échéance. | Un client qui continue de heartbeat sans traiter la pause peut empêcher indéfiniment le timeout promis. | À l'entrée en `TRANSFERRING`, figer une deadline monotone non renouvelable pour l'ancienne génération et refuser/neutraliser ses renouvellements, tout en autorisant uniquement l'ACK final borné. Tester la course heartbeat/transfert/expiration. |
