# ADR-0009 — Validation et préparation des archives et images

2026-10-14 · IMPLEMENTED_DETAIL · auteur /root · complément d'ADR-0006 accepté · supersedes: NONE. Détail d'implémentation revu au SHA126da0a (code/test/sécurité), pas nouvelle approval utilisateur revendiquée.

Périmètre déjà approuvé : US-010, US-027 et US-028. Pas de nouvelle fonction produit ni nouvelle validation utilisateur revendiquée. Candidat d'implémentation soumis à revue indépendante avant livraison.

Décision : ZIP9.0.3, formats Stored/Deflate uniquement (export Stored pour économiser le CPU), manifeste JSON v1 avec SHA-256. Limites annoncées : 1 GiB compressé, 2 GiB logique, 20 000 entrées, document et manifeste 64 MiB chacun ; 6 aperçus, staging2 GiB, expiration15 min. Aucun chemin absolu, traversée, lien, entrée chiffrée, archive imbriquée ou entrée non déclarée. Références des affiches logiques ; vidéos, chemins NAS, tokens et clés privées exclus.

Préparation sans mutation : correspondance complète/injective des profils, préfixe relatif et politique explicite conserver/remplacer. Plans privés côté serveur, UUID opaques côté UI. Application sous transaction SQLite IMMEDIATE, contrôlée par epoch de catalogue et révision de racine ; toutes les données communes/personnelles ensemble ou rien. Reçu export+correspondance garantit la répétition sans nouveaux profils/doublons. Annuler ne supprime que le staging créé par ce service, jamais source ou vidéos.

Images : `image`0.25.10, sans formats par défaut/rayon, JPEG/PNG/WebP seulement. Décodage réel, 8 MiB encodés, dimensions≤4096×4096, budget décodeur64 MiB ; le budget d'allocation est une limite best-effort du fournisseur et non une mesure de pic RSS Pi. Fichiers immuables0600, fsync. Descripteurs de NAS jamais utilisés pour importer une affiche libre.

Sources primaires : [ZIP](https://docs.rs/zip/9.0.3/zip/), [image](https://docs.rs/image/0.25.10/image/), [limites image](https://docs.rs/image/0.25.10/image/struct.Limits.html), vérifiées2026-10-14. Rust1.99.0 local ; compatibilité ARM et consommation physiques restent à mesurer.

Conséquences : dépendances supplémentaires mais codecs bornés ; refus explicite des images géantes/corrompues. Les aperçus volatils ne survivent pas au redémarrage ; l'archive source doit être sélectionnée à nouveau. Aucun import ou restauration de données du foyer exécuté. Retour logiciel par branche antérieure, source/NAS intacts ; tables de reçus ajoutées sans supprimer de données.

État du brouillon historique : snapshot/export enjob, quotaexport, CLI Cinéfolio etrestauration étaient encoreàlivrer. Ils sont désormaisimplémentés ettests locaux liés dans COMPLETION-EVIDENCE/EXPORT-JOBS/CINEFOLIO-EXPORT/OPERATIONS ; aucun import/restaurationfoyer ou performancePi prouvé. Cette miseàjour de statut ne réécrit niADR-0006accepté ni le format défini ci-dessus.
