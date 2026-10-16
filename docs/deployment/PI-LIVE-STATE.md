# État réel du Raspberry Pi et règles de coordination entre agents

## État courant — serveur seul f3fdac5, 15 octobre22:20UTC

Le serveur a été mis à jour sans modifier `/usr/lib/chillcenter/web` ni le worktree de développement web. Vérification indépendante APPROVED_SCOPED : [rapport](../releases/REL-SERVER-ONLY-F3FDAC5-verification.md), [release](../releases/REL-SERVER-ONLY-F3FDAC5.md). Les sections plus bas décrivent l'ancien checkpointffcc, pas le hash courant.

- Source `f3fdac503c4b845571981c1fd1bc00bd0b13ec21`, binaire installé ET processus : `d77e9e1669287a6d10f279c17f33d7f776efe7e194acc01cd542bcea33511c90`.
- Actif PID3117/NRestarts0, démarrage22:14:29UTC, stable au dernier contrôle22:20:30UTC.
- Web existant identique : arbre SHA `07868e61cd75dc36d5bf62eefd258b7409b6ed264bde726cdcf626d45b57fe2b`, contenus/métadonnées/inodes. Aucun nouveau build web installé.
- Configurationd6167cb1…, unité76f7d349…, TLS SPKI3018fb4d…, montageNAS1935a8db… inchangés ; HTTP8080/HTTPS8443 et redirections externes80/443 conservés et testés.
- Sauvegarde privée `/var/lib/chillcenter/data/backups/5dd8df08-adcb-4d09-a6c1-2eb549c0c414` :1746entrées/manifest/SQLite vérifiés. Rollbackffcc : `/root/chillcenter-rollback/chill-server-ffcc634923f6`.
- API2026-10-14/v1 conservées ; titres historiques par défaut, `selector=key` facultatif.70 chemins/89 opérations hérités présents, total76/95.
- Nouveau Mac : `artifacts/macos/Chill Center-f3fdac5.app`.11aa529 reste partiel pour les opérations opaques qui ne transmettent pas le paramètre.
- Aucun changement NAS/config/TLS/unité/pare-feu ou données personnelles. [BUG0072](../debug/BUG-0072-deployment-session-observation.md) conserve l'incident de surveillance pendant sauvegarde ; aucun code retour opérateur inventé.

Tout futur déploiement doit refaire un préflight et coordonner le binaire attendu **d77e**, sans écraser le web ou une autre livraison. Les anciens scripts épinglés surffcc doivent refuser. Cette mise à jour documentaire est sur la branche d'intégration, pas dans le worktree web en cours.

## Historique — déploiement web ffcc

> **État revérifié le 15 octobre 2026 à 20:18–20:23 UTC** (le Pi avait été injoignable d'environ 16 h à 20 h UTC, puis a redémarré : service relancé à 21:03 IST, redirection 443 et configuration conservées). Les valeurs ci-dessous sont celles **observées après** le déploiement du 20:21 UTC. Refaire un préflight en lecture seule avant toute intervention.

Document à lire **avant toute intervention sur le Pi** (livraison, script de déploiement, changement de configuration ou de pare-feu). Mis à jour le 15 octobre 2026 par l'instance qui a développé le client web (branche `codex/web-pwa`). Il sert aussi à prévenir l'agent qui développe le client Mac (branche `codex/player-streaming`) : le Pi **n'est plus dans l'état que ses scripts attendent**.

Règle générale : ce fichier distingue ce qui a été **exécuté et observé** de ce qui est **codé mais pas encore déployé**. Ne pas supposer le Pi tel qu'il est décrit ici sans refaire un préflight en lecture seule (`scripts/pi-preflight-readonly.sh`).

## 1. Ce qui est sur le Pi (exécuté le 15 octobre 2026)

| Élément | Valeur observée |
| --- | --- |
| Binaire serveur `/usr/lib/chillcenter/chill-server` | SHA-256 `ffcc634923f637331cac25fec1e5283b062ef5e2943778bcab59460b6269b0f7` (build de `codex/web-pwa` : merge de `codex/player-streaming` `a59f5e6` **plus** client web, garde d'origine, listener HTTP clair). Précédents : `a7ab7b07…` (client web, 443), `cef54cd0…` (build `c610ea9`) |
| Copie de rollback du précédent binaire | `/root/chillcenter-rollback/chill-server-a7ab7b07633c` et `chill-server-cef54cd0e8ba` (root, `0700`) ; les anciennes copies (`07358f7…`, `9277ac4…`, `cfc87de…`, `r7…`) sont conservées |
| Dossier du client web | `/usr/lib/chillcenter/web` (root:root, `0755`, lecture seule pour le service) ; le serveur le lit à côté de son binaire : **`<dossier du binaire>/web`** |
| Sauvegarde privée prise avant le déploiement | `/var/lib/chillcenter/data/backups/93f3dee3-6297-4319-a455-a0ece8a98437` |
| Configuration `/etc/chillcenter/server.json` | **modifiée le 15 octobre 20:22 UTC** : sha256 `d6167cb1624b6d5888473af76d5470102afedce25b2e29ddbaa032730cd0ceb1` (une ligne ajoutée : `http_listen` `chillcenter.local:8080`) ; `listen` `chillcenter.local:8443` ; ancienne version (`ea24f2f0a98d…`) sauvegardée dans `/root/chillcenter-rollback/server.json.ea24f2f0a98d`. Propriétaire/droits conservés (`root:chillcenter`, `0640`) |
| Unité `chillcenter.service` | **inchangée** (sha256 `76f7d34921ff344e344ba2bffa51263248ec6118dad9e15a39ef7078877d72e2`) |
| Certificat | **inchangé** (SPKI `3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df`, auto-signé, `server_id` `885f7713-b8c6-4891-a8d1-c1410d5f1537`) |
| Redirection du port 443 | table nftables **`ip chillcenter_web`** (hook prerouting, `ip daddr chillcenter.local tcp dport 443 redirect to :8443`), chargée par l'unité **`chillcenter-redirect.service`** (activée), fichiers `/etc/chillcenter/redirect-443.nft` et `/etc/systemd/system/chillcenter-redirect.service` |
| Pare-feu | nftables installé mais **`nftables.service` désactivé**, aucun autre jeu de règles ; `/etc/nftables.conf` est celui par défaut (commence par `flush ruleset`) et **n'a pas été modifié** |

| Écoute HTTP non chiffrée (ADR-0018) | `chillcenter.local:8080` (HTTP/1, web et API ; `/admin/tmdb*`, `/admin/exports*`, `/admin/imports*` refusés en clair) ; table nftables **`ip chillcenter_http`** (`ip daddr chillcenter.local tcp dport 80 redirect to :8080`), unité **`chillcenter-redirect-http.service`** (activée), fichiers `/etc/chillcenter/redirect-80.nft` et `/etc/systemd/system/chillcenter-redirect-http.service` |

Accès : `http://chillcenter.local` (client web, sans chiffrement, réseau local seulement), `https://chillcenter.local` et `https://chillcenter.local:8443` (client Mac, client web) et `https://chillcenter.local` (client web ; 443 redirigé vers 8443). Preuves : `docs/deployment/WEB-CLIENT-PI.md`, section « Exécution du 15 octobre 2026 ».

## 2. Conséquences pour tout autre déploiement sur le Pi

1. **Les scripts qui attendent l'ancien binaire s'arrêteront** (c'est voulu) : un script qui vérifie `cef54cd0…` (build `c610ea9`) refusera de continuer, puisque le binaire installé est `a7ab7b07…`. Ne pas contourner ce refus ; refaire le préflight, puis décider.
2. **Ne jamais déployer un binaire construit sans les changements du client web.** Le serveur doit contenir : le service des fichiers statiques (`crates/chill-server/src/web_static.rs`), la garde d'origine du navigateur et le contrôle `Host` qui accepte le port par défaut (`crates/chill-server/src/api.rs`), le champ optionnel `http_listen` (`crates/chill-server/src/config.rs`) et le second listener (`crates/chill-server/src/main.rs`). Un binaire sans eux : (a) ne sert plus le client web, (b) répond `HOST_REFUSED` à `https://chillcenter.local` (sans port), (c) **refuse de démarrer si la configuration contient `http_listen`** (`deny_unknown_fields`).
3. **Intégrer `codex/web-pwa` dans la branche du client Mac avant de construire la prochaine livraison** (ou l'inverse), au lieu de redéployer depuis une branche qui ne l'a pas. Fichiers communs à fusionner : `crates/chill-server/src/{api,config,main,lib}.rs` ; fichiers ajoutés : `crates/chill-server/src/web_static.rs`, `web/**`, `scripts/pi-*`, `docs/deployment/WEB-CLIENT-PI*.md`, ADR-0014 à ADR-0018 (les ADR 0012 et 0013 du client Mac ont été conservés, ceux du web ont été renumérotés).
4. **Conserver `/usr/lib/chillcenter/web`** : un remplacement du binaire par un script qui nettoie `/usr/lib/chillcenter` supprimerait le client web. Les mises à jour du client web se font avec `scripts/pi-update-web-operator.sh` (octets relus par la revue indépendante n° 4).
5. **Ne pas activer `nftables.service`, ne pas lancer `nft -f /etc/nftables.conf`, ne pas exécuter `nft flush ruleset`** : cela supprime la table `chillcenter_web` et coupe le port 443 (remède : `systemctl restart chillcenter-redirect`).
6. **Ne pas réutiliser les noms** `chillcenter-redirect*.service`, `/etc/chillcenter/redirect-*.nft`, table `chillcenter_web` (et, une fois déployés, `chillcenter_http`).
7. **Compatibilité de l'API** : aucune route ni aucun contrat du client Mac n'a été modifié ; les clients natifs (sans `Origin` ni `Sec-Fetch-Site`) sont acceptés comme avant. Les navigateurs doivent être *same-origin* (voir `docs/architecture/WEB-CLIENT.md`).

## 3. L'écoute HTTP non chiffrée : déployée le 15 octobre 2026 (voir §1)


Décision de l'utilisateur : le client web doit être joignable par les autres membres de la maison sans installer d'autorité de certification ; à défaut de domaine, **HTTP non chiffré sur le réseau local** pour le client web (ADR-0018). Côté code (branche `codex/web-pwa`, tests Rust et essai Linux verts) :

- `ServerConfig.http_listen` (optionnel, adresse LAN, port différent du HTTPS) ; sans lui, comportement strictement inchangé ;
- second listener HTTP clair servant le même client web et la même API ; sur ce listener la garde accepte `Origin` = `http://<Host>` (les navigateurs n'envoient pas `Sec-Fetch-*` aux origines HTTP non fiables) ;
- côté web, `crypto.randomUUID` (indisponible hors contexte sécurisé) remplacé par un repli.

Nouveaux fichiers de livraison (hashes dans le runbook, étapes 9 à 12) : `scripts/pi-enable-http-operator.sh`, `scripts/pi-redirect-80-operator.sh`, noms réservés `chillcenter-redirect-http.service`, `/etc/chillcenter/redirect-80.nft`, table `chillcenter_http`, sauvegarde de configuration `/root/chillcenter-rollback/server.json.<12 premiers hex>`.

Après l'étape d'activation, **le hash de `/etc/chillcenter/server.json` change** (`ea24f2f0…` → nouveau) : tout script qui épingle l'ancien hash refusera de tourner, ce qui est voulu. **Tout script qui réécrit `server.json` doit conserver `http_listen`**, sinon le port HTTP disparaît ; un binaire sans ce champ refuse la configuration (`deny_unknown_fields`). Le binaire à venir (`ffcc6349…`) remplace `a7ab7b07…` dans §1 une fois déployé.

Ce qui reste à faire sur le Pi, **chaque étape après revue indépendante et feu vert explicite de l'utilisateur** : nouveau binaire, ajout de `"http_listen": "chillcenter.local:8080"` à `/etc/chillcenter/server.json` (avec sauvegarde de l'ancien fichier), redirection du port 80 vers 8080 dans une table distincte. **Après cette étape, un retour vers un binaire sans `http_listen` exige de restaurer l'ancienne configuration.**

## 4. Pour l'agent du client Mac : ce qu'il faut adapter

- Le client Mac continue d'utiliser `https://chillcenter.local:8443` avec l'empreinte du certificat ; rien ne change pour lui tant que le certificat n'est pas remplacé.
- Ne pas supposer que `Host` doit porter le port : le serveur accepte aussi le port par défaut (443 en HTTPS, 80 en HTTP clair).
- Toute nouvelle route d'API doit rester compatible avec la garde d'origine (pas de CORS ; pas de dépendance à un en-tête que les navigateurs ne peuvent pas poser).
- Si un déploiement sur le Pi est nécessaire, **demander à l'utilisateur** d'abord ; lire ce fichier, refaire le préflight en lecture seule, mettre à jour ce fichier avec l'état réellement constaté après l'opération.
