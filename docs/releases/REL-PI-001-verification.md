# Vérification indépendante de release — REL-PI-001

- **Environnement** : foyer LAN, `admin@chillcenter.local`, Raspberry Pi 3 B+ / Debian 13.7 ARM64.
- **Source fonctionnelle** : `126da0af167ee62588d92d90297f2dfc51962676`.
- **Composition R7 déployée** : archive `ed485f99e14ab8953b7d007959ab9fc35a11a006d4d989260a703fe9be6ebee2`; binaire `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6`; unité fournisseur `76f7…`; drop-in WorkingDirectory `6bad2f8c529705b246253bb8d593d7ae58155b2eb23bacac7afb404e01a5bf1a`.
- **Gate d’intégration** : `G-INTEGRATION-PI-001-working-directory`, APPROVED.
- **Déployeur** : `/root/pi_deployer`.
- **Vérificateur** : `/root/pi_release_verifier`, indépendant du builder, déployeur et revues amont.
- **Vérifié à** : 2026-10-14 11:07–11:12 UTC.
- **Verdict** : `APPROVED_SCOPED` pour l’installation et les contrôles runtime explicitement listés ci-dessous.

## Preuves de vérification

| Contrôle | Attendu | Observé | Résultat |
| :--- | :--- | :--- | :--- |
| Identité déployée | source/artéfact prévu | SHA-256 du binaire installé = `9a17…36a6`; API déclare version `0.1.0`, API v1 | PASS |
| Service | actif, sans redémarrage | `active/running`, PID 3016, `NRestarts=0` pendant la fenêtre de mesure | PASS |
| Compte et permissions | compte non privilégié, accès NAS limité au groupe approuvé | `chillcenter` uid 997/gid 983, shell `nologin`, groupes 983 et `marvin-admin` 984; config `0640 root:chillcenter`; état/TLS `0700`; clé privée vérifiée par métadonnée uniquement `0600 chillcenter:chillcenter` | PASS |
| Confinement | écriture état local seulement, NAS lecture seule | `ProtectSystem=strict`, `ProtectHome`, `PrivateTmp`, `PrivateDevices`, `NoNewPrivileges`, capabilities vides, `ReadWritePaths=/var/lib/chillcenter`, `ReadOnlyPaths=/srv/volumes/video`; CIFS `//nas.local/Video` effectivement `ro,nosuid,nodev,noexec` | PASS |
| Réseau et coexistence | service limité au LAN, SSH/Cockpit conservés | Chill Center écoute seulement `chillcenter.local:8443`; Cockpit reste `chillcenter.local:9090`; SSH reste sur 22 | PASS |
| TLS | certificat LAN valide, contrôle strict | certificat public auto-signé CN Chill Center, SAN IP `chillcenter.local`, valide du 2026-10-14 au 2028-04-14; curl avec `--cacert`, jamais `-k`, réussit sur `/api/v1/server` et `/api/v1/profiles` | PASS |
| Bootstrap contrôlé | profils QA isolés disponibles | API HTTPS retourne trois profils `QA Pi 9be1667e {1,2,3}`; aucun profil personnel n’est utilisé dans cette vérification | PASS |
| Journalisation | logs de démarrage sans fuite apparente | journal depuis le dernier démarrage : une seule ligne de démarrage; aucun chemin média, jeton ou identifiant observé | PASS |
| Consommation au repos | stable et adaptée au Pi | sur 70 s : RSS 11.1 MiB, CPU 0.7% puis 0.4%, mémoire disponible 664.6 → 661.7 MiB, swap 0, température 46.16 °C, aucun `ffmpeg`/`ffprobe` | PASS |

## Limites et critères non exécutés

- Le scan NAS réel, la requête HTTPS Range sur une vidéo, le décodage Mac/libmpv, pistes audio/sous-titres, reprise, lecture simultanée et la charge 30 minutes ne sont **pas** validés par ce rapport : `NOT_RUN` ici, à couvrir par le probe contrôlé séparé.
- Les 33 user stories / 133 critères d’acceptation ne sont pas déclarés tous validés sans preuves propres à chaque parcours.
- La route `/health` n’est pas une route du contrat serveur et retourne 404; les routes documentées `/api/v1/server` et `/api/v1/profiles` ont été utilisées pour le smoke HTTPS.

## Retour arrière et conclusion

- Aucun trigger de rollback n’est atteint dans ce périmètre. Si un échec critique apparaît dans les tests de lecture, le déployeur doit seulement arrêter/désactiver `chillcenter.service` et préserver binaire, configuration, état et NAS; aucun service tiers ni le montage NAS ne doit être modifié.
- L’échec antérieur d’installation lié à `WorkingDirectory` a été restauré par un vérificateur distinct avant le candidat R7; le service R7 actif ci-dessus est la première installation retenue.
- **Post-deployment gate** : ouvert pour ce périmètre runtime `APPROVED_SCOPED`; les tests médias/performance restants conservent l’état `NOT_RUN`.
