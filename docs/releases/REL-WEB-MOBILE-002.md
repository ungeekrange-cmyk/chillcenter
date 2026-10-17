# Web mobile R2 — contenu et lecture iPhone

**Français** · [English](REL-WEB-MOBILE-002.en.md)

2026-10-17. Correctifs demandés après essai du client web sur iPhone, Safari
et Vivaldi, en HTTP comme en HTTPS. Aucun bundle Mac, binaire serveur,
configuration, certificat, API ou NAS modifié. Aucun push GitHub.

## Corrections

- Catalogue visible immédiatement : recherche et filtres derrière une loupe.
  Changer de rubrique les replie sans effacer les choix ; les filtres du
  sélecteur d'ajout restent indépendants.
- Paysage téléphone : catégories dans un rail gauche, en-tête compact.
- Outils TMDB et détails de saisons/série après les épisodes, repliés par défaut
  sur mobile ; ordre et comportement desktop restaurés au redimensionnement.
- Repli moteur `ManagedMediaSource` quand MediaSource classique est absent.
  Pour ce repli, lecture distante désactivée avant attachement, comme requis
  par [WebKit](https://webkit.org/blog/14735/webkit-features-in-safari-17-1/).
- Refus de démarrage : bouton tactile sur le lecteur préparé, sans nouveau bail
  ni rechargement. Respect des pauses serveur et baux invalides. Guidance de
  rotation absente en paysage ; plein écran vidéo demandé dans le geste quand
  le navigateur ne propose pas le plein écran du panneau.

## Provenance et preuves

| Élément | Valeur |
| --- | --- |
| Source | `8b950334a17641aaa1a20e2129ccfe9a449b813a` |
| Build | `95ebee4fbe0a` |
| Archive | `artifacts/web/chillcenter-web-mobile-8b95033.tar` |
| SHA256 | `0da5d3962cbccabc966f71f1b9901aa61c4a2ef74508bb82ba1909694a1d7a50` |
| Taille / manifeste | 6 628 864 octets ; 81/81 hashes, 82 fichiers avec manifeste |

**304/304 tests unitaires PASS** ; trois parcours Chromium PASS : interface mobile
(320/390/430/736/844px), régression web desktop, lecture réelle de clips synthétiques
MKV H.264/AAC et AVI Xvid/MP3. Le constructeur MSE Chromium est volontairement
exposé sous le nom ManagedMediaSource pour contrôler le repli ; aucune API
Apple native ni vidéo du NAS n'est utilisée. Le rejet initial est simulé,
la reprise et le décodage des images sont réels dans Chromium.

[Revue indépendante source et archive](../reviews/mobile-web-content-first-20261017.md) :
APPROVED_SCOPED pour la source et l'archive. [ADR0027](../architecture/decisions/ADR-0027-mobile-content-and-iphone-playback.md),
[BUG0109](../debug/BUG-0109-mobile-content-obscured.md), [BUG0110](../debug/BUG-0110-iphone-web-playback.md).

## Installation

Assets web seuls remplacés le 17 octobre 2026 à **18:07:25 UTC** ; build précédent
`b1722f00c0a6`. Sauvegarde privée préalable, permissions `0700` :
`/root/chillcenter-rollback/predeploy-mobile-r2-20261017T180658Z`.
Ancien arbre conservé : `/root/chillcenter-rollback/web-20261017T180724Z`.
Le retour arrière consiste à restaurer ces seuls assets avec l'opérateur dédié,
sans remplacer le serveur ou sa configuration.

[Vérification indépendante](REL-WEB-MOBILE-002-verification.md) à 18:08:14 UTC :
**APPROVED_SCOPED**, 81/81 sommes installées et sauvegarde vérifiées, HTTP/HTTPS et
API datée/v1 accessibles. PID `5420` inchangé, zéro redémarrage ; binaire,
configuration, unité, certificat et SPKI inchangés. Seule l'archive de transfert
temporaire a été supprimée après vérification de son hash ; sauvegardes, ancien
arbre et archive locale conservés.

## Retest iPhone requis

Fermer l'ancien onglet et ouvrir `/?v=95ebee4fbe0a` sur le serveur pour éviter
une ancienne page en cache, puis retester les deux vidéos signalées, en
portrait et paysage. Si le navigateur bloque le démarrage, toucher le bouton
de lecture proposé. Le cas iPhone reste **DEVICE_RETEST_PENDING**, pas CLOSED.

La compatibilité du conteneur ne garantit pas celle du codec audio : un AVI
Xvid/MP3 peut rester illisible si Safari refuse le MP3 via MediaSource/MMS ;
le transcodeur existant ne convertit pas MP3 en AAC. Aucune nouvelle prise en
charge de ce codec ni réussite physique Safari n'est prétendue par ces tests.
