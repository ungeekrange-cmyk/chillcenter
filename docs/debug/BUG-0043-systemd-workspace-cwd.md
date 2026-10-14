# BUG-0043 — Répertoire courant systemd et garde du stockage

2026-10-14. ROOT_CAUSE_CONFIRMED ; diagnostic `/root`, correcteur distinct `/root/pi_deployer`.

R7recoveryinstallefichiers/hash9a17/compte997:983+984/confroot0640 et activeunité, puis auto-restart. ObservéRO : NRestarts9/MainPID0/ExecMainStatus1, journal « Le workspace ne peut pas être la racine de données », pas de listener8443, seul parentstate0700créé. Installation matérielle réussie, démarrage nonfonctionnel.

H1 seule : unité sansWorkingDirectory utilise / ; operations.rs426 safe_layout refuse une data_dir imbriquée dans current_dir. /var/lib/chillcenter/data.starts_with(/) est vrai pour toute racineabsolue. Prédiction : erreur déterministe avant TLS/SQLite, rootcwdgardeTrue, cwd/usr/lib/chillcentergardeFalse. Lecturecode confirmeordreensure_no_pending avantpreparedata/TLS ; systemctlWorkingDirectoryvide ; expérimentationRO mêmeprédicatTruepour/Falsepour/usr/lib/chillcenter, binairelibdirroot0755existant. CONFIRMED. Pas de changement/désactivation du garde antiworkspace.

Servicecritique : déployeurautorisé arrête/désactiveuniquementchillcenter pourstopperloop. Preuves : inactive/disabled, bincodeSHAidentique, compte/groupe/confroot:chillcenter0640/statechillcenter0700 conservés, NAS/autresservicesinchangés. Vérificationrestaurationparnouvelleinstanceindépendante requise.

Correctionminimalecausale : WorkingDirectory=/usr/lib/chillcenter dansunitéLinuxfuture et drop-inopérateurimmuable de mêmevaleur pourinstallationR7conservée. Cwdrootownedread-only séparé desdatas ; pas rootUID, aucunélargissementReadWritePaths/IAM/Mount, aucunCoreRust/cert/socket/codec. Source/binaire126/9a17 restentinchangés, configurationeffectivecompositeR7+dropinliéeàhashes dansnouveaugate revu. Re-enable/start uniquementaprèsapprovals etrestaurationinactiveconfirmée, puisverifierTLS/API/NAS/perf distinct. Aucun restartdesautresservices/reboot/migration/suppression.
