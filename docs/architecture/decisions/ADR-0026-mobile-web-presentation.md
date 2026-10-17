# ADR0026 — Présentation smartphone propre au client web

**Français** · [English](ADR-0026-mobile-web-presentation.en.md)

- Date : 2026-10-17. Statut : ACCEPTED, périmètre et concept R1 validés
  explicitement par l'utilisateur avant intégration.
- Complète [ADR0025](ADR-0025-web-shared-ui-parity.md), sans changer les API.
- Portée : fichiers de présentation web et génération de ses seuls assets.

La génération web ajoute des adaptateurs après les scripts communs, sans
modifier `ui/connected` ni le client Mac. Sur smartphone, la barre latérale
devient une navigation basse à cinq entrées : Films, Séries, Docs, Spectacles,
Plus. Plus donne accès à toute la collection, aux sagas, aux listes et à la
configuration. Les boutons délèguent aux actions existantes : aucune seconde
logique de catalogue, collection ou profil n'est introduite.

La grille conserve sa virtualisation et son calcul de hauteur : trois colonnes
par défaut, deux sur les écrans très étroits pour conserver des cibles tactiles
utilisables. Le calcul web adapte les colonnes et la hauteur ensemble ; une
simple surcharge CSS des colonnes désynchroniserait le défilement virtuel.
Les marges réservent la navigation et les zones sûres du navigateur.

Le lecteur demande le vrai plein écran immédiatement dans le geste Lire, avant
la réponse réseau, puis demande le verrouillage paysage si le navigateur le
permet. Un refus ne bloque pas la lecture : message de rotation et possibilité
de réessayer. Aucun paysage simulé par rotation CSS n'est présenté comme un
plein écran natif. Le repli WebKit ne concerne qu'une vraie vidéo compatible,
pas un rendu canvas. L'utilisateur peut quitter le plein écran ; les réponses
asynchrones tardives sont invalidées à la fermeture. Le tampon et les moteurs
de lecture restent inchangés.

Les tests de navigateur synthétiques ne prouvent pas la politique plein écran
d'un téléphone réel. Safari/iPhone et Android nécessitent un essai physique,
notamment en HTTP, où certaines capacités du navigateur restent restreintes.
Le web reste sans IA. Les langues, profils, thèmes et filtres existants restent
les mêmes. Aucun nouveau contrat ni migration serveur.

Livraison : sauvegarde privée des anciens assets, échange du seul dossier web,
vérification indépendante des hashes protégés et du PID inchangé. Retour arrière
par remise des anciens assets, sans arrêt de service ni modification du NAS.
