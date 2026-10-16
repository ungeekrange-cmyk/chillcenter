# Sécurité

Chill Center est destiné au réseau local de confiance du foyer. Il ne fournit actuellement pas d’authentification : un profil n’est pas une frontière de sécurité entre utilisateurs. Ne pas exposer le serveur à Internet ni ouvrir de redirection de port publique.

L’HTTPS chiffre les échanges. Accepter un certificat de première connexion demande de vérifier que l’on contacte son propre serveur. Ne jamais transmettre de clé privée ni de mot de passe dans un rapport.

Les certificats, mots de passe, jetons et configurations réelles ne doivent jamais être commités, même dans un dépôt privé. Le serveur génère son matériel TLS dans son répertoire de données ; les clés TMDB se configurent à l’exécution. Conserver les adresses réelles dans un fichier local ignoré (par exemple `server.local.json`), jamais dans les exemples distribués. Seul le port et une écoute de boucle locale sûre sont fournis par défaut.

Avant publication, scanner l’historique et les objets Git, pas seulement le répertoire courant. Un secret supprimé d’un fichier reste dans les anciens commits : s’il est réel, le révoquer puis convenir explicitement du nettoyage de l’historique. Un test synthétique doit être identifié et relu, pas couvert par une exclusion générale de la détection.

## Signaler une vulnérabilité

Si GitHub propose « Report a vulnerability » pour ce dépôt, utiliser ce canal privé. Sinon, demander un canal privé au mainteneur, sans publier de méthode d’exploitation ni joindre de secret. Aucune assistance de sécurité à long terme ni délai de correction n’est promis à ce stade.

Indiquer la version, les préconditions, l’impact et une reproduction anonymisée. Les dépendances du lecteur et leurs licences restent distinctes du code Chill Center.
