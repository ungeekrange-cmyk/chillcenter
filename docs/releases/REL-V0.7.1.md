# Chill Center v0.7.1 — notes de version

**Français** | [English](REL-V0.7.1.en.md)

Statut : candidat en vérification, pas encore publié. Les manifestes signés et
la provenance de la release fixeront la source exacte et les empreintes.

- Mac : découverte Bonjour/mDNS du serveur au lancement, recherche bornée à
  quatre secondes. Un serveur enregistré fonctionnel conserve la priorité.
  La saisie manuelle s'ouvre par « Vous ne trouvez pas votre serveur ? ».
  Une annonce n'approuve jamais un certificat : la confiance HTTPS reste exigée.
- Serveur Linux : annonce Bonjour après ouverture de l'écoute HTTPS. Nouvelle
  installation : AF_NETLINK autorisé uniquement pour lire les interfaces,
  utilisateur non privilégié et capabilities toujours vides.
- API unique `/api/2026-10-14` ; les anciens chemins `/api/v1` répondent 404.
  Les anciens chemins d'affiches stockés sont normalisés sans effacer le cache.
- Ambiances Pâques et Saint-Valentin approuvées, variantes claires/sombres,
  pétales et silhouettes de lapins. Pâques grégoriennes annuelles : J−30 ;
  Saint-Valentin : J−14. Retour à la saison le lendemain. Packs de rendu 2,
  anciens packs de rendu 1 conservés ; aucune exécution de code depuis un pack.
- Durcissement du lecteur de releases (identifiants fixes, pas de regex issue
  d'un argument) et minimum TLS explicite dans le banc d'essai Linux.

Les quatre composants sont versionnés 0.7.1 pour la distribution. Le paquet web
intègre les ambiances, mais **le web actuellement installé au foyer n'est pas
remplacé** par l'opération serveur autorisée : sa version0.7.0 reste compatible
avec l'API datée. Ni NAS, ni configuration applicative, ni identité TLS modifiés.

Mac : DMG Apple Silicon, macOS27+ requis par les bibliothèques embarquées,
signature ad hoc et Minisign, pas de notarisation Apple. Aucun transcodage
serveur ; usage réservé au réseau local de confiance, sans authentification.
L'installation et le consentement Réseau local macOS restent des actions humaines.

Contrôles détaillés : [livraison](REL-0.7.1-delivery.md),
[ressources](REL-0.7.1-resource-final.md), [sécurité](REL-0.7.1-security-final.md).
La publication GitHub et l'installation Pi sont deux opérations distinctes ;
leurs résultats effectifs sont consignés séparément, sans déduire un succès
d'un simple build ou test simulé.
