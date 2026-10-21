# Durable legacy deployment jobs / Déploiements legacy suivis

## Français

Protocole BUG0128 pour les **prochaines opérations manuelles**, pas un
installateur initial, une nouvelle API ou l'activation du helper/timer. Aucune
version publiée n'est remplacée. Une cible réelle exige toujours autorisation,
code revu, paquets signés vérifiés, pins exacts, contrôle d'occupation et
sauvegarde validée.

### Préparation et lancement

Linux, Python3 standard, Bash et util-linux `flock` sont requis. Vérifier le
commit et les empreintes des deux scripts, puis les placer dans un dossier
root privé0700, chacun mode0700 : `pi-operator-job.py` et
`pi-v07-legacy-server-web-operator.sh`. Les parents ne peuvent être modifiables
par autrui ; `/tmp` root sticky est accepté pour les seuls tests.

Conserver les paramètres `CC_*` et les empreintes des paquets/reçus déjà exigés
par l'opérateur dans un canal privé. `CC_ROLLBACK_DIR` doit être identique au
`--root`. Ne pas publier ces paramètres dans Git ou les notes. Cet exemple ne
contient aucune adresse, clé, certificat ou configuration réelle du foyer.

Choisir **avant la connexion SSH** un UUID canonique, le conserver dans
`CHILL_JOB_ID`. Après staging/revue, dans la session root autorisée :

```sh
python3 -I /root/chillcenter-operator/pi-operator-job.py start \
  --root /root/chillcenter-rollback --job-id "$CHILL_JOB_ID" \
  --mode deploy \
  --operator /root/chillcenter-operator/pi-v07-legacy-server-web-operator.sh \
  --operator-sha256 "$CHILL_OPERATOR_SHA256"
```

`QUEUED` acquitte le lancement, **pas** le succès. Un UUID existant est refusé
sans écrasement : en cas de doute, consulter ce même travail au lieu de relancer.
Le lanceur réserve le verrou avant l'acquittement, puis détache session et flux
SSH. Superviseur, opérateur et sauvegardes héritent du même verrou noyau.

### Reconnexion et résultat

```sh
python3 -I /root/chillcenter-operator/pi-operator-job.py status \
  --root /root/chillcenter-rollback --job-id "$CHILL_JOB_ID"
```

| État | Interprétation |
| --- | --- |
| `QUEUED` / `RUNNING` | Non terminé : attendre et consulter le même identifiant. |
| `SUCCEEDED`, `exit_code:0` | Vrai reçu terminal ; vérification indépendante de santé/données encore requise. |
| `FAILED` | Code de sortie réel ; une valeur négative représente un signal. Examiner aussi `operation_busy`. |
| `SUPERVISOR_LOST` / `START_NOT_OBSERVED` | Aucun reçu fiable, code JSON `null` : ni succès ni échec démontré. |
| `operation_busy:true` | Un propriétaire conserve le verrou, éventuellement une sauvegarde orpheline. Nouvelle opération refusée avec code42. |

Les fichiers `operator-jobs/UUID/` sont privés : snapshots du code revu,
demande, `transcript.log`, `state.json`. Publication atomique/fsync ; le reçu
terminal suit le flush du journal. Ils sont conservés, sans purge automatique.
Ne jamais publier `request.json` ou les journaux bruts : paramètres privés.

### Récupération

Ne jamais supprimer `operator.lock`, `backup.lock` ou le verrou de données :
leur inode stable assure l'exclusion. Ne pas tuer des processus par simple
recherche textuelle. Le statut vérifie boot et instant de démarrage Linux, pas
un PID seul. Il n'autorise aucune terminaison ou récupération automatique.

Si le verrou reste pris, attendre l'opération bornée ou diagnostiquer son
propriétaire exact. Après quiescence, vérifier hashes installés, santé et journal.
Une bascule finalement réussie et sans journal nécessite une vérification, pas
un rollback aveugle. Si une récupération est justifiée et autorisée : même
commande avec `--mode recover` et **un nouvel UUID**, puis contrôle indépendant
de restauration. Les appels directs et l'EXIT trap de l'opérateur refusent eux
aussi de redémarrer le service pendant une sauvegarde survivante.

## English

This protocol is for **future manual operations**, not first install, a public
API or privileged helper/timer activation. Published versions remain unchanged.
Real targets still need authorization, reviewed code, verified signed payloads,
exact pins, quiet guards and a validated backup.

Stage both scripts in a root-owned0700 directory, each mode0700. Linux, Python3,
Bash and util-linux `flock` are required. Keep existing `CC_*` parameters private;
`CC_ROLLBACK_DIR` must match `--root`. Choose a canonical UUID **before SSH**, then
use the `start` and `status` commands above. A `QUEUED` reply is not success;
reusing a job UUID refuses without overwriting its evidence.

The launcher reserves a shared kernel lock before acknowledging, snapshots the
reviewed operator, and detaches from SSH IO/session. All descendants, including
backup, retain the same lock. Atomic private state and transcript record the
actual exit code. Never infer success from stdout, elapsed time, a tool-cell
result, or an intermediate installed-version observation.

`RUNNING`/`QUEUED` means wait. `SUCCEEDED` plus exit0 is the real operator receipt,
not independent health/data proof. `FAILED` retains the real code (negative for
a signal). `SUPERVISOR_LOST`/`START_NOT_OBSERVED` means uncertainty with a null
exit code, not success or proof of failure. `operation_busy:true` refuses every
new operation with42, even if only an orphaned backup survives. Linux boot/start
identity prevents PID reuse from reviving an old receipt.

Never unlink lock inodes or kill substring-selected processes. Diagnose the
exact owner; no automatic cancellation/recovery is authorized. After quiescence,
inspect current hashes, health and journal. A healthy completed swap without a
transition journal needs verification, not blind rollback. A justified authorized
recovery uses a **new UUID**, the same exclusion lock, then an independent
restoration check. Direct execution and the EXIT trap also refuse recovery over
a surviving backup. Private job files are retained; never publish raw requests
or transcripts. No household parameter belongs in this repository.

## Isolated proof / Preuves isolées

```sh
# In the disposable local Linux VM only; service/data operations are fixtures.
sudo python3 -B scripts/test-pi-v07-legacy-operator.py
sudo python3 -B scripts/test-pi-operator-job.py

# On its host: only the local VM's loopback SSH config is accepted.
python3 scripts/test-pi-operator-ssh.py --ssh-config /path/to/local-vm/ssh.config
```

Real Linux locks, signals, util-linux uid switching and SSH transport loss are
exercised; service control, TLS and payloads remain fixtures. Physical power loss,
filesystem corruption, full real-systemd deployment and Pi/NAS tests are NOT_RUN.
The SSH hook additionally refuses any guest except `colima-v07-security`.
