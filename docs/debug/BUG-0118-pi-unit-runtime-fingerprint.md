# BUG0118 — Pi deployment compares runtime state as configuration

- Priority: P1, deployment operator; not a defect in the published v0.7 packages.
- Status: FIXED / VERIFIED_PI_GUARD, 20 October 2026.
- Affected operator: `24f62ac2f4da95fd51bf01f81ab934b35762f888`.
- Fix: `815b523a383e3d0a61e1b116f27495e1ca2584d0`, independent scoped approval.

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

Résultat : la comparaison conserve User/Group/FragmentPath statiques et
l'empreinte du contenu effectif `systemctl cat`. Le test complet fait varier
PID et heure au redémarrage ; les sept tests ciblés passent. La seconde tentative
a répondu `deployment=PASS`. Le vérificateur indépendant a confirmé les
empreintes statiques identiques, le serveur v0.7 actif et les données inchangées.
Le gate global reste distinct dans le rapport de vérification ; cette clôture
concerne le garde fautif.

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

Verified correction: static identity and the effective unit-content hash replace
the runtime structure. Seven focused tests pass, including a full restart with
changing process metadata. The second real deployment returned `deployment=PASS`;
the independent verifier confirmed equal static fingerprints, the active v0.7
server and unchanged data. Overall release verification is a separate gate; the
first failed attempt remains retained above.
