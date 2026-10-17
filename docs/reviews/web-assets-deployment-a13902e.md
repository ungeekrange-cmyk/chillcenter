# Revue indépendante — déploiement des seuls fichiers web

17 octobre2026, instance `/root/web_assets_operator_review`, distincte des auteurs,
du déployeur et du vérificateur post-livraison. Rapport transcrit par le coordinateur.

**APPROVED_SCOPED**, aucun défaut bloquant pour :

- Source a13902ea73aa51fb09977f1e6649e252ff39d984, porteur documentaire a0ba4e2.
- Opérateur `scripts/pi-update-web-assets-operator.sh`, SHA256
  `b0baea0b2a572b65c9a79cd792276143b45092dc95523eb47815b8742c743996`.
- Archive web SHA256
  `32176d8fabb110d0889187a7bcbed7ad19ee1d8a25530043fc6a714b2dc885e4`,
  build `1cce6dd6cd9d`.

Le reviewer a vérifié85membres :79fichiers réguliers et6répertoires, tous sous
`web/`, sans chemins absolus, remontée, liens ou nœuds spéciaux. Les78entrées du
manifeste couvrent exactement les fichiers hors manifeste ; contrôle strict des
SHA PASS. Une reconstruction temporaire indépendante produit le même build et
une arborescence identique hors manifeste de packaging.

`bash -n` et `--self-test` PASS. Par rapport àe6e23db, le seul changement de
l'opérateur est le message d'exemple d'une variable obligatoire, ligne44, et non
sa logique. Les31tests de simulation VM jetable historiques restent applicables
à ce delta ; ils n'ont pas été rejoués. Ne jamais exécuter la simulation sur le Pi.

L'opérateur ne modifie que les fichiers web préparés/installés et le retour
arrière privé ; ni arrêt/redémarrage de service, ni écriture du binaire, de la
configuration, de l'unité, des certificats ou du NAS. Ses contrôles post-bascule
exigent le même PID et les mêmes empreintes binaire/configuration/unité/SPKI.
Un échec ou signal géré restaure l'ancien répertoire web.

Note opérationnelle : le diagnostic de ligne44 cite un DNS, mais le validateur
de ligne46 exige une IPv4. Les valeurs d'exécution doivent respecter ce contrôle ;
aucune adresse réelle n'est ajoutée au code commité.

L'approbation couvre uniquement cette archive, cet opérateur, la sauvegarde
privée préalable et le remplacement des fichiers web, sans redémarrage. Le
reviewer n'a pas accédé au Pi et n'a modifié ni source ni installation.
