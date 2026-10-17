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

Assets web seuls déployés le 17 octobre à **20:50:15 UTC**, opérateur inchangé,
sortie `0`, build précédent `a15795b5c4cd`. Sauvegarde privée `0700` :
`/root/chillcenter-rollback/predeploy-mobile-r4-20261017T204951Z`.
TAR SHA256 `37216843cfa433d3c3982a0a77b9f815dd974af3c3143552765cb0accb7270f2` ;
manifeste `8019e0efe658c9dbd6b1c93113d7cce61ec1e17d7aea5fac72252437dbd37d54`.
Ancien arbre : `/root/chillcenter-rollback/web-20261017T205014Z`.
PID5420 et zéro redémarrage ; binaire/configuration/unité/certificat/SPKI
inchangés. Point d'entrée HTTP sans port et URL versionnée vérifiés depuis le LAN.
Retour arrière : restaurer ces seuls assets conservés, sans migration ni
modification du serveur. [Vérification indépendante](REL-WEB-MOBILE-004-verification.md).
**APPROVED_SCOPED à20:51:07UTC** : 81/81 hashes, rollback, API/ressources et
invariants validés ; aucun signal d'erreur borné. Seule l'archive de transfert
temporaire a été retirée après contrôle de son hash, sauvegardes et archives
locales conservées.

**DEVICE_RETEST_PENDING** : Safari physique, plein écran selon codec/API et vraies
zones caméra restent à confirmer. Un canvas Xvid peut toujours être refusé par
le plein écran natif ; aucun contournement de cette restriction n'est prétendu.
Fermer l'ancien onglet et utiliser `/?v=67773f7877d3` après installation.
