# BUG0148 — Accès au téléchargement depuis une fiche

21 novembre 2026 · Correction des clients web et Mac dans le code de la fiche
partagée.

Dans le client web 0.7.13 du Pi, **Télécharger** est placé sous le résumé et
les informations du fichier. Un résumé long pousse les actions sous la
zone visible. Depuis le port HTTP 80, Chromium a reproduit l’exception
`root.crypto.randomUUID is not a function` dans la commande consultative
`downloads_command:list`. Le pont générait un identifiant d’appareil pour
toute commande de téléchargement, y compris les simples lectures. Le message
du transfert était placé dans la page derrière la boîte de dialogue.

La fiche met les actions courantes au-dessus du résumé. L’édition, la note,
le chemin du fichier, l’image et TMDB sont réunis dans le volet **Modifier**
fermé par défaut, après les recommandations. La fiche série place également
ses outils et ceux des saisons après les épisodes. Le pont ne crée
l’identifiant d’appareil qu’au démarrage du transfert. Le bouton HTTP affiche
dans la fiche un lien sur le même hôte, au port HTTPS standard de Chill Center
(8443).

La régression Chromium couvre la visibilité en bureau et à 390 px, le volet
fermé, le passage HTTP vers HTTPS, la conservation du brouillon en conflit et
la lecture hors connexion d’une copie. La vérification du parcours HTTP
utilise une bibliothèque synthétique et ne démarre aucun transfert sur le Pi.
