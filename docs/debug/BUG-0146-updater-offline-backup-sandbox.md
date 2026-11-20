# BUG0146 — Sauvegarde serveur dans le sandbox du helper

20 novembre 2026. ROOT_CAUSE_CONFIRMED par analyse source et restrictions
systemd effectives ; correction durable non livrée dans serveur0.7.8.

L’installation serveur0.7.8 par API détecte le manifeste signé, puis échoue
`apply_failed` : journal `offline server backup failed`, avant création d’un
nouveau dossier de sauvegarde. L’ancien serveur est redémarré normalement.
Identité/chmod des chemins et UID/GID du helper corrects, disque libre5.5Go.
Aucun média, historique ou secret modifié pendant le diagnostic.

L’unité réellement installée et `deploy/updates/chillcenter-update.service`
utilisent ProtectSystem=strict et n’autorisent en écriture que data/updates et
data/backups dans les données du service. Or `operations::backup` ouvre le
verrou data/.chillcenter-data.lock en lecture/écriture, puis appelle une
préparation qui chmod la racine data. Ces deux opérations sont incompatibles
avec les chemins autorisés. Le verrou échoue avant création du backup ;
autoriser seulement celui-ci laisserait ensuite l’échec chmod. L’errno précis
du processus lancé dans l’unité n’a pas été capturé : son stderr est borné puis
jeté par le helper. Ne pas prétendre disposer d’une trace EROFS de ce processus.

Récupération réellement exécutée : helper signé existant,
`apply server --retry-accepted`, root local sans force et umask0077, hors unité
restreinte. Sauvegarde complète aa35049b-10b9-4870-8d03-a72aaadfd839 (1929fichiers,
manifeste final privé) ; installation du slot0.7.8-9 et santé réussies. SHA256 du
binaire actif identique à l’asset public. Serveur et timer actifs. Les
restrictions de l’unité restent inchangées ; aucune ouverture de droits DB/TLS.
Ce succès ne prouve pas le bon fonctionnement des futures mises à jour serveur
via cette unité. Les mises à jour web n’ont pas cette sauvegarde offline.

Correction future : validation offline non mutante des chemins existants,
et autorisation/provisionnement du seul verrou de données du service ; garder
base SQLite/TLS en lecture seule et sauvegardes comme destination d’écriture.
Tester dans un environnement systemd isolé : backup complet, UID/GID, modes,
symlinks refusés, groupe/autres non autorisés, absence de mutation DB/TLS,
échec avec redémarrage de l’ancienne version, nouvel accès de lecture interdit
pendant la maintenance. Ne pas élargir simplement ReadWritePaths à toute data.

Preuves privées sous /private/tmp/chillcenter-web-wake.yBYp2S :
restart-pi-helper-journal.log, restart-backup-identity.log,
restart-backup-sandbox.log, restart-root-recovery-trace.log (fd2 uniquement),
restart-server-installed-proof.log. Aucun contenu de clé capturé.
