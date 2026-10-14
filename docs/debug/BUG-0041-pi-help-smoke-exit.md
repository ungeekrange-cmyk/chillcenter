# BUG-0041 — Contrôle d'exécution Linux suppose une aide exit0

2026-10-14. ROOT_CAUSE_CONFIRMED, diagnostic `/root`, correction distincte `/root/pi_deployer`.

Symptôme gelé : source126da0a/binaire9a17ffe5, archive r6 b5c20d97/scriptf6acfeac/gate r6. Sur Pi via master utilisateur, socket/artefact/config/preflight/ELF64/lddPASS, staging privé créé, puis arrêt1 avant sudo/useradd/unit. Attendu : preuve que le binaire peut démarrer sans démarrer de serveur ; réel : le contrôle `set-e` considère l'aide comme échec.

H1 seule : la CLI revue ne traite pas `--help` comme succès mais retourne volontairement son usage pour un argument non reconnu. Prévision : chaîne d'usage connue surstderr et exit1. Expérience RO SSH : exécuté le binaire exact du staging avec --help, sortie5lignes identique à main.rs:48 préfixée `Chill Center : `, puis HELP_EXIT=1 ; systemctlLOADnotfound/inactive, comptechillcenterabsent. CONFIRMED. Le binaire est effectivement exécutable sur vraiARM64, ldd charge libc/libm/libpthread/libdl/ld-linux existants. Aucun syscall d'installation privilégiée exécuté.

Correction causale autorisée : installer opérateur seulement, vérifier explicitement exit1 et sortieusage exacte pour ce binaire verrouillé, refuser tout autre code/signal/sortie. Test local RED ancien simple-sete/--help, GREEN gardecontractuel, négatifsconfig/error/crash/wrongusage. Ne pas changer la CLI produit/binaire pour éviter une modification fonctionnelle non nécessaire ; ne pas ignorer globalement les erreurs. Nouveau hash script/manifest/gate et relectures indépendantes avant tentative ; stagingr6 conservé, aucun rerun dans même chemin ni destruction de données.
