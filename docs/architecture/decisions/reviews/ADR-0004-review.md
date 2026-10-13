# Independent ADR Review

- **ADR**: `ADR-0004` at `CC-DOC-001`
- **Branch / Base Commit / Reviewed Commit**: `NOT_CREATED` / Cinéfolio `90c149b797b45cba8a56dca2b74eed06c4225060` / `NOT_CREATED`
- **Artifact Hash / Reviewed At UTC**: `a0f660d7650e02245d1eea577ddbecf783bb248211b0ad305a8cca7a892a7264` / `2026-10-13T15:00:47Z`
- **Author Instance**: `/root`
- **Reviewer Instance**: `/root/architecture_review`
- **Verdict**: `CHANGES_REQUESTED`

- [x] Le problème et les moteurs de décision sont étayés.
- [x] Trois options sont évaluées équitablement.
- [x] Le choix respecte l'absence de transcodage serveur et le client Mac Apple Silicon.
- [ ] Les contraintes normatives de `stream_cb` sont toutes capturées dans le contrat/enabler.
- [x] Migration, rollback et remplacement par ADR sont crédibles.
- [x] Les composants affectés sont identifiés.
- [x] Le gate utilisateur reste en attente ; l'ADR demeure `PROPOSED`.
- [x] Aucun lien de supersession n'est requis.

## Findings and Required Changes

| ID | Evidence | Risk | Required change |
| :--- | :--- | :--- | :--- |
| `ADR4-F-001` | La documentation primaire `mpv/stream_cb.h` consultée le 2026-10-13 qualifie l'API d'instable, impose un `read` bloquant, interdit d'appeler les API libmpv depuis les callbacks, permet aux streams de survivre au handle et n'offre `cancel_fn` qu'à partir de l'API 1.106. L'ADR mentionne seulement FFI/rendu/licences et un enabler général. | Deadlock, use-after-free, annulation inefficace lors d'un transfert, ou bundle utilisant une API trop ancienne ; la frontière critique TLS/lecture resterait sous-spécifiée. | Ajouter aux critères obligatoires de l'enabler : version/libmpv API minimale avec `cancel_fn`, pin exact, lifetime/refcount jusqu'à `close`, interdiction de réentrance libmpv, exécution bloquante hors runtime async, seek initial à 0, annulation concurrente et erreur après révocation. Maintenir le choix `PROPOSED` jusqu'à cette preuve ; tout moteur de remplacement exige un ADR. |
