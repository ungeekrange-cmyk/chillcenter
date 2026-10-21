# Chill Center 0.7.1 — plan opératoire Bonjour / Pi

**État : PREPARED_ONLY — ne pas exécuter avant le hash final du nouveau binaire,
la revue sécurité distincte de ce nouvel artefact, et une connexion SSH humaine
ouverte.** L'autorisation humaine porte sur la seule exception `AF_NETLINK`, la
sauvegarde privée fraîche et la mise à jour du serveur. Elle ne couvre ni le NAS,
ni les réglages, TLS, le web, un helper/timer, ni une release de thèmes.

## Entrées épinglées et exclusions

- Binaire actuellement installé :
  `68d526cd3f3af7cee9b07b06c62f46260656dda7cccc4c12994dee85431771e5` à
  `/usr/lib/chillcenter/chill-server`.
- Le seul opérateur de swap/backup est le script déjà relu
  `scripts/pi-server-binary-operator.py`; ce plan ne le modifie pas.
- Le hash du candidat Linux était `8cf144…2491c` pour `fdb80b5`, mais il est
  maintenant **stale** après le changement du validateur de thèmes. Ne pas le
  substituer ni démarrer une opération avec ce hash.
- Config : `/etc/chillcenter/server.json`; données :
  `/var/lib/chillcenter/data`; TLS :
  `/var/lib/chillcenter/data/tls/{certificate.pem,service-key.pem}`; web
  `/usr/lib/chillcenter/web` — tous sont invariants de l'opérateur binaire.

## Préflight réel, lecture seule — 21 octobre 2026

Le script exact `pi-server-binary-operator.py` de `aba6402` (SHA-256 local
`700a08a9fdc99a3730d9bfbd77624c47f533605a6d7451d811b5fc64434373c7`) a été
envoyé seulement sur stdin via la session SSH ouverte par l'humain. En mode
`preflight`, son candidat était le binaire installé et les pins ancien/nouveau
étaient volontairement le même SHA `68d526…71e5`; ce mode ne journalise pas,
ne crée pas de sauvegarde, ne stoppe pas et ne remplace rien. Résultat :
`preflight=PASS`.

Le préflight a exercé ses contrôles de repos (service actif, API datée, scan et
jobs inactifs, aucun bail ni socket client détecté), snapshot cohérent de la
base et invariants privés config/TLS/données/web/unité. Les empreintes restent
dans le processus et n'ont pas été affichées. Baseline systemd lu séparément :
`MainPID=107443`, `NRestarts=0`, `User=chillcenter`,
`NoNewPrivileges=yes`, capabilities vides et familles exactement
`AF_INET AF_INET6 AF_UNIX`.

`chillcenter.local` ne se résout pas sur le Pi, donc une première tentative
strictement TLS a refusé avant toute action (`strict_tls_failed`). La reprise
lecture seule a utilisé l'URL HTTPS privée fournie par l'opérateur (port8443) : son IP est l'unique SAN
du certificat et l'adresse réellement écoutée. Ce n'est pas un contournement de
la vérification TLS; l'URL IP/SAN doit être conservée pour l'opérateur final.
Les routes actuelles et l'état cité ici sont une baseline, pas une preuve de
découverte Bonjour ni une autorisation de déploiement.

## Opération sérielle après les gates

1. Depuis la connexion SSH explicitement ouverte par l'humain, relever en lecture
   seule l'état du service `chillcenter`, les hashes connus et le contenu effectif
   de `RestrictAddressFamilies`. Refuser toute dérive : l'ensemble initial doit
   être exactement `AF_UNIX AF_INET AF_INET6`.
2. Copier les scripts relus et le nouveau binaire dans un espace privé, vérifier
   localement leurs SHA-256 exacts, puis exécuter `pi-bonjour-family-operator.py
   apply --rollback-dir /root/chillcenter-rollback`. Il écrit seulement
   `/etc/systemd/system/chillcenter.service.d/20-discovery-netlink.conf`, avec
   exactement `[Service]` et `RestrictAddressFamilies=AF_NETLINK`, puis fait
   `daemon-reload`. Il préserve les octets de l'unité, le jeu initial, et ses
   propres octets dans le répertoire root `0700`, journal root `0600`.
3. Confirmer que l'ensemble effectif est exactement les trois familles initiales
   plus `AF_NETLINK`, sans changement de user/capabilities. **Avant tout arrêt**,
   invoquer l'opérateur binaire révisé avec les chemins et les deux hashes exacts.
   Son invariant d'unité est donc pris après l'exception autorisée; il crée et
   valide la sauvegarde applicative cohérente avant le swap.
4. Le succès attendu exige les preuves du script binaire (TLS strict, API datée,
   `v1` 404, données/config/web/TLS inchangés) puis les contrôles spécifiques
   Bonjour : famille effective exacte, service non-root/capabilities inchangés,
   annonce et browse Mac réels. Les tests locaux et mocks ne prouvent pas ces
   derniers points.

## Échec et rollback

Si le swap échoue, laisser l'opérateur binaire exécuter sa récupération ou
exécuter son mode `recover` avec les mêmes pins jusqu'à `recovery=PASS` et un
journal `rollback_complete`. Ensuite seulement, exécuter le mode `rollback` de
l'opérateur Bonjour : il vérifie le chemin, owner/mode et SHA-256 de **sa seule**
drop-in, la supprime, fait `daemon-reload`, redémarre le serveur restauré, et
prouve le retour au jeu original sans `AF_NETLINK` **et le SHA-256 exact de
`systemctl cat` préservé avant l'exception**, avant le redémarrage. Une drop-in
modifiée, une unité/dépendance modifiée, ou une récupération/santé non prouvée
est un arrêt sûr : ne rien supprimer de plus, conserver le journal et la
maintenance, et ne pas annoncer de rollback réussi.

Le rollback ne touche jamais l'unité fournisseur, une drop-in préexistante, le
NAS, la configuration, les certificats, les données ou le web.
