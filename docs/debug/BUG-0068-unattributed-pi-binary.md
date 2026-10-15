# BUG-0068 — Binaire Pi remplacé sans attribution pendant la préparation

2026-10-15, coordinateur `/root`, diagnostic borné `/root/inspect_deploy_exit`, vérification indépendante `/root/verify_catalog_release`. INVESTIGATING ; écritures Pi suspendues.

Attendu : remplacer c610 SHA `cef54cd0e8baf45f70ee0f57584001521c7dcd705d4f8ab351eb8074f524b3c3` par11aa SHA `de98776dd5d0d20c72b7770591c78375186026e283b73ae4c66fe21b82398eb0`, avec sauvegarde privée/rollback, sans toucher NAS/config/TLS.

Première procédure : retour3 sans compte rendu final, ancien hash observé lors du premier diagnostic, service inactif. Remise en service immédiate autorisée : PID9457/HTTPS strict PASS. Présence d’une sauvegarde supplémentaire93f3dee3-6297-4319-a455-a0ece8a98437 (manifest mtime15:44:44Z), non validée ni attribuée à cette release. Rollback c610 prévu absent. La preuve ne permet pas de désigner la ligne initiale retournant3.

Deuxième procédure avec garde-fou renforcé/testé : refus2 en préflight sans arrêt. Nouveau diagnostic : fichier installé ET `/proc/9457/exe` SHA `a7ab7b07633c916c358f1656aa8f154b207185d5ac306c374d2a7a52b0c7d527`, mtime15:42:44Z, provenance inconnue. Différent des deux artefacts prévus. À15:57:04Z, actif/running/NRestarts0, config/unité/SPKI inchangées. Un autre déployeur concurrent est une hypothèse plausible, non confirmée. Ne pas attribuer ce hash à un commit ni l’écraser sans coordination.

Ledger : H1 négation `! is-active` sous errexit causant seule la panne REFUTED par mock ; H2 reprise silencieuse tolérant un échec de `systemctl start` CONFIRMED comme faiblesse latente, pas cause prouvée ici. Même logique dans le script historique. Garde-fou opérateur : reprise sans errexit, contrôle actif+HTTPS borné, échec explicite70, trace phase/ligne/code sans données. Deux mocks PASS : échec restart explicite ; reprise HTTPS conserve le code initial. Scripts conservés dans `/private/tmp/chillcenter-catalog-release.HPoPXM`, première version préservée.

Pas de correction spéculative du serveur, restauration de données, scan/TMDB/IA/lecture vidéo. Prochaine action : clarifier la publication inconnue puis choisir une seule livraison et son retour arrière. Voir REL-LIBRARY-UX-SYNC-001-verification.

## Attribution ultérieure — 15 octobre, 21:11Z

L'utilisateur a confirmé une autre mise à jour puis sa fin. Les documents du worktree `codex/web-pwa` attribuent a7ab au chantier web et consignent son remplacement par ffcc. Vérification SSH en lecture seule : service actif/PID1634/NRestarts0, binaire installé et `/proc/1634/exe` SHA `ffcc634923f637331cac25fec1e5283b062ef5e2943778bcab59460b6269b0f7`, identique à l'artefact local de ce worktree ; HTTP8080 et HTTPS8443 actifs. Attribution RESOLVED ; ce n'est pas la livraison du serveur11aa. La vérification défavorable de cette ancienne livraison reste inchangée.

Aucun redéploiement effectué pour ce contrôle. Le serveur web reste partiellement compatible avec Mac11aa529 ; intégration locale seulement, voir [rapport](../implementation/MAC-WEB-COMPATIBILITY-20261015.md). Ne pas restaurer un serveur antérieur ignorant `http_listen` ou dépourvu du client web.
