# BUG-0025 — Les sagas anciennes sans tables collections contournent les limites

2026-10-14 ; revue indépendante `d94418e` CHANGES_REQUESTED/P1 ; auteur correction root. Cause : la branche fallback groupe des noms de saga sans appliquer les plafonds de la branche collections. Attendu1000 membres/saga et1000 collections ; réel1001+ admissibles sur anciennes bases, malgré contrat annoncé.

H1 confirmée par code et fixture SQLite sans tables collections :1001 médias dans une saga, puis1001 sagas distinctes. Correction au responsable : plafond avant chaque push/création du groupe, même constanteMAX_MEMBERS1000. Une erreur précède l'application au scratch ; source identique et aucun profil partiel. La régression est exécutée avant clôture dans les preuves du lot. Retour : commit antérieur sur branche isolée, pas de migration/data supprimée. US027/028/MIG001 ; aucun NAS touché.
