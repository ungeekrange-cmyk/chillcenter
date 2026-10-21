# REL-0.7.2 — revue indépendante d’intégration

- **Mode :** `INTEGRATION_REVIEW`
- **Candidate intégrée :** `5cf1bbe70f1dd4ee98d67ae691334377df265b73`
- **Sources runtime :** Mac/serveur/web `a38d4b16616ebd6735fa4aab0b518e28d767079e`; thèmes réutilisés `c603f29db2936ee40f7bf66484187b4e237d9bfd`
- **Delta opérateur :** `3d27456a3e2c46fb2dc8937b4e00370f85d54470`; SHA-256 `12a87083be5c18cdbabade89f8a248beb7952defe2f219d25138d0b4d4e56d0c`
- **Staging scellé :** `/private/tmp/chillcenter-v072-release.B82bwZ`
- **Reviewer :** `/root/v072_test_review`; intégrateur : `/root`
- **Verdict :** **APPROVED**

## Preuves reproduites

| Vérification | Résultat |
| :--- | :--- |
| `sha256sum -c SHA256SUMS` | PASS : les 22 assets listés correspondent. |
| Minisign, `SHA256SUMS`, provenance et quatre manifests avec `release.pub` | PASS : toutes signatures valides. |
| Provenance | PASS : release `v0.7.2`; Mac/serveur/web source `a38d4b1`; thèmes `c603f29`; versions 0.7.2/0.7.2/0.7.2/0.7.1; API `2026-10-14`; format/rendu thèmes 1/2. |
| Sources propres scellées | PASS : `a38d4b1`, sans changement. |
| Artefact thèmes réutilisé | PASS : archive livraison SHA-256 `22c3fbe4de190c5c64a7c5c7be28e152f4bbcf76e4e7068f4522fe0559dc1d47`; manifeste/signature thèmes v0.7.1 valides. |
| Wrapper serveur/web | PASS : tar serveur contient binaires, scripts signés, unités, manifests et guides FR/EN; ZIP web contient payload, script signé, manifests et guides FR/EN. |
| DMG Mac | PASS : SHA-256 `1d80517bdf1f5e4e079c14bc7ad6398b83de6347fc582fccf204140416b364f5`; `hdiutil verify` PASS; montage local lecture seule PASS; app montée identique au staging, `codesign --verify --deep --strict` PASS; version 0.7.2/build 11, minimum macOS 27.0. |
| Notes | PASS : `NOTES.md` est bilingue, décrit explicitement le périmètre, thèmes inchangés, signature ad hoc/non notarisation et préservation NAS/réglages/TLS. |

La provenance ne prétend pas un déploiement : `deployment_performed:false`. Les
preuves source précédentes sont récapitulées dans `docs/reviews/BUG-0127-tests.md` :
Rust ciblé 4 PASS, Node 24 PASS, DOM Chrome Mac/web PASS et harnais opérateur
8 PASS. Les revues sécurité/code indépendantes de `a38d4b1` et `3d27456` sont
favorables.

## Limites et rollback

Cette porte approuve les octets de staging et leur publication autorisée, pas une
GUI AppKit réelle, ni un Pi/NAS, ni une installation domestique ou un comportement
après déploiement. La signature Mac est ad hoc, non notariée. Aucun réglage, NAS,
TLS ou Pi n'a été contacté pendant cette revue. Rollback : conserver/restaurer les
archives précédentes; les binaires antérieurs ignorent le répertoire optionnel de
checks. Toute différence entre les octets publiés et ce staging annule ce verdict.
