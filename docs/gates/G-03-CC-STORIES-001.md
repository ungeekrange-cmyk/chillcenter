# G-03 — Validation des stories et autorisation d'implémentation

- Statut : APPROVED ; utilisateur humain : « validé, go implémentation ».
- Date : 2026-10-13, Europe/Dublin ; instance de consignation /root.
- Entrée : CC-STORIES-001, 33 stories/133 AC ; revue indépendante APPROVED r2, SHA-256 082cb455e3729ea4c82130aac17365931e00152e1d526d13dae940af2ce67d5f.
- Input digest de revue : f72ae20ae52f900281bba1f2df4a854d7867471d6ce67ddda8a327bb87ce31e4.
- Action autorisée : implémenter la version Linux/Mac complète selon le DAG, prérequis lecteur d'abord ; opérations ordinaires de branche/commits/build/tests/repo autorisées par cette demande.
- Adresse Pi prévue : 192.168.1.42 ; état matériel déclaré par l'utilisateur : pas encore prêt. Aucune connexion/deployment/smoke Pi revendiquée ; Linux/NAS/SSH non configurés implicitement.
- Montage prévu : `/srv/volumes/video`. Correction utilisateur : « mais ça c'est la configuration qu'on aura là. Il faut que je puisse la configurer ». Adresse/port DNS-IP éditables dans le client ; écoute/SAN via configuration serveur, racine NAS via administration persistante. Aucun hardcoding de ces exemples dans les comportements.
- Lane : DEV_STANDARD, exigences/architecture/contrats approuvés, sans changement de périmètre. Revues indépendantes avant remise/merge ; aucun cloud ni déploiement production.
- G-04 : validation de la remise de l'implémentation après preuves ; non ouverte par G-03.
