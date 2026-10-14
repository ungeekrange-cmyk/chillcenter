# Vérification et exploitation prévues

État réel Pi/NAS du14octobre2026 : V-01installationARM64/TLS/confinement PASSscoped après corrections opérateur ; scan2496 et lectures10s AVI/MKV/MP4 PASS, SRT externe et dimensionsHD vérifiés côtéMac. V-02charge30min deux puis trois lecteurs en cours, pas déjàPASS. [Release et revues indépendantes](releases/REL-PI-001.md), [médias et limites](releases/REL-PI-001-media-evidence.md). Les étatsNOT_RUN des checkpoints ci-dessous ne prévalent pas sur les preuves nouvelles, et les fixtures ne prouvent toujours pas les pannes/son/veille physiques.

Mise à jour14 octobre2026 : la mention NOT_RUN ci-dessous était l'état de l'étape2 et reste historique. Exécutions réelles et portées par story dans [COMPLETION-EVIDENCE](implementation/COMPLETION-EVIDENCE.md) :255 testsRust,24Node,7packaging, fixturesTauri etbundlefinal126da0a. Pi/Linux/NAS désormaisprêts d'après l'utilisateur, produitnoninstallé. V-01/02 et partiesmatériellesV-03–12NOT_RUN (ARM/systemd/2–3fluxNASWi-Fi, sonhumain/veilleOS/fournisseurs). Ne pas convertir une règle unitaire en benchmark physique. Runbooks actuels Linux/backup/restore dans docs/implementation.

Révision `CC-DOC-001`. Statut de tous les tests applicatifs ci-dessous : `NOT_RUN`. À transformer en critères d'acceptation lors des user stories, puis en preuves liées à un commit et un artefact exacts. Aucun benchmark n'est présenté comme réussi.

## Parcours critiques

| Référence | Preuve attendue |
| --- | --- |
| V-01 | Serveur Linux ARM64 lancé sur Pi 3 B, sans dépendance Tauri/macOS et sans encodeur vidéo actif. |
| V-02 | Deux puis trois clients lisent des fichiers représentatifs 1080p/720p ; débit réel, buffering, RSS, CPU, mémoire système et température enregistrés sur Wi-Fi. Test de 30 min avec seeks et pistes différentes. Refaire sur réseau réel avant promesse de capacité. |
| V-03 | MKV/MP4/AVI ; codecs H.264/HEVC/autres observés ; audio anglais manquant ; sous-titres intégrés/externes ; plein écran, vitesses, fallback logiciel mesuré. Enabler stream_cb : ABI/version API ≥1.106, seek initial/ultérieur, annulation pendant read/seek bloqué, fermeture/destruction concurrente, absence de deadlock et fuite. Aucun « format supporté » sans fixture. |
| V-04 | Deux appareils du même profil : transfert ACK/concurrent/déconnecté/réveil/tampon ; renouvellement ancien refusé pendant TRANSFERRING, deadline figée, ACK unique avant deadline puis refusé. Nouvelle génération liée au média suivant et rejet d'événements/flux du précédent. Aucun vieux paquet ne réécrit la reprise ; profils différents indépendants. |
| V-05 | 10 s effectives dans les derniers 10 % : seuil exact, seek seul, pause/buffering, ×0,5/×2, frontière 90 %, durée inconnue, cumul entre sessions et réinitialisation manuelle. |
| V-06 | Playlist/saga/série : contexte, nouvelle saison, compte à rebours 0/5/10/20/30/désactivé, annulation, disparition du prochain fichier et modification concurrente. |
| V-07 | Import/export aller-retour, affectation Cinéfolio à un profil, import répété, conflit, archive malformée/oversize/traversée, crash avant/après commit ; source intacte. |
| V-08 | Racine NAS déplacée, fichier renommé, ambiguïté d'identité, NAS déconnecté pendant scan/lecture ; aucune purge ni fusion silencieuse. |
| V-09 | Première confiance TLS, renouvellement annuel même SPKI, client absent reconnecté sans accès HTTP préalable, certificat nouveau expiré refusé, SPKI changée avant expiration (confirmation) et après expiration (fallback), SAN incorrect/signatures invalides, remplacement urgent de clé. Même politique API/vidéo/sous-titres. Horloge simulée, pas d'attente d'un an. |
| V-10 | Notes/vus/playlists isolés par profil ; suppression confirmée ; correction métadonnée commune concurrente ; IA du profil précédent refusée. |
| V-11 | Apparence système/clair/sombre ; toutes saisons et limites d'événements ; manuel/désactivé ; réveil après changement de date ; clavier et contraste. |
| V-12 | Snapshot de SQLite en activité, restauration contrôlée, interruption systemd, redémarrage invalidant les baux, migration et rollback avec base sauvegardée. |

## Ordre d'exécution futur

Vérifier d'abord l'enabler lecteur natif + transport HTTPS, puis le squelette Linux/SQLite ; unitaires de règles et contrats ; tests d'intégration API/Range/transferts ; parcours réels Mac/Pi/NAS. Build/typecheck/lint pertinents et tests UI hérités après adaptation. Les mocks servent aux contrats, pas à prouver le Wi-Fi, le rendu natif ou la rotation TLS réelle.

## Runbook minimal futur

1. Installer Linux Lite 64 bits ; configurer horloge et montage NAS lecture seule. Préparer compte système, disque local et configuration de l'adresse LAN.
2. Installer le binaire versionné et l'unité systemd. Démarrer, lire les diagnostics ; saisir le token TMDB dans Administration si souhaité.
3. Connecter le Mac à l'adresse ; approuver l'empreinte initiale ; créer/choisir un profil. Le Mac ne reçoit aucun accès SMB/NFS.
4. Ajouter la racine côté serveur, scanner ; lancer un import facultatif avec aperçu et profil de destination.
5. Sauvegarder SQLite via un snapshot cohérent, affiches et configuration ; secrets sauvegardés séparément avec permissions restreintes. Ne pas copier seulement le `.db` pendant que WAL est actif.
6. Mise à jour : pause contrôlée des lectures si migration, sauvegarde préalable, binaire immutable, contrôle de santé et smoke. Si schéma incompatible avec l'ancien binaire, restaurer la sauvegarde pré-migration avant retour ; documenter les données créées depuis qui ne seront pas reprises.

Aucune de ces opérations d'installation ou de livraison n'est exécutée pendant l'étape 2. Pas de coût cloud nouveau ; consommation électrique, capacité de disque et abonnement IA éventuel restent ceux de l'installation du foyer.
