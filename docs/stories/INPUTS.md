# Entrées et autorité de planification

Révision CC-STORIES-001 ; auteur /root ; date 2026-10-13.

- Périmètre approuvé : [CC-FONC-002](../discovery/SCOPE.md), avec 22 exigences, exclusions et corrections utilisateur conservées.
- Architecture et contrats approuvés : CC-DOC-001 ; [preuve G-02](../gates/G-02-CC-DOC-001.md) et manifeste historique SHA-256 0be59a21e1d93eda975b3cb9f29a70fd3de5fa21f99ad8da4db392d815025d43.
- Revue indépendante architecture : APPROVED r2 ; choix de moteur reste à prouver par enabler, pas garanti par cette approbation.
- Source Cinéfolio : 0.5.0, commit 90c149b797b45cba8a56dca2b74eed06c4225060 ; données et application source préservées.
- Pas de PRD séparé imposé : l'utilisateur a défini puis approuvé son workflow fonctionnalités → documentation/architecture → stories → implémentation. SCOPE joue ici le rôle d'entrée fonctionnelle approuvée du skill story-breakdown ; aucune approbation PRD fictive.
- Les 33 stories et neuf work-items reprennent ce périmètre complet. Aucun client web, serveur Mac, dictée, authentification, transcodage serveur ou cloud ajouté.
- Les points sont relatifs et initialement estimés par /root ; la revue vérifiera cohérence, atomicité et absence de cycles.
- Rôle « gestionnaire du foyer » : personne utilisant Administration, sans nouveau niveau d'authentification ni restriction aux autres profils.
- Constat technique source : les limites internes du sélecteur et de l'IA doivent rester bornées ; aucun grand catalogue n'est dupliqué en RAM au démarrage de chaque appareil.
- Enabler 006/007 nécessite matériel Pi, réseau, accès Linux et fichiers représentatifs avant preuve. Aucun accès SSH, chemin de montage ou identifiant n'est supposé ; fixtures et builds seuls ne valident pas le NAS/Wi-Fi.

Après revue et validation utilisateur G-03, le premier travail autorisé sera la preuve lecteur puis les fondations selon le DAG. Le code n'est pas commencé à cette étape.

