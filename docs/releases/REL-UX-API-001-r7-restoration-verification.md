# Vérification indépendante — restauration R7 et sauvegarde privée

- **Mode** : `DEV_STANDARD`, `INTEGRATION_AND_RELEASE`
- **Verifier** : `/root/ux_api_post_verify`, distinct du deployer
- **Entrée opérateur** : `/private/tmp/chillcenter-ux-pi-deployment-authorized.md`
- **Cible** : Pi DEV `192.168.1.42`, après tentative autorisée interrompue avant remplacement
- **Revue UTC** : `2026-10-14T18:51:55Z`
- **Verdict du périmètre restauration/backup-metadata** : **APPROVED**
- **Déploiement source `9277ac4d`** : **NOT_DEPLOYED / non certifié par ce rapport**

Ce verdict couvre uniquement le retour effectif à l'ancien service R7 et la structure/cohérence SQLite de la sauvegarde privée. Il ne prouve ni l'installation du nouveau binaire, ni l'API datée, ni la connexion du nouveau client.

## Restauration de l'ancien service

Contrôles effectués en lecture seule via le master SSH utilisateur existant, laissé ouvert.

| Contrôle | Attendu | Observé | Résultat |
| :--- | :--- | :--- | :--- |
| Service | ancien R7 actif après reprise | `active/running`, PID `4980`, démarrage `2026-10-14T18:45:50Z` | PASS |
| Stabilité | PID stable, aucun restart | PID `4980` puis `4980`, `NRestarts=0` aux deux lectures | PASS |
| Binaire installé | ancien hash R7 | fichier installé et `/proc/4980/exe` = `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6` | PASS |
| Identité processus | non-root, groupe NAS approuvé | UID `997`, GID `983`, groupes `983 984` | PASS |
| Confinement | politique antérieure inchangée | `NoNewPrivileges=yes`, capabilities vides, `ProtectSystem=strict`, `ProtectHome=yes`, `PrivateTmp=yes`, `PrivateDevices=yes` | PASS |
| Chemins systemd | état local RW, NAS RO dans l'unité | `ReadWritePaths=/var/lib/chillcenter`, `ReadOnlyPaths=/srv/volumes/video` | PASS |
| Smoke runtime | ancien endpoint, TLS strict | `curl --cacert` avec certificat public obtenu par SSH, sans `-k`; `/api/v1/server` retourne `api_version=1` | PASS ancien R7 |

Aucune route datée, aucun catalogue, profil, bail, scan ou Range n'a été appelé.

## Sauvegarde privée

Racine examinée : `/var/lib/chillcenter/data/backups/ba950d51-fc88-49ee-b801-4641fac531ff`.

| Contrôle | Observé | Résultat |
| :--- | :--- | :--- |
| Ancestry privée | data root `0700`, parent `backups` `0700`, racine UUID `0700`; UID `997`, GID `983` | PASS |
| Publication | `manifest.json` est postérieur à toutes les entrées déclarées, conforme au code qui publie le manifeste en dernier | PASS |
| Manifeste | format `1`, schéma `3`, data root attendu, 5 entrées uniques, formes SHA-256 valides | PASS |
| Complétude structurelle | catalogue et configuration déclarés/présents, tailles déclarées exactes, 6 fichiers réels = 5 entrées + manifeste, aucun symlink | PASS |
| Modes manifest/config | fichiers réguliers `0600`, UID `997`, GID `983` | PASS |
| Mode catalogue | fichier régulier `0644`, UID `997`, GID `983` | NOTE, voir ci-dessous |
| SQLite | ouverture `mode=ro&immutable=1`, `PRAGMA query_only=ON`, `PRAGMA quick_check` | PASS (`ok`) |

### Note de portée sur le mode du catalogue

`catalog.sqlite` n'est pas `0600` mais `0644`. La confidentialité effective reste assurée par les trois ancêtres `0700`, appartenant au compte de service, et aucun symlink n'est présent. Le code de restauration impose la racine générée privée avant de valider le manifeste. Cet écart de mode feuille est consigné, non modifié, et ne bloque pas le verdict borné de confidentialité/cohérence de cette sauvegarde.

Les empreintes de contenu des entrées TLS n'ont pas été recalculées par le verifier afin de ne pas lire la clé privée sauvegardée. Aucun secret, certificat privé, nom de profil, note, contenu de profil ou contenu du catalogue n'a été affiché ou copié. `quick_check=ok` prouve la cohérence SQLite de la copie, pas une restauration destructive exécutée.

## Disposition

- L'ancien service est restauré et stable ; aucune autre action corrective n'est requise sur cette restauration.
- La sauvegarde peut être conservée comme preuve privée de la tentative. Ce rapport n'autorise pas sa restauration ni sa copie.
- Le parseur corrigé et la seconde tentative relèvent du deployer et d'une revue distincte. Toute prochaine preuve doit repartir du fait que `9277ac4d` n'est toujours pas actif.
- Après un futur signal `DEPLOYED`, effectuer une nouvelle vérification post-déploiement du binaire actif, PID/restarts, confinement, TLS strict, API datée/OpenAPI et connexion client contrôlée.
