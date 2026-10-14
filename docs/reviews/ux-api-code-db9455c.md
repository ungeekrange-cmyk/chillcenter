# Revue delta indépendante — catalogue continu et oracle QA lecteur natif

- Verdict code : **APPROVED**
- Candidat exact : `db9455cf202a59be7ec17079ab3ad75cbcb07d44`
- Base : `0a7e8f300d0e1f1b0723cf91d6aade15412027f5`
- Auteur(s) du delta : builders distincts ; reviewer : `/root/ux_api_code_review`
- Revue UTC : `2026-10-14T16:58:39Z`
- ADR-0011 : contenu inchangé, approbation précédente conservée sur SHA-256 exact `4e8456c60c1cc980a246f8f7192a15d642affce57a67346760ff49f49a9acaff`
- Périmètre : delta tests catalogue, harnais QA natif et rapports historiques. Aucun changement serveur, API, modèle, lecteur de production, CSS, AppKit, dépendance ou état persistant.

## Findings

Aucun finding code ou maintenabilité ouvert sur le candidat exact.

## Analyse du delta

### Catalogue continu

Les quatre tests ajoutés exercent désormais la fonction réellement livrée `loadPage`, et non seulement le calculateur de fenêtre :

- chargement de 103 titres par offsets `0/48/96`, sans trou ni doublon, compteur final correct et accès au dernier titre par la fenêtre DOM bornée (`ui/connected/catalog-loading.dom.test.js:19-30`) ;
- déduplication d'un lot chevauchant sans perte des éléments suivants (`:31-35`) ;
- isolation d'une réponse tardive après changement de profil (`:36-42`) ;
- isolation d'une réponse tardive après nouvelle recherche sur le même profil (`:43-48`).

Le harnais exécute le corps livré de `app.js` dans un DOM minimal et utilise le vrai `catalogWindow`; il vérifie les appels à l'API et l'état rendu. Il reste volontairement un test composant synthétique, mais ferme correctement la réserve de couverture Node sur le multi-batch et les contextes périmés. Aucun comportement produit n'est modifié par ce commit.

### Oracle et traces du lecteur natif

Le diagnostic indépendant démontre que l'ancien échec de hit-test était une course avec l'auto-masquage normal après 2,5 secondes : le même point passe de `#native-video` au bouton après l'appel existant `showPlayerControls()`. Le correctif respecte exactement cette cause : il révèle synchroniquement les contrôles et vérifie l'absence de `controls-hidden` immédiatement avant de mesurer puis d'appeler `elementFromPoint` (`ui/connected/player-qa.js:23-26`). Il ne retire ni n'affaiblit le hit-test, et conserve ensuite les assertions d'inactivité, pointeur et focus (`:26-28`). Aucun fichier du lecteur de production ou de la couche native n'est touché.

Les transitions de phase des trois modes passent maintenant par un helper borné qui n'émet que mode, phase, durée et `document_hidden` (`player-qa.js:4-6`, `49-50`, `57-58`). Le command handler traite explicitement le préfixe `TRACE ` comme non terminal, tronque la sortie à 300 caractères et n'appelle pas `app.exit` dans ce cas (`apps/desktop/src/main.rs:650-667`). Il n'y a donc ni faux échec terminal, ni secret/URL/identité ajouté aux traces.

Le test de régression impose l'ordre causal phase → révélation → rectangle → hit-test et interdit les réassignations silencieuses de phase (`ui/connected/player-qa.test.js:1-12`). Son assertion source est volontairement étroite, mais cohérente avec un harnais debug-only de faible surface.

Le timeout historique dans la capture framebuffer n'est ni expliqué ni déclaré corrigé par ce changement. Les traces ajoutées améliorent sa prochaine localisation sans masquer ce statut ; il reste du ressort du reviewer tests et, s'il se reproduit, d'un cycle de diagnostic séparé.

## Vérifications exécutées

- `git rev-parse HEAD` : candidat exact ; `git status --short` : propre.
- `git diff --check 0a7e8f3..db9455c` : PASS.
- `node --check ui/connected/player-qa.js` : PASS.
- `node --test ui/connected/*.test.js ui/connected/tests/*.test.js` : **45 PASS**, 0 échec/skip.
- Lecture du diagnostic `/private/tmp/chillcenter-ux-native-diagnostic.md` : hypothèse H1 confirmée par traces naturelles et expérience causale ; source restaurée avant handoff.
- Lecture du rapport builder `/private/tmp/chillcenter-ux-native-qa-builder.md` : périmètre causal respecté et limites déclarées.

## Évidence réutilisée et limites

- Les rapports code, sécurité et tests de `0a7e8f3` sont ajoutés comme historique. Leur contenu n'est pas réécrit rétroactivement ; le delta présent est relu indépendamment.
- L'ADR-0011 et son contrat API ne changent pas. Son approbation précédente reste attachée au hash documentaire exact indiqué ci-dessus ; ce rapport ne prétend pas une nouvelle décision d'architecture.
- Les deux nouveaux parcours Tauri natifs surveillés sont des preuves runtime séparées et étaient encore en cours au moment de cette revue. **Aucun PASS natif n'est revendiqué ici.** Leur verdict appartient au reviewer tests.
- Pi/NAS, Wi-Fi, deux vidéos Full HD, trois flux, fournisseur TMDB/IA réel, RSS physique, codecs/HDR, son humain, veille et multi-écran restent hors de cette revue.

## Disposition

Le code et les tests du delta sont approuvés pour `db9455cf202a59be7ec17079ab3ad75cbcb07d44` uniquement. La promotion exige encore le verdict indépendant tests applicable à ce SHA et les autres gates de livraison courants. Cette approbation n'est transitive vers aucun SHA ultérieur et ne vaut ni PASS Tauri natif, ni validation Pi/NAS, ni autorisation de déploiement.
