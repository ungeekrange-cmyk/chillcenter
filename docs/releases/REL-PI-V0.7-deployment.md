# REL-PI-V0.7 — préparation legacy / legacy preparation

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
