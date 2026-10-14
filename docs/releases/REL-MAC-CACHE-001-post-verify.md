# Additif de vérification indépendante — cache client 624f244

- **Client revu** : `624f2447b5c0ede6b4a053ea3fa7b82db85c0121` (revues code, tests, ADR et sécurité référencées dans `docs/reviews/chillcenter-cache*624f244.md`).
- **Bundle contrôlé** : `Chill Center-624f244.app`, 75 MiB, sans remplacement du bundle précédent.
- **Serveur inchangé** : source `126da0af167ee62588d92d90297f2dfc51962676`, binaire Pi `9a17…36a6`, service R7 + drop-in WorkingDirectory déjà vérifié.
- **Verdict** : `APPROVED_SCOPED` pour l’intégrité du bundle et l’état runtime serveur observés; les performances de lecture triple restent `NOT_RUN` / en cours.

| Contrôle | Observation | Résultat |
| :--- | :--- | :--- |
| Exécutable client | SHA-256 `aa32c0958bf9b818a7f635bf1bf65e90dc9099ab24b4275d5ec7a6392733bd9d` | PASS |
| libmpv embarquée | SHA-256 `a548453541616ee85970fd31da1aeafd7c7bf501a552e63502f6d69abbac31d7` | PASS |
| Signature bundle | `codesign --verify --deep --strict` : valide sur disque et respecte la designated requirement | PASS |
| Chargeur exécutable | dépendances macOS système inspectées par `otool -L`; aucune erreur de chargement rapportée par cette inspection statique | PASS_SCOPED |
| Serveur Pi | `active/running`, PID 3016, `NRestarts=0`; écoute uniquement `192.168.1.42:8443` | PASS |
| NAS | montage CIFS `//192.168.1.92/Video` toujours `ro,nosuid,nodev,noexec` | PASS |
| TLS Pi | certificat public conserve SAN `192.168.1.42`, validité 2026-10-14 → 2028-04-14 | PASS |
| Transcodage côté serveur | aucun processus `ffmpeg`, `ffprobe`, HandBrake ou VLC observé | PASS |

## Limites obligatoires

- Le scan de 2 496 éléments est signalé par l’intégrateur; il n’est pas une nouvelle mesure indépendante dans cet additif.
- Lors de ce contrôle, le serveur affichait RSS 24.6 MiB et CPU 13.2 %, cohérents avec une activité en cours; ce n’est pas une baseline de repos ni une preuve de performance.
- L’essai de trois flux réels (Dune 1 1920×768, Dune 2 1920×800 et un flux 720p) est encore en cours. Le buffering important observé sur Dune 2 interdit toute conclusion `PASS` sur 3 flux ou sur l’objectif de 30 minutes.
- Aucun redémarrage, écriture, lecture de clé privée, lecture d’identifiants NAS ou modification de configuration n’a été effectuée par ce vérificateur.
