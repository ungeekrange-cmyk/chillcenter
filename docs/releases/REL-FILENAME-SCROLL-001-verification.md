# Vérification indépendante — REL-FILENAME-SCROLL-001

**Verdict : APPROVED_SCOPED**  
**Contrôle :** 2026-10-15, après bascule annoncée à `2026-10-15T12:43:07Z`  
**Périmètre :** contrôle post-déploiement DEV_FAST, lecture seule du Pi et une seule sonde réelle du client. Ce rapport n'est ni une approbation de revue de source, ni une validation Tauri/GUI humaine, de lecture vidéo, TMDB, scan ou NAS.

## Artefact et disponibilité

- Serveur exécuté : `/usr/lib/chillcenter/chill-server` SHA-256 `cef54cd0e8baf45f70ee0f57584001521c7dcd705d4f8ab351eb8074f524b3c3`, conforme à l'artefact déployé déclaré.
- Service : `active/running`, PID `8460`, `NRestarts=0`.
- Aucun événement `warning` à `alert` de `chillcenter` depuis la bascule (`0`).
- Le commit de livraison déclaré `c610ea9a542f289902ebd6c356bcb1f369b7f150` existe et est ancêtre du checkout observé. Le checkout était ultérieurement sur `d08d80f6e8de1012c937aab51f8a5681706c161a`; ce rapport ne revendique donc pas une indépendance de provenance/compilation du binaire à partir des sources.
- Bundle Mac privé : exécutable `chillcenter-desktop` SHA-256 `ecde360fea63c2e8c85693a2dee657cb288bf6146d5a33c3a956706b77007c0e`, conforme à la valeur transmise. Son essai GUI reste hors périmètre.

## Smoke réel et sécurité de transport

- Sonde privée autorisée, SHA-256 `8b563984a9313a362f7bdeb3d27376f61b983da2a3ad7bf7705b8cf0fe0ff47f`, exécutée une fois avec un nouveau répertoire client privé et sans création de profil.
- Résultat : connexion HTTPS épinglée, catalogue intégral (2496 éléments), fiche/sagas/listes, snapshot inchangé et réouverture chaude du cache : **PASS**. Durée totale 9 933 ms ; parcours local 5 002 ms. La sonde confirme `household_writes=false`.
- SPKI observé : `3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df`, conforme.
- Routes HTTPS authentifiées par le certificat local : `/api/2026-10-14/server` **PASS** et alias `/api/v1/server` **PASS**. OpenAPI déclaré inchangé : `01423e6a3ace79453671a59cb67674db2c575b27bd295a1c2a83965906d9acae`.

## Préservation et retour arrière

- Configuration SHA-256 `ea24f2f0a98db2b0a080ab70113a2fbe8eb1006a20e2eae646d115448f3783f9` et unité SHA-256 `76f7d34921ff344e344ba2bffa51263248ec6118dad9e15a39ef7078877d72e2`, conformes aux valeurs pré-bascule transmises.
- SPKI et empreinte du montage vidéo (`e3711e5a4ae29124d937a663a2248cf642791efe63fa6ceedb9e8ea8a1a524b3`) conformes aux valeurs pré-bascule transmises. Le montage reste hors accès de contenu dans ce contrôle.
- Sauvegarde privée `2cbaefcf-58fa-4b12-a9c3-3f03b75105ae` : permissions `0700`, propriétaire `chillcenter`; manifeste v1/schema 3, 1 713 entrées vérifiées par taille et SHA-256, ensemble exact de 1 714 fichiers sans lien symbolique : **PASS**. Empreinte agrégée des contenus : `9938ef278d386abdc1dd501d14f29b8765cbebb1a2d71c4502dbbdc7e2ac9119`.
- Base SQLite de sauvegarde ouverte en `mode=ro&immutable=1`; `PRAGMA quick_check` : **PASS** (1 base).
- Rollback conservé : `/root/chillcenter-rollback/chill-server-cfc87de-f85375ad4ac33fd0`, SHA-256 `f85375ad4ac33fd0993151b547e8ca731d0ffa49441beb8b85756f0433f842cd`, conforme à l'ancien binaire attendu. Il est disponible pour la procédure documentée; aucun rollback n'a été exécuté.

## Limites et rollback

L'approbation couvre uniquement l'état réellement servi, le smoke client borné, TLS/API, intégrité de sauvegarde et disponibilité du rollback. Elle ne couvre pas une preuve indépendante de build source→binaire, les validations humaines Tauri/visuelles, ni les fonctionnalités explicitement exclues. En cas de régression, arrêter la promotion utilisateur et appliquer le rollback binaire CFC documenté, puis demander une nouvelle vérification indépendante de l'état restauré.
