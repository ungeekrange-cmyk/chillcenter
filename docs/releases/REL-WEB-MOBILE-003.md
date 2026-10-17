# Web mobile R3 — AVI MP3 et commandes mobiles

**Français** · [English](REL-WEB-MOBILE-003.en.md)

2026-10-17. Correctif exclusivement web. Client Mac, binaire serveur, API,
configuration et NAS inchangés. **Préparé localement, non déployé** : le Pi
reste sur R2 tant qu'une livraison de ces assets n'est pas autorisée/exécutée.
Aucun push GitHub ou changement de visibilité.

## Modifications

- AVI MP3 : repli audio AAC-LC stéréo dans le Worker FFmpeg existant quand
  le navigateur refuse les deux formes MP3 natives. Aucun transcodage serveur.
  Le chemin MP3 natif reste utilisé quand il est accepté.
- Vignettes de catalogue mobile sans actions superposées : toucher ouvre la
  fiche ; actions desktop et boutons de fiche conservés.
- Plein écran mobile demandé dans le geste, avant la file réseau ; API vidéo
  iPhone utilisée lorsque le plein écran du panneau est désactivé/absent.
  Avant préparation de la vidéo, une nouvelle touche peut être nécessaire.
- Recherche conservée entre catégories : signalement non reproduit ; couverture
  E2E ajoutée qui vérifie la requête et les cartes filtrées, même avant debounce.

## Version et archive immuable

| Élément | Valeur |
| --- | --- |
| Source | `acd7aebfc3c9b902eb7b4b414a14750d86017e08` |
| Build | `a15795b5c4cd` |
| Archive locale | `artifacts/web/chillcenter-web-mobile-acd7aeb.tar` |
| SHA256 archive | `34a1663eda23e972ecef6e8b68ac7641432465f8d751eeaea0de8bffe9316bda` |
| Taille / manifeste | 6 683 648 octets ; 82 fichiers dont 81 manifestés |
| WASM audio SHA256 | `5075105bfdfcde924be0163a785a9d5fa0d2f1eb2c0d52aecf5ea571497d47d5` |

Le WASM audio passe de 967 016 à 1 011 837 octets (+44 821). FFmpeg 9.0.2,
source et image Emscripten épinglées inchangées ; ajout du seul décodeur MP3,
compilation sans réseau, licence LGPL, aucune dépendance JavaScript ajoutée.
La VM de compilation locale, initialement arrêtée, a été démarrée puis arrêtée ;
contexte Docker initial restauré après compilation.

## Vérifications exécutées

- `node --test ui/connected/*.test.js ui/connected/tests/*.test.js web/test/*.test.js` : **311/311 PASS**, aucun skip.
- Trois parcours du build : `scripts/test-web-mobile.cjs`,
  `scripts/test-web-parity.cjs`, `scripts/test-web-mobile-playback.cjs` : **PASS**.
  Données/vidéos synthétiques locales, aucun fichier NAS ou profil réel.
- MP3 : vrai WASM, corrélation audio >0,97 contre FFmpeg indépendant, durée
  complète, fragments multiples, trames réparties entre chunks AVI, seek/close
  tardifs rejetés, chemin natif conservé. Le parcours navigateur force le refus
  du MP3 natif pour exercer réellement l'AAC produit par le Worker.
- Plein écran réel Chromium et appels natifs iPhone simulés pour l'ordre et
  l'état ; aucun succès Safari physique déduit de cette simulation.
- Diff des chemins Mac/serveur/communs vide ; aucune nouvelle adresse privée,
  clé ou certificat dans les lignes ajoutées contrôlées.

[Revue indépendante source/archive](../reviews/mobile-web-mp3-r3-20261017.md).
[ADR0028](../architecture/decisions/ADR-0028-web-mp3-fallback.md),
[BUG0110](../debug/BUG-0110-iphone-web-playback.md),
[BUG0111](../debug/BUG-0111-mobile-posters-fullscreen-search.md).

## Limites et prochaine validation

Retester physiquement les AVI/MKV signalés sur Safari/Vivaldi iPhone,
notamment le son, les sauts, la pause et le bouton plein écran. **DEVICE_RETEST_PENDING**.
Un Xvid rendu sur canvas ne peut pas entrer dans le plein écran vidéo natif
si Safari n'offre pas le plein écran des panneaux : lecture conservée dans la
zone disponible et limite affichée, sans faux état plein écran. Ce lot ne
prétend pas avoir supprimé cette restriction de plateforme.

Une éventuelle installation remplace uniquement `web/` après sauvegarde privée
et vérification des hashes protégés ; restauration des anciens assets pour
retour arrière, sans migration ni redémarrage du serveur. Utiliser alors
`/?v=a15795b5c4cd` pour éviter une page antérieure en cache.
