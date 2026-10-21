# Operational Delta Review — OS package maintenance R3

- **Review Type / Reviewer Instance:** code / `/root/os_maintenance_code`
- **Parent Reviewed SHA / Verdict:**
  `1d0f03395dc600a2c289701cf0e42928edf4d284` / `CHANGES_REQUESTED`,
  [OS-PACKAGES-code-r2.md](OS-PACKAGES-code-r2.md)
- **New Source SHA:** `1811c9342aef13d451105f144b73137bc595b871`
- **Source Delta Scope:** test harness only in
  `scripts/test-os-package-maintenance.py`; runtime files and operations guide
  are byte-identical to the approved `6abfc719` source. README links, historical
  R2 reports and the prepared delivery plan are non-runtime evidence carriers.
- **Test Evidence Validity:** R1 runtime conclusions remain valid; R2 test
  evidence is superseded by the fresh R3 run below.
- **Current Verdict:** `APPROVED`
- **Limitations / Required Follow-up:** distinct current test/security gates and
  deployment-time Debian checks remain required; no code finding is open.

FR : le constat test-only `CODE-R2-F-001` est corrigé sans changement runtime.
EN: the test-only `CODE-R2-F-001` finding is closed with no runtime change.

## Delta review / Revue du delta

- `_term_fixture` utilise désormais une échéance monotone de 10 secondes,
  surveille aussi la terminaison précoce avec `poll()`, puis conserve les
  assertions de readiness, code 143, diagnostic, ordre exact, environnement et
  absence de reboot.
- Le `finally` termine le groupe encore actif, attend le processus, utilise
  SIGKILL après timeout borné, reap aussi un processus déjà terminé et ferme les
  deux pipes. Le chemin d'échec qui avait laissé un enfant/pipes ouverts en R2
  est donc couvert.
- `test_stalled_term_readiness_is_reaped` force une readiness bloquée, exige le
  timeout attendu et vérifie processus terminé et pipes fermés. Aucun nettoyage
  n'est laissé au hasard ou à la fin naturelle du faux `sleep`.
- Les changements d'index et le plan `REL-OS-PACKAGES-20261021.md` ne modifient
  ni l'application ni l'exécution. Le plan conserve cron installé en dernier,
  aucun start/apt/reboot immédiat, les modes root et l'absence d'activation du
  helper de mise à jour Chill Center.

## Evidence and reuse / Preuves et réutilisation

- `git diff --exit-code 6abfc719..1811c934 --` sur les trois fichiers
  `deploy/linux/maintenance/` et le guide d'exploitation : **PASS**. Les hashes,
  dimanche 03:00 local, unité statique oneshot, échec fermé avant reboot,
  permissions installées et absence de delta API/updater restent ceux revus R1.
- `python3 -B scripts/test-os-package-maintenance.py` : **9/9 PASS**, 0 échec,
  skip ou retry, 3,79 s sur macOS reviewer.
- `bash -n` du script runtime, parsing AST du test et `git diff --check
  6abfc719..1811c934` : **PASS**.
- Les exécutions builder/Main signalées (9/9 frais et répétitions TERM) sont
  cohérentes, mais l'approbation ci-dessus s'appuie aussi sur le run indépendant
  de ce reviewer.

`systemd-analyze`, cron Debian 13, dépôts/transactions APT réels, concurrence
avec `unattended-upgrades`, reboot et état post-boot restent **NOT_RUN** ici.
Le déploiement doit vérifier `root:root` (`0750` script, `0644` unité/cron), les
hashes et l'unité inactive sans démarrer le service. La première exécution
planifiée ne constitue pas une preuve anticipée. Toute modification source après
`1811c9342aef13d451105f144b73137bc595b871` rend ce verdict caduc.
