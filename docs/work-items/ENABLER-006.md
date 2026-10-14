# ENABLER-006 — Mesurer ressources et trois lectures sur le Pi

14octobre2026 : IN_PROGRESS, mesures exécutées avec échecs, nonDONE. Pi3B+/Debian13.7/NASRO installé et vérifié, dataset2496 réel (pas3000), troisformats10sPASS, deuxHD30minterminés avecstallsMKV, puis3fluxréels(Dune1/Dune2/720p)30min : **Dune2exit1/progression<80%**, pascapacité3validée malgréRSS24,23Mio/CPUmax22,95%d'uncœur/swap0. Essaiadditionnel deuxDune10minterminéexit0/PASSmaisbuffer184débutDune2, pasfluiditéuniverselle. Voir [preuves exécutées/limites](../releases/REL-PI-001-media-evidence.md) ; ne pas clore l'enabler sur compilation, moyenneCPU ou lecture partielle. Les statuts documentaires anciens ci-dessous conservent leurhistorique.

Correction fonctionnelle utilisateur14octobre : le budgetclient64Mio deEXIT-03historique est remplacé par [ADR0010CC-CACHE-002](../architecture/decisions/ADR-0010-client-playback-cache.md), cibleavance60s/défaut128/64Mio, configurationlocale16–512/0–256Mio, plafonds approximatifs de paquets, pasRSS. Les mesuresmémoireclient anciennes sontSTALE ; seuils serveur restentinchangés. Dataset3000/RSSwebviewmax/pannesphysiques/backseeksansrefetch encoreNOT_RUN.

- Type : ENABLER ; aucune user story technique déguisée.
- Révision : CC-STORIES-001 ; statut documentaire : REVIEW_REQUESTED.
- Estimation : 5 points ; Mesures bout en bout sur matériel réel et profils de charge, aucune complexité déduite de la seule résolution.
- Depends On : US-016, US-017, US-019, US-023, ENABLER-002.
- Blocks : ENABLER-007.
- Trace : F-01, F-03, F-04 ; décisions ADR-0001, ADR-0004 ; vérifications V-01, V-02, V-03.
- Auteur : /root ; gate G-03 en attente, travail non commencé.

## Preuves de sortie requises

- ENABLER-006-EXIT-01 : Matériel/réseau/fichiers réels identifiés : Pi 3 B/B+, Wi-Fi, NAS 1 Gb/s, deux puis trois profils et vidéos 1080p/720p pendant trente minutes.
- ENABLER-006-EXIT-02 : Rapporter débit encodé/pics, buffering, RSS/CPU/température et mémoire système ; mesurer avec seeks/pistes et sans jobs lourds en concurrence.
- ENABLER-006-EXIT-03 : Objectifs approuvés : RSS serveur ≤100 MiB repos/3 000 fiches, ≤200 MiB trois lectures hors scan ; CPU repos ≤2 % d'un cœur sur 60 s ; cache vidéo client ≤64 MiB, buffers serveur ≤1 MiB/flux.
- ENABLER-006-EXIT-04 : Files bornées, scan isolé, fingerprint ≤1 MiB/s et suspendu pendant lecture ; pas de préchargement intégral ni de transcodage.
- ENABLER-006-EXIT-05 : Absence de matériel ou insuffisance Wi-Fi : preuve NOT_RUN ou échec explicite, pas succès extrapolé ; une cible à trois lecteurs non atteinte n'est pas masquée.

## Contraintes

Les interfaces approuvées autorisent fixtures et tests locaux pour préparer l'intégration. Elles ne prouvent pas le serveur déployé, la lecture native ou le Wi-Fi réel. Enregistrer environnement, SHA, commandes, résultats et limites ; pas de modification de Cinéfolio, des vidéos ou d'identités/credentials supposés. Toute preuve non exécutée reste NOT_RUN.

