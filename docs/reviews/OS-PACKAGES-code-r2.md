# Operational Delta Review — OS package maintenance R2

- **Review Type / Reviewer Instance:** code / `/root/os_maintenance_code`
- **Parent Reviewed SHA / Verdict:**
  `6abfc719bff2e8eadd4658d34fb6925a4762df4d` / `APPROVED_SCOPED`,
  [OS-PACKAGES-code.md](OS-PACKAGES-code.md)
- **New Source SHA:** `1d0f03395dc600a2c289701cf0e42928edf4d284`
- **Source Delta Scope:** `scripts/test-os-package-maintenance.py` only for
  executable behavior; the three maintenance runtime files and the bilingual
  operations guide are byte-identical to the parent-reviewed SHA. The three
  historical review reports added by the commit are evidence carriers.
- **Test Evidence Validity:** runtime/code conclusions remain valid; the new
  test harness requires correction and a fresh exact-SHA rerun.
- **Current Verdict:** `CHANGES_REQUESTED`
- **Limitations / Required Follow-up:** fix `CODE-R2-F-001`, freeze a new
  test-only SHA, then repeat the accelerated code/test/security evaluation.

FR : le runtime approuvé à R1 est inchangé, mais le nouveau test `TERM` a échoué
lors de la première exécution reviewer et ne nettoie pas son sous-processus sur
ce chemin. EN: the approved runtime is unchanged, but the new `TERM` test failed
on the reviewer's first full-suite run and leaks its child on that path.

## Finding prioritaire / Priority finding

### `CODE-R2-F-001` — P1 gate-blocking, test-only — attente fragile et absence de cleanup

Dans `scripts/test-os-package-maintenance.py:111-123`, la boucle attend seulement
50 × 20 ms, puis l'assertion de ligne 118 s'exécute avant `killpg()` et
`communicate()`. Lors de la première suite exacte, `upgrade` n'était pas visible
dans cette fenêtre : le test a échoué, le processus `97814` est resté actif
jusqu'à la fin naturelle du `sleep 10`, et Python a émis des `ResourceWarning`
pour le subprocess et les deux pipes non refermés. Résultat observé : **7 PASS,
1 FAIL**, environ 10,7 s. Trois relances ciblées ont ensuite réussi en
0,60–0,70 s chacune; elles montrent que le défaut est intermittent, pas absent.

Action requise : employer une échéance monotone avec marge suffisante pour
attendre la borne `upgrade`, et garantir dans un `try/finally` que tout chemin
d'échec termine le groupe de processus puis appelle `communicate()`/referme les
pipes. L'oracle doit toujours conserver les assertions rc 143, diagnostic,
préfixe exact, environnement et absence de reboot. Aucune modification des
trois fichiers runtime n'est nécessaire.

## Delta et preuves réutilisées / Delta and reused evidence

- `git diff --exit-code 6abfc719..1d0f0339 -- deploy/linux/maintenance/*
  docs/operations/OS-PACKAGE-MAINTENANCE.md` : **PASS**, aucun octet runtime ou
  guide modifié. Les conclusions R1 sur dimanche 03:00 local, unité statique
  oneshot, échec fermé avant reboot, permissions installées et absence de delta
  API/updater restent valides.
- `git diff --check 6abfc719..1d0f0339` : **PASS**; `bash -n` et parsing AST du
  test : **PASS**.
- Les nouveaux oracles de succès, d'échecs et d'environnement sont plus précis
  et lisibles. Le seul constat ouvert concerne leur cycle de processus sur
  timeout.
- Les limites R1 restent inchangées : systemd/cron/apt/reboot réels et état
  post-boot sont `NOT_RUN` dans cette revue macOS. Le gate de permissions
  `root:root` (`0750` script, `0644` unité/cron) reste à vérifier avant copie du
  cron; aucune installation ni exécution distante n'a été réalisée ici.

Ce rapport porte sur le SHA exact `1d0f0339`; un correctif test-only le rendra
historique et exigera une nouvelle réévaluation accélérée.
