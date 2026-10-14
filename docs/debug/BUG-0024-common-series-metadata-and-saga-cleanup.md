# BUG-0024 — Édition CAS/IA divergente des métadonnées série et sagas

2026-10-14 · auteur `/root/ai_server_contract` · état : FIXED_PENDING_INDEPENDENT_REVIEW.

## Symptôme et cause confirmée

Les routes d'édition à révision attendue et l'application IA appelaient
`MediaPatch::apply` directement. Contrairement à `Database::patch`, elles
n'appliquaient pas l'héritage des métadonnées d'une série cible. Une fiche film
reclassée en épisode perdait donc les métadonnées communes disponibles. Elles
laissaient aussi jusqu'à une future lecture les membres de saga et marqueurs
legacy, ce qui pouvait rendre un export incompatible avec l'import qui refuse
un épisode membre d'une saga film-only.

## Correction et preuves

`metadata_edit::apply_common_patch` exécute dans la transaction appelante la
lecture bornée de la série cible, l'héritage, le retrait immédiat des membres et
marqueurs de saga lorsque la catégorie n'est plus `film`, ainsi que le bump des
révisions de collection concernées. L'appelant conserve le bump de révision du
média. L'IA utilise ce helper et recherche aussi le champ `series` avec le même
échappement LIKE littéral. Son plafond vérifie maintenant le total de sagas
avant une création, sans refuser une saga existante.

La régression locale utilise uniquement SQLite temporaire : métadonnées
héritées, note personnelle conservée, UUID conservé, associations/markers
retirés et export sans l'épisode dans une saga. L'adaptateur de la route CAS
`Store::update_media_at_revision` est maintenant raccordé par le coordinateur,
ainsi que l'ancien helper public d'édition ; la régression utilise le vrai parcours CAS et vérifie le bump de révision et l'UUID. Tests exécutés dans la preuve du delta d'intégration avant clôture ; les rapports du candidat antérieur restent historiques.

## Retour et portée

Retour logiciel : retirer ce helper et son appel IA. Aucun NAS, fournisseur,
secret, fichier vidéo ou état personnel n'est modifié par ce correctif. Cette
note ne constitue pas une approbation indépendante.
