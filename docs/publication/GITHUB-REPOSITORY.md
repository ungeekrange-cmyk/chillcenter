# Préparer le projet GitHub public

Le livrable principal est le **dépôt du projet** : sources, README, licence, contribution et sécurité. GitHub Pages est une présentation optionnelle, pas une nouvelle application web. Aucun dépôt, site ou historique n’a été publié par ce lot.

## Configuration distribuée

- Aucun hôte du foyer dans les valeurs par défaut : les deux exemples serveur écoutent sur la boucle locale, port `8443`.
- Conserver l’adresse LAN, les chemins réels et les noms TLS dans une configuration locale ignorée, par exemple `server.local.json`. Les DNS figurant dans la documentation sont des exemples, pas des valeurs fonctionnelles préconfigurées.
- Les outils opérateur historiques demandent explicitement `CHILLCENTER_TARGET_HOST` ; la première installation demande aussi `CHILLCENTER_LISTEN_ADDRESS`, `CHILLCENTER_TLS_NAME`, `CHILLCENTER_NAS_SOURCE`. La reprise R7 demande la source NAS ; le harnais demande `CHILLCENTER_SERVER_URL`.
- Aucun outil n’a été exécuté contre un appareil. Les anciens hashes de scripts et gates restent des preuves historiques ; les scripts modifiés doivent être revus et les gates régénérés avant toute réutilisation. Le premier installateur reste lié à son ancien artefact approuvé : ce n’est pas un installateur générique pour la version actuelle.

Les IP privées encore présentes dans des tests sont des fixtures synthétiques destinées à tester les garde-fous LAN. Les adresses de boucle locale sont intentionnelles et ne doivent pas être remplacées par une écoute wildcard.

## Vérification des informations sensibles

Effectuer les contrôles sur les fichiers commités **et sur tout l’historique qui deviendrait public**, y compris les branches et tags. Un contrôle de l’index ou de HEAD seul est insuffisant.

Outil utilisé pour ce lot : Gitleaks `8.30.1`, binaire officiel téléchargé et vérifié par SHA-256. Il s’exécute localement, sans transmettre les sources. Exemple de commande à lancer avec un rapport hors dépôt et des droits privés :

```sh
umask 077
gitleaks git --log-opts='--all --full-history' --redact=100 \
  --ignore-gitleaks-allow --report-format json \
  --report-path /chemin/prive/audit-redacted.json .
```

Ne pas activer le mode verbeux ni recopier une valeur détectée dans un ticket. Inspecter les alertes en privé. Les alertes de ce lot ont été classées indépendamment comme fixtures hexadécimales déterministes de tests TMDB sous `cfg(test)`, sans vérifier leur validité auprès d’un fournisseur. Aucune règle générale d’exclusion des clés ou dossiers de tests n’a été ajoutée.

Compléter par un inventaire des noms sensibles, blocs PEM, objets DER et objets Git complets : une recherche de texte ou un scan de diff ne suffit pas à exclure tous les certificats. Les certificats/clés du serveur sont générés à l’exécution et ne sont pas des assets à distribuer. Les mots de passe du NAS ne doivent jamais être nécessaires au client ou au dépôt.

Les extensions de certificats/clés, fichiers d’environnement et configurations `*.local.json` sont ignorés. Cela réduit les accidents futurs mais n’efface pas les fichiers déjà présents dans l’historique. Un scan est une assurance raisonnable dans son périmètre, pas une preuve mathématique d’absence de secret.

## Historique : décision encore requise

L’audit n’a pas identifié de secret réel dans les sources/historiques locaux examinés. **Les anciennes adresses du foyer restent néanmoins dans des commits antérieurs.** Leur anonymisation dans la version proposée ne les retire pas de Git. Des chemins d’exploitation et informations personnelles existent aussi dans la documentation interne ; celle-ci n’est pas une vitrine publique validée.

Deux stratégies possibles, à approuver explicitement avant publication :

1. Préparer un état initial public assaini, **sans historique privé**, en gardant intégralement le dépôt de travail et ses archives privés. Choisir un dépôt de publication distinct évite d’exposer les anciennes branches et tags.
2. Assainir l’historique du dépôt existant, toutes les branches/tags concernés et les copies distantes. C’est une opération coordonnée avec sauvegarde et accord des autres chantiers ; ne pas la réaliser implicitement ni forcer un push.

Dans les deux cas, relire et rescanner l’artefact exact retenu, vérifier les droits de licence/visuels et les fichiers d’exploitation exclus, puis seulement décider de la visibilité GitHub. La préparation actuelle ne certifie ni les références distantes non récupérées, ni les objets de reflog inaccessibles, ni les fichiers non suivis. Le chantier web et les versions livrées restent intacts.

Voir [l’option GitHub Pages](GITHUB-PAGES.md), qui ne publie qu’un sous-ensemble explicitement autorisé ; sa validation n’approuve pas automatiquement celle du dépôt complet.
