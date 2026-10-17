# Web mobile R4 — bandeau éphémère et zones caméra

**Français** · [English](REL-WEB-MOBILE-004.en.md)

2026-10-17. Correctif web ciblé après retour Safari. Aucun changement Mac,
serveur, API, NAS, dépendance ou moteur audio. Question apps natives traitée
en faisabilité seulement : aucune app iOS/Android native créée ou signée.

## Version et preuves

- Source `fa13d447abd4c35e9ba1f14f85ef05eaa2b25070`, build `67773f7877d3`.
- Archive conservée : `artifacts/web/chillcenter-web-mobile-fa13d44.tar`.
- SHA256 `c139dca29fe63c5bb9ed016b8ac7f180b83f8f01475ff379be8eb298dba80182` ;
  6 687 744 octets, 82 fichiers dont 81 manifestés.
- **315/315 tests Node PASS**, QA du build mobile et parité desktop web PASS.
  Délai réel du bandeau contrôlé dans Chromium ; timer déterministe sous poll,
  resize/langue et fermeture ; entrées WebKit simulées, insets gauche/droite
  injectés pour vérifier la géométrie. Pas de nouvelle lecture NAS.
- [Revue indépendante source/archive](../reviews/mobile-web-fullscreen-r4-20261017.md).

## Correctifs

Le message plein écran disparaît après **2,5 secondes**, sans réapparition sur
les états du lecteur. Seule une nouvelle tentative peut le réafficher brièvement.
Le plein écran vidéo essaie aussi `webkitSetPresentationMode("fullscreen")`
lorsque l'API précédente échoue ; l'API standard demande de masquer la navigation.
L'état retourné reste l'état réel, sans boucle de forcing ni faux succès CSS.

Les quatre marges système protègent vidéo, commandes, fiches et rail paysage des
encoches/Face ID/poinçons ; le fond reste bord à bord. Un arrondi flottant qui
faisait parfois perdre une colonne après redimensionnement a été reproduit et
corrigé uniquement dans l'adaptateur web. [BUG0112](../debug/BUG-0112-persistent-fullscreen-notice-cutouts.md).

## Installation et limites

Paquet préparé ; l'installation et la vérification après sauvegarde sont
consignées ici une fois exécutées. Retour arrière : anciens assets web conservés,
aucune migration et aucun redémarrage nécessaires.

**DEVICE_RETEST_PENDING** : Safari physique, plein écran selon codec/API et vraies
zones caméra restent à confirmer. Un canvas Xvid peut toujours être refusé par
le plein écran natif ; aucun contournement de cette restriction n'est prétendu.
Fermer l'ancien onglet et utiliser `/?v=67773f7877d3` après installation.
