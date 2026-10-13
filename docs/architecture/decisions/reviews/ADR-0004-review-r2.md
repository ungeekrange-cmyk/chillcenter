# Independent ADR Delta Review — R2

- **ADR**: `ADR-0004` at `CC-DOC-001`, corrected delta
- **Branch / Base Commit / Reviewed Commit**: `NOT_CREATED` / Cinéfolio `90c149b797b45cba8a56dca2b74eed06c4225060` / `NOT_CREATED`
- **Artifact Hash / Reviewed At UTC**: `420e79856984533e2d73c7b0a3899bac2c8ae2c0f911931df054dc21ccd5601d` / `2026-10-13T15:07:17Z`
- **Related Hashes**: `ARCHITECTURE.md` `931b93441f39b131dc0a7623454c44d80fa8a2faeaad4abde02c505be3b2e251`; `VERIFICATION.md` `d241301b0bba8579d425dcbd01c77c5eb887d6fb2e6853845c371883fb92293b`; `SOURCES.md` `d4cfb81e028613f898aa936cd0f89bb3693aa529a70e6c1584a5d9e09b84632c`
- **Author Instance**: `/root`
- **Reviewer Instance**: `/root/architecture_review`
- **Prior Review**: `ADR-0004-review.md`, `CHANGES_REQUESTED`, SHA-256 `6f4b0872acdfbb49ed841a57a3276e708bd50de3fa02a26ab667974b9b431c64`
- **Verdict**: `APPROVED`

## Delta disposition

| Initial finding | Resolution evidence | Status |
| :--- | :--- | :--- |
| `ADR4-F-001` | Le dossier qualifie explicitement `stream_cb` d'instable, exige un moteur/ABI épinglé et une client API ≥1.106, isole les callbacks bloquants hors UI/runtime Tokio, interdit la réentrance libmpv, impose `cancel_fn` non bloquant, retour d'erreur après annulation et lifetime refcounté jusqu'à `close_fn`/destruction. `V-03` exige seek, annulation pendant IO, fermeture concurrente, absence de deadlock et fuite. | `RESOLVED` |

La faisabilité reste honnêtement `NOT_RUN` et conditionnée au premier enabler ; ce n'est pas une preuve d'implémentation. Un échec impose toujours un ADR de remplacement. Aucun nouveau finding. L'approbation est limitée au hash ci-dessus ; l'ADR reste `PROPOSED` jusqu'au gate utilisateur.
