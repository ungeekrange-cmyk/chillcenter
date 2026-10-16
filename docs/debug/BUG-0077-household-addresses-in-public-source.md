# BUG-0077 — Adresses du foyer dans les exemples et outils

16 octobre 2026, préparation du dépôt public ; **FIXED_LOCAL, historique privé à traiter séparément**. Les défauts du programme serveur étaient déjà limités à la boucle locale, mais deux exemples de configuration et des outils opérateur pointaient vers une installation réelle. Les comptes rendus reproduisaient aussi ses adresses.

Cause confirmée par inventaire des fichiers suivis et des objets Git. Version proposée : exemples loopback, port `8443` conservé ; cibles opérateur et URL du harnais fournies explicitement à l’exécution ; noms DNS anonymisés dans les comptes rendus, avec avertissement que les anciens hashes/gates décrivent les sources historiques. Les fixtures LAN utilisent des adresses synthétiques, pas l’installation du foyer.

Régression : exemples analysés par le vrai `ServerConfig`, validations de configuration ; tests locaux des scripts et faux SSH prouvant qu’aucune connexion ne part si la cible est absente/invalide ; propagation NAS jusque dans les blocs sudo ; compilation hors ligne du harnais et refus de démarrer sans URL explicite. Aucun SSH réel, déploiement, accès NAS ou lecture vidéo.

Le correctif ne supprime pas les anciennes adresses des anciens commits. Aucun secret réel identifié dans le périmètre scanné ; les alertes TMDB sont des fixtures déterministes, revues indépendamment. Publication du dépôt complet toujours suspendue au choix d’un état public sans historique privé ou d’un assainissement coordonné, après accord explicite. Retour arrière : revert de la correction locale seulement ; ne pas réintroduire une cible implicite dans un dépôt public.
