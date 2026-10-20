# v0.7 — corrections et preuves de reprise

**Français** | [English](COMPONENT-UPDATES-FIXES-20261019.en.md)

## Clôture des vérifications — 20 octobre 2026

Source runtime/packaging finale `3e174b9e6ed89e08f67e763c3b30028c15767080` :
[code APPROVED_SCOPED](../reviews/component-updates-code-3e174b9.md),
[sécurité APPROVED](../reviews/component-updates-security-3e174b9.md),
[tests APPROVED](../reviews/component-updates-tests-3e174b9.md).
Les huit HIGH et quatre défauts fonctionnels sont clos. Le dernier contrôle
AppKit d'annulation 0.7.2 passe : un GET du seul manifeste, aucun téléchargement
de paquet, arbre/exécutable/signature/marqueur inchangés et sortie 0. Les trois
acceptations précédentes restent des observations QA défavorables conservées.
La vraie installation signée, le redémarrage et les deux gardes lecteur passent.

Régression runtime inchangée : **435 Rust PASS, zéro échec, deux NFR existants
ignorés ; 359 Node PASS**. Packaging Python **19/19 PASS**, dont cinq tests DMG.
DMG réel vérifié/monté en lecture seule, application identique au bundle signé,
raccourci Applications et instructions FR/EN ; minimum réel macOS27 calculé
sur les 49 Mach-O. Aucune signature Apple Developer/notarisation revendiquée.
Les rapports séparent les sources revues des futurs commits porteurs de docs.
Les [notes v0.7](../releases/REL-V0.7.md) et manifestes signés suivent la
publication ; aucun Pi, NAS ou client web déployé n'a été contacté/modifié.

## Checkpoint historique du 19 octobre

Source de référence : `6041f4dc8acb6e14ea59bcb4e8ce6cec5d9edcaa`.
Ce document complète le [checkpoint initial](COMPONENT-UPDATES-20261019.md),
sans effacer les revues défavorables. **À ce checkpoint historique, le candidat
n'était pas publié et les gates natives/finales restaient ouvertes.**
Le Pi, sa configuration, le client web déployé et le NAS n'ont pas été contactés.

Delta `3913267` : le premier vrai lancement Tauri a révélé
[BUG0115](../debug/BUG-0115-native-updater-bootstrap-config.md), champ public
obligatoire du SDK absent avant l'application du builder. JSON fixé sur la même
clé publique et test de désérialisation réel ajoutés ; retest natif en cours.

Revues indépendantes de `3913267` : [code APPROVED_SCOPED](../reviews/component-updates-code-3913267.md),
[sécurité APPROVED](../reviews/component-updates-security-3913267.md),
[tests CHANGES_REQUESTED](../reviews/component-updates-tests-3913267.md).
Seul CU88-TEST-003 reste ouvert : parcours natif réel. Les huit HIGH et les
quatre défauts fonctionnels sont clos dans leurs revues respectives.
Le delta natif passe 8/8 tests ciblés. Bundle Mac candidat construit et vérifié
(0.7.0/build9, 48 dylibs arm64, libmpv API2.5), non publié. Les
[journaux conservés](../releases/evidence/v0.7-candidate/) distinguent les suites
de référence, le delta natif et les preuves VM.

## Corrections des huit constats de sécurité

| Constat | Correction et régression |
| --- | --- |
| Autorisation d'activation périmée | Jeton à usage unique, transaction/cible/PID/boot/instant de démarrage liés, durée limitée et consommation durable avant exécution. Rejeu refusé sous le vrai sandbox systemd. |
| Cache partiellement publié | Répertoire temporaire privé, taille/hash et fichiers synchronisés avant publication atomique ; interruptions de téléchargement et réutilisation du cache complet testées. |
| Environnement de bootstrap | Bash privilégié, PATH fixe, variables de démarrage supprimées, Python système résolu/vérifié et mode isolé, Minisign système absolu. |
| Sauvegarde non bornée | Temps et stdout/stderr plafonnés, groupe de processus arrêté/récolté ; sorties infinies et descendants bloqués testés. Sauvegarde exécutée sans privilèges résiduels. |
| Préparation pendant l'arrêt | Extraction, validation et permissions finales préparées avant maintenance. L'occupation ne consomme pas de séquence root et ne change pas le slot actif. |
| Récupération incompatible avec systemd | Pré-démarrage privilégié `+`, racines de composants en lecture seule dans le processus serveur, écritures root réservées au helper. Vérification en VM avec les unités réelles. |
| Slots illisibles | Dossiers 0755, données 0644, exécutable serveur 0755, refus des liens/fichiers spéciaux. Lecture réelle par le compte de service et refus d'écriture vérifiés. |
| Arrêt/crash sans reprise | `ExecStopPost` borné, ancien slot restauré avant redémarrage, drapeau de maintenance retiré, récupération idempotente. TERM/KILL pendant sauvegarde et candidat déjà actif testés. |

La VM a aussi révélé deux défauts réels : l'arrêt externe ne doit pas expirer
au même instant que les 30 secondes de systemd ; une unité `Type=simple` peut
être démarrée avant que son écoute HTTPS soit prête. L'arrêt supervisé dispose
maintenant d'une marge et d'un repli borné ; le contrôle de santé attend au plus
15 secondes avec CA, résolution et version attendue fixes. Le budget d'arrêt
du helper est explicite : 150 secondes pour une récupération pessimiste d'environ
115 secondes. Un candidat ignorant TERM a été tué à 30 secondes puis remplacé
par l'ancien service sain lors de l'essai réel.

`CAP_SETUID`, déjà dans la borne autorisée, est conservée pendant le lancement
du helper pour permettre la descente vers le compte de service. Pendant les
deux sauvegardes artificiellement bloquées, les UID/GID réels/effectifs/sauvés
sont ceux du service, les groupes supplémentaires sont vides et
`CapPrm`, `CapEff`, `CapAmb` sont tous nuls. `NoNewPrivileges` reste actif.

## Corrections des quatre défauts fonctionnels

- Occupation : même demande différée, même digest, origine manuelle/automatique
  conservée, délai de reprise de 15 minutes, plafond de trois tentatives ;
  changement de politique annulant une demande automatique en attente.
- Crash : réconciliation des demandes non terminales et verrou global ; une
  acceptation root interrompue exige une reprise locale explicite, pas un clic LAN.
- Vérification en cache : seules les vérifications terminées avec succès et
  résultat « disponible » annoncent une mise à jour. Les octets exacts d'une
  transaction acceptée restent conservés avant toute nouvelle découverte.
- Reprise Mac/thèmes : reçu privé durable lié aux octets signés, signature,
  version, source, séquence et instant d'acceptation. Une reprise expirée ne
  vaut que pour ces octets acceptés pendant leur validité ; une nouvelle découverte
  expirée, une substitution et un instant futur sont refusés. Chaque installation
  Mac demande encore une confirmation native.

Les erreurs transitoires conservent l'état serveur connu ; une ancienne API
non compatible est distinguée d'une indisponibilité. Le planificateur ne
consomme que les composants de son hôte.

## Preuves exécutées

- macOS Apple Silicon, Rust 1.99.1 : suite Rust sérialisée **433 PASS, 0 échec,
  2 harnesses NFR préexistants ignorés** ; client desktop 35 tests dont 7 updates.
- Suite JavaScript/web/documentation : **359 PASS**, aucun ignoré.
- Clippy workspace/all-targets `-D warnings`, format Rust, diff et contrat
  statique de durcissement : PASS.
- Chromium local : interface de configuration réelle avec pont synthétique,
  conservation du brouillon, CAS, erreurs 503/404 et catalogue utilisable pendant
  une requête lente : PASS. Ce n'est pas une preuve AppKit.
- VM locale jetable Ubuntu 24.04.4 LTS ARM64, noyau 6.8.0-117, systemd 255,
  Minisign 0.11 : matrice de panne complète PASS ; deux tests de ledger root
  réellement exécutés **2/2 PASS**, au lieu d'être assimilés aux tests macOS.
- Occupation créée par une véritable mutation HTTP TLS laissée ouverte :
  demande manuelle différée puis reprise par `tick`, même identifiant et deuxième
  tentative, paquet web signé installé et version observée par le serveur réel.
  L'horodatage de la seule fixture a été avancé pour ne pas attendre 15 minutes.
- Manifestes/signatures de test uniquement : cas valides Mac/web/thèmes,
  altérations, expiration, incompatibilités, archives ZIP hostiles, bornes,
  état corrompu et historique limité. Aucune clé privée de production utilisée.

Commande Linux principale (dans la VM gardée par son hostname dédié) :

```sh
sudo env CHILL_UPDATE_DISPOSABLE_VM=1 python3 -u scripts/test-updates-linux-root.py \
  --updater /chemin/absolu/chill-update --server /chemin/absolu/chill-server \
  --resume-fixture --reset-fixture-ledger
```

Ce script refuse un hôte ordinaire. L'option de remise à zéro archive uniquement
le ledger synthétique de cette VM. Ne jamais lancer ce harnais sur une installation
réelle. Les unités finales sont aussi validées par `systemd-analyze verify`.

## Limites et publication

Les essais ne prouvent pas un appareil Safari physique, une panne secteur,
un disque plein ou toutes les défaillances réseau GitHub. Aucun essai sur Pi/NAS
n'est revendiqué. La confirmation/installation/redémarrage AppKit doit encore
être observée dans un bundle QA isolé ; cette limite n'est pas un PASS implicite.

Les sources et anciens livrables sont préservés. Les paquets doivent être
construits sur une source propre, vérifiés et signés avant création des releases
indépendantes et de la release récapitulative v0.7. Le retour arrière conserve
les anciens slots et sauvegardes ; l'anti-rollback ne doit jamais être désactivé.
