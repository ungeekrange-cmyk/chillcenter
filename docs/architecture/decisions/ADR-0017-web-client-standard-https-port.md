# ADR-0017 — Client web sur le port HTTPS standard (443 redirigé vers 8443)

- Status : `PROPOSED` ; date : `2026-10-15`. Passe à `ACCEPTED` après revue indépendante des octets déployés et validation utilisateur.
- Decision Owner / Author Instance : `/root` ; Reviewer Instance : à désigner (≠ auteur).
- Branche : `codex/web-pwa`.
- Supersedes / Superseded By : `NONE` / `NONE`. Complète ADR-0014 (client web) et ADR-0005 (HTTPS) sans les modifier.
- Related Items : [WEB-CLIENT](../WEB-CLIENT.md), `docs/deployment/WEB-CLIENT-PI.md`.

## Contexte

Demande utilisateur (15 octobre 2026) : le client web doit être joignable sur le port « normal », `https://chillcenter.local`, pas sur `:8443`. Le serveur écoute sur `chillcenter.local:8443` ; les clients Mac enregistrés pointent sur ce port et ne doivent pas être touchés.

Contraintes lues dans le code : `check_browser_origin_and_host` (`crates/chill-server/src/api.rs`) refuse tout `Host` dont le nom n'est pas dans `tls_names` **ou dont le port (défaut 443 si absent) diffère du port `listen`**. Un navigateur qui ouvre `https://chillcenter.local` envoie `Host: chillcenter.local` sans port ; une redirection de port réseau 443 → 8443 produirait donc `HOST_REFUSED`. L'unité systemd retire toute capacité (`CapabilityBoundingSet=`, `AmbientCapabilities=` vides) : le service ne peut pas lier un port < 1024.

## Options examinées

| Option | Pour | Contre |
| --- | --- | --- |
| A. Second listener 443 dans le serveur | URL propre, tout dans le binaire | nouvelle capacité `CAP_NET_BIND_SERVICE` pour le service, champ de configuration, deux serveurs TLS à tenir (renouvellement, limites de connexions), plus de surface à relire |
| B. Déplacer le serveur sur 443 seul | trivial | casse les clients Mac et tout profil enregistré sur `:8443` |
| **C. Redirection de port réseau 443 → 8443 sur le Pi + contrôle `Host` qui accepte aussi le port par défaut** | le service ne gagne aucun droit ni aucun port ; configuration et unité du service inchangées ; réversible en supprimant une règle | une règle pare-feu à poser et à persister sur le Pi (hors du service) ; le port par défaut devient acceptable dans `Host` |
| D. Redirection HTTP 301 depuis un listener 443 | l'URL finale garde `:8443` | exige quand même un listener 443 : mêmes contraintes que A |

## Décision

Option **C**.

1. **Rust** : le contrôle `Host` accepte le port de `listen` **ou** 443. Le nom reste obligatoirement l'un de `tls_names` (garde anti-rebinding DNS conservée). Cas couverts par test : `localhost`, `localhost:443`, `localhost:8443` acceptés ; `localhost:9443`, `localhost:80`, `attacker.example`, `attacker.example:443` refusés ; une `Origin` sans port ne correspond pas à un `Host` avec port.
2. **Réseau (Pi)** : une règle `redirect` (nftables, hook prerouting) limitée à l'adresse du Pi, TCP 443 → 8443, dans une table nftables dédiée, installée par un script d'opérateur séparé, relu, après un relevé **en lecture seule** du pare-feu existant. Ce script n'est pas le script V8 de déploiement (dont les octets ont été relus et ne changent pas).
3. Le port 80 n'est pas ouvert : pas de redirection HTTP, `http://` ne répond pas.

## Conséquences

- Le client web : `https://chillcenter.local` ; le client Mac : inchangé sur `:8443`.
- Le trafic local du Pi vers lui-même (loopback) n'est pas redirigé : les sondes locales restent sur `:8443`.
- Le certificat reste auto-signé : l'avertissement du navigateur et les limites du Service Worker décrites dans WEB-CLIENT demeurent.
- Retour arrière : supprimer la table nftables ; le serveur continue de répondre sur `:8443`. Un binaire plus ancien refuserait `Host` sans port, donc le web sur 443 cesserait de fonctionner mais rien d'autre.
- Risques résiduels relevés par la revue n° 5 (voir runbook, étape 8) : adresse primaire de l'interface, `flush ruleset` du fichier nftables par défaut si `nftables.service` est activé plus tard, état `failed` de l'unité si la table a déjà disparu.
- Non prouvé tant que non exécuté sur le Pi : présence et état de nftables / iptables, règles existantes, persistance au redémarrage, comportement avec un vrai navigateur.
