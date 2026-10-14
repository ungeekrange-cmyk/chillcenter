# Gate post-déploiement Pi — périmètre runtime

Status: VERIFIED_SCOPED

14 octobre2026. Verdict indépendant `/root/pi_release_verifier` APPROVED_SCOPED dans [rapport](REL-PI-001-verification.md), après installation source126/binary9a17 + archiveR7/overlayWorkingDirectory. Ce gate ferme uniquement les checks exécutés : identité/service/TLS strict/permissions/confinement/NASRO/coexistence/repos ; pas toutes les stories ni les charges longues encore en cours.

Retour sûr du premier échec confirmé par une nouvelle instance `/root/pi_restoration_verifier`, [rapport conservé](REL-PI-001-restoration-verification.md) APPROVED_RESTORATION. Sources/archive/config/bin/account/NAS conservés ; seul le serviceChillCenter avait été arrêté/désactivé avant le correctif.

Les nouvelles preuves médias et NFR se rattachent à [la fiche](REL-PI-001-media-evidence.md) et devront être jugées sur leurs résultats réels. Pas de clôture générale ENABLER-006 ou133AC sur ce gate. Les opérations non exécutées gardent NOT_RUN et les erreurs historiques ne sont pas reclassées enPASS.
