# Livraison ressources — 16 octobre2026

Coordinateur/auteur `/root`, déployeur distinct `/root/configuration_deployer`, vérificateur distinct `/root/configuration_release_verifier`. Source exacte **f9803557a452f59cbbe3cbf5b1c4bf1b48a9aa67**, base228c4c6, branche `codex/resource-optimizations`. DEV_FAST familial ciblé ; autorisation explicite optimisation/mesures/build Mac/mise à jour Pi, fermeture clients confirmée puis recontrôlée. [Revues code/sécurité APPROVED_SCOPED](../reviews/resource-optimization-f980355.md) ; [ADR0024 accepté](../architecture/decisions/ADR-0024-artwork-content-deduplication.md).

## Artefacts et tests

- Mac : **`artifacts/macos/Chill Center-test-f980355.app`**, interne0.5.1/build6,48dylibsARM64/signature ad-hoc/libmpvAPI2.5 PASS. SHA256 exécutable **ad699344f983eee9ba3e204b57d925009152a0688cec8e3677e794a2d3f135ee**. Versions/tag précédents conservés,9fb intermédiaire non remis.
- Serveur : **`artifacts/linux/chill-server-only-f980355/{chill-server,manifest.json}`**,ARM64/glibc2.28 ; SHA256 binaire **6fecabc69258e372d71730a23451f5ff47565f60cbbc352552c12d8ac39424ea**, manifeste3f97479f1cc89ff331dc3a54a1eedeb46d0ea6fc6162e3538289d634f7bb1c32. Buildlocked/offline/validate-elf PASS, aucun paquet web/unité/configuration inclus.
- Client final51tests PASS et benchmark séparé ; serveur137PASS/1benchmark ignoré, API76PASS, Clippy client/serveur lib/tests/examples-Dwarnings PASS après correction des deux emprunts inutiles. Pas de runtime/dépendance/Cargo.lock changé. Deux warnings natifs dead_code et un warning linker historiques conservés, pas une régression masquée.
- [Mesures avant/après](../performance/RESOURCE-OPTIMIZATION-20261016.md), benchmarks locaux10000médias ; aucun nouveau lecteur général, fournisseur TMDB/IA ou scan NAS exécuté.

## Cache réel du Mac

Client fermé/recontrôlé, outil Rust de la source revue ; migration explicite terminée rc0. **3926/3926références d'images** vérifiées, même total logique173576179octets. Ledger privé : **3930fichiers préexistants** contenu/taille/mode identiques, fichiers non-images inode/date identiques,600nouveaux blobs seulement. Allocation de fichiers du cache, inodes uniques : **198754304→49860608octets**, soit **148893696octets≈142Mio** de moins, pas un chiffre de volume libre APFS garanti. Images seules181870592→32976896octets. Aucun catalogue, WAL/SHM, paramètre, historique ou version n'est supprimé.

Manifestes privés avant/aprèsSHA9afe38307b44911e376f6eb049b0a3667d4e3dc3a42d1f3302da280c256843f8 /1d31a2a22608cb8b03633a8ae87781b1f86a89654f2037c6ee1f00a7628852b2 ; vérificateur distinct relit le cache courant et confirme l'état après byte-identique. Les noms et contenus privés de ces manifestes restent horsGit. Retour ancien bundle lisible via les mêmes chemins, aucune migration SQLite inverse nécessaire.

## Pi — binaire seul

Opérateur V5 exactSHA **a673a7a4d6494210f12140652363368787f3588e7486c2466405eb7083018c8d**, chaîneV4/V3/V2 inchangée revue au lot précédent et recontrôlée. Préflight oldfile/processdbda76e377a0f8ce84262396e6c339e44474846434351f9b4ec5bb5db4f5f100, connexions0, jobs persistés absents/idle ; pause de service pour sauvegarde exacte, sans intervention pendant les attentes d'I/O, remplacement atomique du seul exécutable.

**Fin durable22:03:11UTC, rc0** ; PID5420, actif/running, NRestarts0, ExecMainStatus0. Hash installé **et** processus6fecabc…24ea. Ancien exécutabledbda76…f100 conservé privé pour rollback. Sauvegarde opaque **3c3d64c3-ac3b-4101-8da3-503a43222936**,0700 ; manifesteSHA **985849e9f9d7efc4a28039e689427d76e666cff0ce0ec1934b457e202e786dd0**,1924checksums et1SQLitequick_check indépendants PASS.

Invariants après bascule : configurationd6167cb1624b6d5888473af76d5470102afedce25b2e29ddbaa032730cd0ceb1, unité76f7d34921ff344e344ba2bffa51263248ec6118dad9e15a39ef7078877d72e2, SPKI3018fb4d28af3899ae9a0261c89ef92d7ebfe508ecbd613b38f6dd6d7d4740df, web07868e61cd75dc36d5bf62eefd258b7409b6ed264bde726cdcf626d45b57fe2b, montagebdfed6860cca749f250293cab6df1ace5b2ba0457e246afd956fa0d1245cb92a tous inchangés ; `library.json` toujours absent. Aucun fichier du client web installé ou en chantier, NAS, configuration, certificat ou unité modifié.

## Vérification et rollback

[Vérification post-bascule distincte](REL-RESOURCE-OPTIMIZATION-001-verification.md) **APPROVED_SCOPED**22:04:21UTC : sauvegarde exacte/permissions/ancien binaire, service/hash, six sondes HTTP/HTTPS strict des APIs2026-10-14/v1 et web200, aucun warning+ journal, intégrité cache réelle. RSSPi14872Kio n'est qu'un échantillon au repos et **pas** une mesure avant/après démontrant un gain RAM sur Pi.

Retour serveur : restaurer uniquement l'exécutable précédent préservé, redémarrer et recontrôler service/API/web/invariants. Aucun schéma ou réglage n'est changé par ce lot ; ne pas restaurer aveuglément la base ou des états utilisateur plus récents. Cache Mac : les anciennes feuilles restent compatibles ; pour défaire physiquement les liens si nécessaire, recopier une feuille vers un fichier temporaire vérifié puis rename, jamais écrire dans un inode partagé. Aucun rollback recommandé par le vérificateur.

Pas de push/publication, checkout de la branche active canonique ou modification du chantier web. Les branches/worktrees, sources, historiques et artefacts livrés restent conservés.
