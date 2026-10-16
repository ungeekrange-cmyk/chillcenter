# BUG-0076 — Écrasement local via un lien physique de sortie

16 octobre 2026, construction locale du site ; **VERIFIED_LOCAL**, non publié. Revue sécurité de `df95432`, identifiant `SEC-PUBLIC-001`, LOW : un fichier autorisé préexistant dans `dist/public` peut être un lien physique vers un autre fichier du même utilisateur. L’écriture en place modifie alors cet autre inode. Reproduction indépendante isolée confirmée ; ce n’est pas une exposition sur un runner GitHub neuf.

Correction `2cb6f73` : refus des fichiers de sortie avec `nlink > 1`, y compris le manifeste, avant toute écriture. Régression reproduisant le lien et vérifiant les deux contenus inchangés : PASS. Revue sécurité et code indépendantes : APPROVED au SHA `2cb6f730e00bb344a0eba36e6220c8076415296a`.

Les mutations concurrentes des chemins de sortie par le même utilisateur sont hors modèle et explicitement interdites pendant une construction. Retour arrière : revenir au parent sans exécuter le constructeur sur un répertoire non vérifié ; aucun fichier utilisateur n’a été supprimé pour la correction.
