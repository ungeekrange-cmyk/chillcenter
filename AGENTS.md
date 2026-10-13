# Consignes Chill Center

Lire `docs/PROJECT_CONTEXT.md`, `docs/discovery/SCOPE.md`, l'architecture, l'index des ADR et le journal des bugs avant toute intervention. Le projet est distinct de Mongoose Posture : aucun cloud AWS/Azure, aucune priorité d'authentification ou livraison de ce dernier ne doit lui être imposé.

Respecter les quatre étapes demandées par l'utilisateur : fonctionnalités, documentation/architecture, user stories, implémentation. Une validation explicite termine chaque étape ; ne pas avancer à la suivante sans elle. Les demandes et corrections ultérieures de l'utilisateur priment.

Préserver Cinéfolio, ses données, le NAS et tout travail existant. Un import ne renomme, ne déplace ni ne supprime une vidéo. Le serveur n'effectue aucun transcodage. Le client n'accède pas directement au NAS.

Avant de coder, examiner l'état Git, travailler sur une branche `codex/`, figer les versions compatibles et vérifier les contrats concernés. Ne pas déployer sur un matériel ou un réseau supposé. Pas de publication ou de création de coût cloud implicite.

Documenter chaque décision significative dans un nouvel ADR. Ne pas réécrire une décision acceptée : la remplacer par un ADR lié. Chaque défaut reproductible reçoit un BUG avec symptômes, environnement, hypothèses, preuves, correction et résultat de régression. Distinguer un bug applicatif, une dépendance indisponible et une observation d'outillage. Ne pas présenter une vérification prévue comme exécutée.

Si une revue indépendante est utilisée, le reviewer n'est pas l'auteur et n'édite pas le contenu revu. Un seul auteur par chemin ; pas de délégation récursive implicite. Les restrictions de ressources du Pi et le périmètre familial restent prioritaires.
