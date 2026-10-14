# BUG-0021 — `iconutil` refuse l'iconset généré

- Date : 2026-10-14.
- Type : observation d'outillage de packaging, pas défaut fonctionnel.
- Environnement : macOS Apple Silicon, `/usr/bin/iconutil`, `/usr/bin/sips`,
  branche `codex/mac-bundle-complete`, base `76b3fb5`.
- Attendu : convertir les dix PNG canoniques 16–1024 px en `AppIcon.icns`.
- Réel : `iconutil` retourne deux fois `Invalid Iconset` et code 1.

## Reproduction et hypothèses

H1 — dimensions ou noms invalides : réfutée. `sips` et `file` confirment les dix
noms canoniques et leurs dimensions 16, 32, 64, 128, 256, 512 et 1024 px ; les
xattrs retirés ne changent pas le résultat.

H2 — le convertisseur de cet hôte refuse cet iconset sans diagnostic plus fin :
confirmée comme limite d'outillage reproductible. `actool` n'est pas disponible
dans les Command Line Tools installés. Le packaging ne doit pas dépendre d'un
outil qui refuse son entrée canonique.

## Correction et régression

`scripts/helpers/make_icns.py` écrit le conteneur ICNS documenté avec les chunks
PNG modernes `icp4`, `icp5`, `icp6`, `ic07`, `ic08`, `ic09`, `ic10`, tous
dérivés de `icons/branding/vector/default.png`. Le bundle conserve aussi le SVG
vectorisé approuvé comme provenance.

Sur le candidat final, `file` reconnaît l'ICNS, `iconutil` l'extrait avec succès,
`CFBundleIconFile` le référence et la signature stricte du bundle reste valide.
L'ouverture Finder et l'inspection visuelle restent **NOT_RUN**. Aucun asset de
marque source ni code métier n'est modifié. Rollback : revenir au script de
développement sans icône ; aucun état utilisateur n'est affecté.
