# REL-PI-V0.7 — déploiement existant / legacy deployment

## État courant / Current state — 20 October 2026

**`VERIFIED / APPROVED_SCOPED`.** La seconde tentative a répondu
`deployment=PASS`, après une nouvelle sauvegarde privée. La
[vérification indépendante finale](REL-PI-V0.7-verification.md) est
`APPROVED_SCOPED`, dernier relevé à `2026-10-20T20:37:32Z`.

Serveur actif stable, zéro redémarrage inattendu et zéro avertissement/erreur.
Les 29 tables sémantiques, réglages et TLS sont identiques à la référence ;
catalogue complet de 2 496 vidéos sans doublon. Les 99 fichiers installés sont
validés : 97 servis identiques, deux notices Markdown de licence correctement
non servies par la liste MIME fermée. HTTPS strict sur 443/8443, API datée et
alias v1, filtres, OpenAPI, en-têtes, PWA et thèmes sont vérifiés. Aucun test
physique Safari ni nouvelle lecture vidéo n'est revendiqué.

- Paquets publiés inchangés : serveur et web `0.7.0`, web build `a1c12ba4a582`.
- Opérateur corrigé/revu : `815b523a383e3d0a61e1b116f27495e1ca2584d0`, SHA-256
  `4dd8451281a6a3037861fab15eccaff8623273b558ae167dadbad51e02ed6cfb`.
- Revue indépendante [APPROVED](REL-PI-V0.7-operator-review.md), sept tests
  ciblés PASS ; nettoyage réel de descendants ignorant TERM vérifié sur Linux,
  sans fichier de test ni interruption du service.
- Tentative finale commencée le `2026-10-20T20:26:34Z`, terminée avec succès
  avant `20:29:55Z`. Aucun paquet reconstruit, resigné ou republié.
- Sauvegarde cohérente :
  `/var/lib/chillcenter/data/backups/4cb8e869-7990-4e35-8311-b2ded8d4bd74`.
- Versions précédentes :
  `/root/chillcenter-rollback/chill-server-20261020T202641Z` et
  `/root/chillcenter-rollback/web-20261020T202641Z`.
- Référence logique, WAL inclus : **2 496 vidéos**, schéma 3 et empreintes
  sémantiques de 29 tables ; le premier relevé sans WAL de 2 497 vidéos n'était
  pas l'état logique courant et ne sert pas de comparaison.

Le premier essai a été restauré, pas déclaré réussi. La vérification indépendante
a confirmé l'ancien binaire et web exacts, TLS/réglages identiques, 29 tables
sémantiquement inchangées et service stable. Sa sauvegarde privée
`6554e936-1886-4c55-b80a-b33d1735112f` est conservée : 1 924 entrées validées et
manifest, base cohérente, zéro différence d'empreinte. Le garde corrigé compare
le contenu effectif de l'unité et ses propriétés statiques, plus les réglages,
sans confondre PID et configuration. Voir [BUG0118](../debug/BUG-0118-pi-unit-runtime-fingerprint.md).

Minisign étant absent du Pi, les signatures sont vérifiées localement avec la
clé publique épinglée ; les manifestes, signatures, clé, charges et opérateur
exacts sont ensuite revalidés sur le Pi. Aucune dépendance système n'a été
installée. Aucune clé privée de signature n'est lue ; les secrets TLS sauvegardés
et vérifiés restent sur le Pi. Ni helper privilégié, ni timer ni politique de
mise à jour automatique ne sont installés ou activés dans ce lot afin de
préserver la configuration existante.

Écart de préflight consigné : un fichier temporaire d'URL sous `/run` a été créé
puis immédiatement supprimé pendant un contrôle prévu en lecture seule. Aucun
service, réglage, certificat ou montage n'a été changé par cet écart ; il n'est
pas requalifié en opération strictement sans écriture.

The second authorized attempt returned `deployment=PASS` using the unchanged
published server/web 0.7.0 bytes after a fresh private offline backup. The first
attempt rolled back on a false invariant caused by runtime systemd PID/time;
independent restoration verification confirmed the exact old pair and all 29
database semantic digests. The failure and both backups are retained. The fix
hashes static effective unit configuration. Only the server binary and web tree
are replaced: existing settings, TLS, accounts/groups, units, NAS and Mac remain
outside the mutation scope. No root updater/timer or automatic-update policy is
activated. Signatures are checked locally, then exact transferred bytes on the
Pi. Separate post-deployment verification is now `APPROVED_SCOPED`: exact
binary/web bytes, both API prefixes, strict external TLS, catalogue facets,
all 29 database semantic digests, settings, backup and stable service passed.
The authorized version-status GET initialized only a private empty update lock;
it changed no policy or existing preference. No physical Safari or new
video-playback test is claimed.

## Historique de préparation — états ci-dessous remplacés par l'état courant

The preparation statuses below are historical, superseded by the current state.

**État / Status: `BLOCKED`.** Une tentative contrôlée a été rollbackée; aucun
nouveau payload ne demeure en service.

Tentative autorisée ultérieure: la bascule des nouveaux payloads a atteint ses
contrôles de disponibilité, puis le garde de persistance a déclenché un rollback
réussi. Le constat est un faux positif: `ExecStart` issu de `systemctl show`
contenait le PID et l’horodatage dynamiques du nouveau processus, malgré les
fichiers persistants inchangés. Aucun état de livraison n’est revendiqué par
cette tentative; une nouvelle sauvegarde et revue restent requises. Voir le
tracer [BUG0118](../debug/BUG-0118-pi-unit-runtime-fingerprint.md).

Ce lot est limité à un Pi déjà installé: remplacement groupé du binaire serveur
et de l’arbre web v0.7.0. Il n’exécute pas le premier installateur et ne crée ou
ne modifie ni helper/timer, unité systemd, compte/groupe, politique de mise à
jour, configuration, certificat/TLS, montage NAS ou réglage utilisateur.

Entrées candidates immuables: runtime revu
`3e174b9e6ed89e08f67e763c3b30028c15767080`; carrier publié
`3a043334c5c3c676251ab34bf8bab2e4bfb8df4f`; publication/intégration
indépendante [APPROVED](REL-V0.7-verification.md). Serveur ARM64 SHA-256
`68d526cd3f3af7cee9b07b06c62f46260656dda7cccc4c12994dee85431771e5`;
web SHA-256 `0fa1aabf20d9f84b3e2722b0af7017dd09de9503da4524140a19f593072d7b57`,
build `a1c12ba4a582`, version `0.7.0`. La seule clé manipulée est la clé
publique de production ID `18FD790B48C202F8`; aucune clé privée n’est lue.

L’opérateur `scripts/pi-v07-legacy-server-web-operator.sh` reçoit tous les
chemins et URL par variables d’environnement: aucune IP ni chemin NAS familial
n’est codé. Il refuse signature/manifest/empreinte incompatible, API non
stricte TLS, schéma SQLite différent de 3, scan ou job actif, socket client
établie, et archive web dangereuse. Il documente que l’absence du helper legacy
fait répondre `UPDATE_HELPER_NOT_INSTALLED`: c’est une limite attendue, pas un
échec à contourner.

Son interface est fermée: exactement `--self-test`, `--preflight`, `--deploy`
ou `--recover`, sans argument supplémentaire. `--recover` est l’unique reprise
explicite après une coupure non interceptable: il lit le journal durable sous la
racine rollback, réconcilie les chemins réellement présents, restaure le couple
ancien, vérifie ses hash/build, TLS et le service, puis marque le rollback
terminé. Il ne télécharge ni n’installe de nouvel artefact.

Après les gates, il fait une sauvegarde hors ligne sous l’identité du service.
La sauvegarde du binaire existant utilise son snapshot SQLite cohérent (dont le
contrat applicatif réalise `quick_check`); l’opérateur valide directement le
manifest, les empreintes, l’ensemble exact, les secrets TLS inclus et l’absence
de WAL/SHM non déclarés. Il
conserve sous une racine root `0700` l’ancien binaire, le web précédent et les
empreintes metadata/octet de config, library, certificat, build web et unité.
Le journal est fsync avant l’arrêt; une erreur interceptable remet **binaire et
web** avant redémarrage, tandis qu’un SIGKILL/power-loss exige le rerun explicite
`--recover`;
la base, la configuration et le NAS ne sont jamais restaurés automatiquement.

Avant toute mutation il faut: (1) revue indépendante du SHA exact de
l’opérateur; (2) préflight SSH lecture seule confirmant architecture ARM64,
chemins legacy, identité, certificat public, service/espace et activité nulle,
et inventoriant sans contenu les réglages persistants au-delà de
config/library/TLS;
(3) autorisation explicite du coordinateur couvrant le SHA et la cible exacte.
Une instance distincte doit ensuite vérifier la version réellement servie,
service/journaux, backup et invariants config/library/TLS/unité. Cette
préparation ne revendique ni smoke live, ni lecture NAS, ni Safari.

| Élément | État |
| --- | --- |
| Préflight SSH lecture seule | `NOT_RUN` |
| Revue indépendante opérateur | `PENDING` |
| Autorisation cible/SHA | `PENDING` |
| Staging, arrêt, sauvegarde, bascule | `NOT_RUN` |
| Vérification indépendante post-déploiement | `NOT_RUN` |
