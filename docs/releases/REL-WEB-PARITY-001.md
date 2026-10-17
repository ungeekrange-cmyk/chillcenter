# Candidat Mac/web 0.6.1 — preuves de remise

**Français** | [English](REL-WEB-PARITY-001.en.md)

Voir [périmètre, vérifications et retour arrière](../implementation/WEB-PARITY-20261017.md).

## Source et artefacts

Source compilée : **a13902ea73aa51fb09977f1e6649e252ff39d984**, merge des bases
3b2e28b et e6e23db. Version interne **0.6.1 / build 8**, sans nouveau tag.

- Mac Apple Silicon : `artifacts/macos/Chill Center-test-a13902e.app`, 81 Mio.
  SHA256 de l'exécutable :
  `84524a7e67f4dc743797a82d969d1cc9b0a3a2c0671cec993ffbd02c19241b08`.
- Web : `artifacts/web/chillcenter-web-test-a13902e.tar`, 6 587 392 octets,
  build **1cce6dd6cd9d**, 79 fichiers avec manifeste d'intégrité.
  SHA256 de l'archive :
  `32176d8fabb110d0889187a7bcbed7ad19ee1d8a25530043fc6a714b2dc885e4`.

Les deux artefacts ont été copiés dans le projet canonique avec des noms neufs,
sans écraser une livraison. Hash Mac source/copie identique, copie web identique.

## Vérifications exécutées

- **265/265 tests Node**, **22/22 contrôles documentation/publication** PASS.
- DOM connecté ciblé : conflits média/bibliothèque, saisie conservée, relecture
  explicite, fermeture pendant réponse différée, icônes/taille/centrage,
  synchronisation non bloquante et réduction des animations PASS.
- Web construit avec son vrai pont et IndexedDB contre API synthétique :
  homonymes, TMDB, import d'image dans fiche ouverte, configuration, absence
  d'IA et langue persistée PASS. Pas un essai réseau domestique.
- Compilation Rust release verrouillée et hors ligne, signature ad-hoc profonde
  stricte PASS, **48 dylibs ARM64** relocalisées, libmpv client API **2.5** chargé.
  Deux avertissements dead_code déjà connus, aucun échec de compilation.
- [Revue indépendante finale APPROVED_SCOPED](../reviews/web-parity-20261017.md).
- Diff courant vérifié sans nouveau secret/certificat privé ni adresse du foyer
  détecté ; exemples web importés anonymisés. Pas de réécriture d'historique ni
  prétention à un nouvel audit exhaustif de tout l'historique privé.

## Préservation et limites

La v0.6 publiée demeure inchangée : exécutable
`7eb9dd979f473f1dc09309bdc314fe93c9890d8eabf6a42203c7a5232787e3f7`,
tag v0.6 à69da608. L'autre worktree web reste àe6e23db, propre.
**Aucun accès/déploiement Pi/NAS, aucune publication GitHub ou changement de visibilité.**
L'API datée2026-10-14 et `/api/v1` restent inchangées.

Restent à valider humainement : rendu et confort dans Tauri, navigation en
navigateur réel avec HTTPS/confiance certificat, lecture et ressources sur le
réseau domestique. Les différences codecs/plein écran/permissions du navigateur
ne sont pas assimilées à libmpv. Pour tester le web : servir le contenu `web/`
de l'archive depuis une instance HTTPS ; ne pas ouvrir `index.html` en `file://`.

Retour arrière : reprendre le bundle v0.6 conservé. La distribution web courante
du Pi n'a pas été changée, donc aucune restauration distante n'est nécessaire.
