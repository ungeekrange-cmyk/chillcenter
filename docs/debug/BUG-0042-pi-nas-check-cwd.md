# BUG-0042 — Test NAS après changement d'UID conserve un cwd privé

2026-10-14, ROOT_CAUSE_CONFIRMED ; diagnostic `/root`, auteurcorrectif distinct `/root/pi_deployer`.

Symptôme r7 : runtime/ELF/ldd/contratusagePASS, useradd exécuté, puis find sous serviceUIDéchoue « Failed to restore initial working directory: /home/admin: Permission denied ». Binaire/unit/config/start non atteints. Source126/binary9a17/archiveed485f99/scriptd3cd903f ; pas de problème des vidéos ou du groupe NAS.

H1 seule : cwd hérité /home/admin inaccessible à UIDservice, GNUfind peut énumérer NAS mais son rétablissement du cwd échoue. Prévision : même compte/montage/commande réussit uniquement depuis /. Expérience RO SSH : id997/gid983/group984 et shellnologin confirmés ; unitLOADnotfound/inactive ; /usr/lib/chillcenter,/etc/chillcenter,/var/lib/chillcenterabsents. Testoriginalcwdexit1, même commande précédée de `cd /` exit0. CONFIRMED, seuls cwd changés, aucun fichier NAS lu/affiché/modifié.

Correction causale : `cd /` avant le test de compte NAS dans installerpremièreinstallation. État partiel à préserver : compteuser997/group983 déjàcréé et explicitementautorisé, appartenance984 conforme. Ne pas réexécuter useradd ni contourner arbitrairement le garde premièreinstallation. Un complément opérateur distinct peut terminer r7 après vérifications exactes compte/groupe/shell, tous cheminsserviceabsents, stagingr7/archive/binaire/config/unitéidentiques au gate. Revue indépendante du script de reprise et hash avant installation ; source/binaireproduit restent inchangés. Aucun userdel, suppressionNAS, remount ou autre service.
