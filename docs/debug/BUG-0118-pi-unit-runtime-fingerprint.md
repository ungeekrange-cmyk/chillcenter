# BUG0118 — Pi deployment compares runtime state as configuration

- Priority: P1, deployment operator; not a defect in the published v0.7 packages.
- Status: CONFIRMED / FIX_IN_PROGRESS, 20 October 2026.
- Affected operator: `24f62ac2f4da95fd51bf01f81ab934b35762f888`.

## Français

Le premier déploiement domestique a validé la sauvegarde hors ligne, démarré
le serveur v0.7 et servi le nouveau web, puis déclenché à tort
`persistent_state_changed`. Le retour arrière automatique a répondu
`recovery=PASS` et remis l'ancien binaire et l'ancien web en service.

La différence avant/après est exclusivement dans `systemctl show -p ExecStart` :
cette propriété contient aussi le PID et l'heure de démarrage. Ils changent
normalement lors d'un redémarrage. Les lignes de configuration, certificat,
absence de `library.json` et empreinte du jeton TMDB sont identiques.

Hypothèse unique confirmée par la différence réelle : l'opérateur confond
configuration persistante et état d'exécution. La correction doit comparer le
contenu effectif de l'unité et ses propriétés statiques, sans ignorer une
modification réelle de l'unité. Le test doit faire varier PID/heure après
redémarrage tout en conservant la configuration. Une nouvelle revue et une
nouvelle tentative sont nécessaires avant de conclure à un déploiement réussi.

Preuves privées : démarrage de la tentative à `2026-10-20T20:18:45Z`, comparaison
dans `pre-v07.s70gre`, ancien SHA-256 rétabli
`6fecabc69258e372d71730a23451f5ff47565f60cbbc352552c12d8ac39424ea`, service actif
après retour arrière. La sauvegarde et les arbres précédents sont conservés sur
le Pi ; aucune modification du NAS, des réglages ou du client Mac.

## English

The first household attempt validated its offline backup, started the v0.7
server and served the new web, then incorrectly failed the persistent-state
guard. Automatic rollback returned `recovery=PASS` and restored the previous
server/web pair. The actual before/after difference is confined to the runtime
PID and start time embedded in systemd's `ExecStart` property; protected
configuration, certificate, library-file absence and TMDB token hashes match.

Compare effective unit contents and static properties instead of the runtime
structure. Regression coverage must change PID/start time on restart while
keeping configuration fixed. A new independent review and successful deployment
verification are required; the failed attempt must not be relabelled successful.
Private backups and previous versions are retained. NAS, existing settings and
the Mac application are outside this correction's mutation scope.
