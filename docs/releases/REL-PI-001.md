# REL-PI-001 — Première installation Linux / NAS

14 octobre 2026 · état **VERIFIED_RUNTIME_SCOPED**, service/TLS/NASRO vérifiés indépendamment, scan2496préservé, premières lectures réellesPASS ; essaislongsterminés, capacité3flux **FAILED**(Dune2<80%tempsréel), exact2Dune10min **QUALIFIED_PASS**avecstallsDune2. [Vérificationfinaledistincte](REL-PI-001-final-media-verification.md), [preuve/mesures](REL-PI-001-media-evidence.md). Les mentionsen cours/NOT_RUNplusbas sont historiques ; pasrequalificationenPASS desrisquesrestants.

ArtefactR7immuable remis dans `artifacts/linux/chillcenter-server-126da0a-pi64-r7.tar.gz` +SHA256`ed485f99e14ab8953b7d007959ab9fc35a11a006d4d989260a703fe9be6ebee2`, runtimeoverlay`10-working-directory.conf`SHA256`6bad2f8c529705b246253bb8d593d7ae58155b2eb23bacac7afb404e01a5bf1a` etreceiptJSON. NouveauclientMac624f244 et [tamponconfigurable](REL-MAC-CACHE-001.md) livrésséparément ; aucunredéploiementPi pourcesréglages. Ancienclient126conservé. TroisprofilsQA retirésaprèsfins, catalogueSQL2496etservicePID3016/NRestarts0activeconfirmés.

## Métadonnées et autorisation

- Produit/source revue : `126da0af167ee62588d92d90297f2dfc51962676`, carrier local `16cf62c8602319e7cd645161310dab1ba5a9c7ad` ; version0.1.0. Code/test/sécurité dans COMPLETION-FINAL-126da0a ; source255 testsPASS, bundleMac déjà livré.
- G-04 : approuvé avec réserves. Demande explicite de l'utilisateur : « Passe à l’installation de Chill Center sur le Pi et aux tests avec le NAS. »
- Cible vérifiée : admin@chillcenter.local, Raspberry Pi3 B+Rev1.3, Debian13.7 ARM64, RAM905MiB, glibc2.41, réseau Wi-Fi wlan0. Horloge synchronisée, Europe/Dublin. Sudo non interactif disponible.
- Service/configuration/données Chill Center absents avant installation ; ne pas remplacer un éventuel état apparu depuis sans nouvel inventaire.
- Déploiement : seule une version Linux ARM64 identifiable, sans pile Tauri/libmpv/transcodeur. Compte système non privilégié, données locales, HTTPS bindLAN explicite et montage NAS laissé intact. Pas de découverte cloud/expositionInternet.

## Pré-vérification réelle

Connexion SSH directe réussie après le refus d'agent historique ; aucune authentification contournée. CIFS automount : `/srv/volumes/video`, source `//nas.local/Video`, ro/nosuid/nodev/noexec. Permissions0750, uid999/gid984 (`marvin-admin`), fichiers0640. Lecture admin échoue, lecture sudo réussit. Choix utilisateur demandé pour permettre au compte Chill Center de lire sans root ; aucun groupe/remontage modifié jusque-là.149fichiers vidéo /513753167933octets à profondeur2 ; ce n'est pas le nombre final du scan récursif. Services Cockpit/SSH/rpcbind préexistants préservés ; le port Chill Center sera vérifié libre avant choix.

Choix reçu : « Oui, utiliser ce groupe existant » à la question explicitant le possible accès à d'autres fichiers de ce groupe. L'utilisateur autorise le nouveau compte `chillcenter` à rejoindre `marvin-admin` (gid984), pas à modifier le montage, changer les autres comptes, lire/copier les identifiants NAS ou tourner enroot. Le bind prévu estchillcenter.local:8443, libre lors de l'inventaire. Cette configuration d'installation exacte fera partie des gates du livrable Linux.

## Artefact et gates

Compilation Mac PASS en1m44, Rust1.99 verrouillé, cargo-zigbuild0.23.4/Zig0.17.0 stable vérifiés par sources primaires. Commande : `cargo zigbuild --release --locked --offline -p chill-server --target aarch64-unknown-linux-gnu.2.28` (environnement privé CARGO_HOME/RUSTUP_HOME, caches explicites). ELF64 PIE AArch64, interpréteur `/lib/ld-linux-aarch64.so.1`,13MiB, CPU générique Pi3 et glibc2.28 pour hôte2.41. Source revue126da0a/buildcheckoutde5ac7b, diff apps/crates/ui/scripts/deploy/Cargo* vide à la compilation. Aucun crate modifié. BinaireSHA256 `9a17ffe55e3d4128b9d483d170f5d2ede71382cef42d8637917c8f31e63b36a6`. L'exécution surPi et ses dépendances réelles restent NOT_RUN.

Paquet installé r7 : `/private/tmp/chillcenter-linux-package.0iMbsN/chillcenter-pi64-r7.tar.gz`,6,2MiB, SHA256 `ed485f99e14ab8953b7d007959ab9fc35a11a006d4d989260a703fe9be6ebee2`. Même disposition que package-linux.sh avec binaire précompilé et manifeste fidèle à cargo-zigbuild, pas faux manifeste cargo-build. Versions r1–r6 conservées. Binaire/config/lock inchangés, unitéfournisseurSHA76f7d34921ff344e344ba2bffa51263248ec6118dad9e15a39ef7078877d72e2 conservée ; overlayimmuableWorkingDirectorySHA6bad2f8c529705b246253bb8d593d7ae58155b2eb23bacac7afb404e01a5bf1a corrige le démarrage. Compositiondéployée exacte = archiveR7 + drop-in revu, pas falsification du manifeste initial. [Gate r7](G-INTEGRATION-PI-001-r7.md), [reprise partielle](G-INTEGRATION-PI-001-r7-recovery.md), [overlay final](G-INTEGRATION-PI-001-working-directory.md), code/test et sécurité indépendants APPROVED. Déployeur `/root/pi_deployer`, post-déploiement `/root/pi_release_verifier`.

Sources primaires d'outillage : [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild), [release0.23.4](https://github.com/rust-cross/cargo-zigbuild/releases/tag/v0.23.4), [Zig0.17.0 stable](https://ziglang.org/download/), [Debian13 et cycle de support](https://www.debian.org/releases/trixie/). Outils installés dans `/private/tmp/chillcenter-linux-cross.1M5r5u/venv`, pas dansPython/Rust personnels ou sur lePi. Seul l'ancien cache de compilation de revue `/private/tmp/chillcenter-test-review-ded8-target` (inventaire CACHEDIR.TAG/tmp/debug, aucun.git/source) a été supprimé pour récupérer4,8Go ; entièrement recréable, rapports/sources/bundles conservés.

## Installation et retour

Compte/champs et unité issus du runbook LINUX-PACKAGING ; configuration locale `/etc/chillcenter/server.json`, données `/var/lib/chillcenter/data`, binaire `/usr/lib/chillcenter/chill-server`. IP et montage restent configurables. Installer un nouvel état seulement ; aucune migration Cinéfolio/duNAS. Secrets SMB non lus/copiés, cléTLS générée côté serveur en0600, jamais sortie dans les preuves.

Retour d'une première installation : arrêter/désactiver uniquement chillcenter.service si le smoke critique échoue, conserver binaire/configuration/données/compte pour diagnostic et correction ; ne rien supprimer du NAS ni toucher aux autres services. Pour toute mise à jour ultérieure, sauvegarde cohérente pré-migration avec service arrêté et utilisateur de service ; restaurer la sauvegarde si ancien schéma incompatible. Aucun rollback destructif improvisé.

## Vérification après installation — PENDING

Version/hash/config/unit/user/montageRO/TLS/health/bootstrap/profils, scan réel conservateur puis vidéoHTTPSRange/pistes/reprise/seeks/profilstransferts et deux/trois lectures pendant30min. Mesurer RSS/CPU/débit/température/mémoiredisponible/buffering ; seuils de l'architecture, sans inventer succès. Pas de déconnexion physique du NAS, coupure électrique ou reboot affectant Marvin sans accord spécifique ; simuler la perte côté client ou arrêter/reprendre uniquement le nouveau service lorsque sûr. Tests de restauration avec jeu isolé, jamais remplacer les données du foyer pendant le benchmark.

Vérificateur distinct `/root/pi_release_verifier` APPROVED_SCOPED après70srepos, TLS strict et contrôles de service/confinement. [Rapport verbatim](REL-PI-001-verification.md), [gate périmètre runtime](G-POST-PI-001-runtime.md). La charge longue ne s'achève pas sur ce verdict ; [preuves médias réelles](REL-PI-001-media-evidence.md) distinguent scan2496/10,14s, AVI/MKV/MP410s et SRT/dimensionsHD vérifiés, mesures2/3 encoreen cours, restesNOT_RUN.

## Tentatives, diagnostic et reprise exacte

R1 : contrôles locaux PASS, arrêt avant staging/useradd car garde NAS refusait l'option ro initiale. Audit distant a confirmé tous chemins/compte/unité/staging absents ; stop/disable sans objet, pas de rollback destructif. H1 seule confirmée par gardeexit1/tokenroexit0, [BUG-0040](../debug/BUG-0040-pi-install-ro-first-option.md). R2 corrigée mais BLOCKED par revues indépendantes SEC-PI-002 (options contradictoires/plusieursCIFS). R3 exige un seul CIFS exact ro sans rw,9 fixturesPASS ; revues code/test/sécurité APPROVED [gate r3](G-INTEGRATION-PI-001-r3.md).

Tentative r3 : vérifications locales PASS mais aucune sortie distante confirmant staging/installation. Diagnostic SSH root ultérieur exit255 : `sign_and_send_pubkey: signing failed ... Raspberry key ... communication with agent failed`, puis Permission denied(publickey). Le serveur est joignable, sa clé host est connue, il accepte la clé publique mais l'agent local ne signe plus ; pas de contournement de confiance. Inventaire local processus : aucune installation/SSH du déployeur encore en cours. Toutes sessions diagnostiques root ont terminé255 ; aucune tâche cachée laissée. Ne pas interpréter une sortie vide comme installation réussie ni réessayer aveuglément.

R4 ajoute `--ssh-control`; r4/r5 BLOCKED pour tokensOpenSSH/parentdu socket, r6 corrige le validateur : socketpréétabli/UID/modeprivé/type, parentlstatnonlink/UID/exact0700, `%` et `$` refusés avant toute expansion. Options identiques SSH/SCP/strictknownhosts/Batch/timeouts ; aucune clé privée lue, wrapperSSH ou ~/.ssh modifié. Socket `/private/tmp/chillcenter-ssh.UAjtlb/control`, créé en Terminal par utilisateur (« done »), parent0700/socket0600 ; masterPID31195/ssh-OcheckPASS. Inventaire via cette vraieconnexion : LOADnotfound/inactive, config/state/r3stagingabsents ; aucun effet r3, reprise r6 non destructive autorisée. Commande de création utilisée :

```sh
ssh -MNf -o StrictHostKeyChecking=yes -o ControlPersist=2h \
  -S /private/tmp/chillcenter-ssh.UAjtlb/control admin@chillcenter.local
```

Reprise effectivementexécutée : r6binaryELF/ldd et usageexit1 confirmés surPi, arrêt avantprivilèges [BUG0041](../debug/BUG-0041-pi-help-smoke-exit.md). R7corrige contratCLIexact, créecompte997:983+984 puistestNAScwd/home/adminfail ; mêmefindde/cwd/exit0 confirme [BUG0042](../debug/BUG-0042-pi-nas-check-cwd.md). Complémentr7gated vérifiecompteexact+absentspaths et5fichiersidentiques àarchive avantcopie/enable, exit0. Auto-restartensuite9fois, cwd/contreprotectionworkspace [BUG0043](../debug/BUG-0043-systemd-workspace-cwd.md) : stop/disableonlyChill par déployeur, préserveaccount/bin/config/state ; nouvelleinstance `/root/pi_restoration_verifier` APPROVED_RESTORATION. OverlaygatedWorkingDirectory=/usr/lib/chillcenter ajoutéroot0644sansreplacevendor/core ; enable/start exit0, après6s serviceactive/MainPID3016/NRestarts0/cwdséparé. Aucun service tiers/montage supprimé/redémarré.

Première vérificationrootRO11:07UTC : active/NRestarts0 maintenus, certificatpublicSANchillcenter.local, notBefore14oct2026 11:01:41GMT/notAfter14avr2028 11:06:41GMT, empreinteSPKI SHA256 `3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df`. Cléprivée jamaislue. Ces métadonnées ne remplacent pas le GETHTTPS/client et la vérificationindépendante. Le contrôlegateverdictlexicalLOW ethelperexterne sont mitigés par inspectionexacte verdict/hashsavantsactions ; les étatsratés/précédentsgatesconservés, aucune erreur masquée enPASS.

Harnais Rust headless contrôlé dans `scripts/pi-probe` : buildoffline/lockedPASS, refus sans consentementPASS ; aucun test NAS/vidéo exécuté. ProfilQAuniquement, TLSépinglé auSPKIcertificatpublicSSH, vidéo HTTPS sans stockage client, septvitesses/pistes/seek/heartbeat/reprise seconddevice/refus bail libéré. Exiger vrais échantillons720p/1080p, deux puis trois flux30min et métriques, pas extrapoler les trois premiers items ni la compilation. Voir README du harnais pour limites et nettoyage des profils de test uniquement.
