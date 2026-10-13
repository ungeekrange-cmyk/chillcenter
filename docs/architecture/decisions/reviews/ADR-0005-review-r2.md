# Independent ADR Delta Review — R2

- **ADR**: `ADR-0005` at `CC-DOC-001`, corrected delta
- **Branch / Base Commit / Reviewed Commit**: `NOT_CREATED` / Cinéfolio `90c149b797b45cba8a56dca2b74eed06c4225060` / `NOT_CREATED`
- **Artifact Hash / Reviewed At UTC**: `1d493e7b5ca10c4938ed18292eca8cd29f6c703acf06aee3a00b766f7a8a335f` / `2026-10-13T15:07:17Z`
- **Related Hashes**: `ARCHITECTURE.md` `931b93441f39b131dc0a7623454c44d80fa8a2faeaad4abde02c505be3b2e251`; `CONTRACTS.md` `727de8ae7def76f9f1c753b786ff1647618e2826f5ef11f9efb0e2868b7f5299`; `VERIFICATION.md` `d241301b0bba8579d425dcbd01c77c5eb887d6fb2e6853845c371883fb92293b`
- **Author Instance**: `/root`
- **Reviewer Instance**: `/root/architecture_review`
- **Prior Review**: `ADR-0005-review.md`, `CHANGES_REQUESTED`, SHA-256 `1cfad7a93dab735a5831c210741934863f6e95e50989bc9f2dd55c0409033297`
- **Verdict**: `APPROVED`

## Delta disposition

| Initial finding | Resolution evidence | Status |
| :--- | :--- | :--- |
| `ADR5-F-001` | Le certificat annuel conserve une clé de service stable et le client épingle sa SPKI. Le verifier décide au handshake à partir de SPKI, SAN, période, structure et signatures TLS ; aucun téléchargement HTTP préalable n'est requis. Le changement de clé avant expiration demande confirmation, après expiration conserve exactement le fallback accepté par l'utilisateur. `V-09` couvre client absent, nouveau certificat expiré, SAN/signatures invalides et remplacement urgent. | `RESOLVED` |

Le coût de la clé longue durée est explicitement compensé par droits `0600`, compte de service, sauvegarde séparée et rotation d'urgence avec réapprobation ; aucune CA ou dépendance cloud n'est ajoutée. Aucun nouveau finding. L'approbation est limitée au hash ci-dessus ; l'ADR reste `PROPOSED` jusqu'au gate utilisateur.
