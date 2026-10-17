# Livraison — client web smartphone

**Français** · [English](REL-WEB-MOBILE-001.en.md)

Date : 2026-10-17. Concept R1 et déploiement des seuls assets web autorisés
explicitement par l'utilisateur. Cette livraison ne change pas le tag `v0.6`,
les bundles Mac ni la visibilité GitHub. Aucun push GitHub dans ce lot.

## Contenu livré

- Navigation basse Films, Séries, Docs, Spectacles et Plus sur smartphone.
  Plus ouvre toute la collection, les sagas, les listes et la configuration.
- Logo, profil avec nom/initiale en haut à droite et recherche accessible.
  Filtres genres/années, fiches et configuration adaptés au tactile.
- Grille virtualisée sans pagination : deux colonnes jusqu'à 360px, trois
  au-delà sur téléphone ; dernier rang dégagé de la navigation et des zones sûres.
- Demande du vrai plein écran dans le geste Lire, puis demande paysage selon
  les capacités du navigateur. Refus explicite : indication de rotation et
  possibilité de réessayer, sans bloquer ni relancer la vidéo.
- Navigation et décors masqués pendant la lecture ; retour après fermeture,
  y compris après redimensionnement. Décors Noël/Halloween alignés au panneau.
- Langues FR/EN, thèmes et profils conservés. Toujours aucune IA sur le web.

L'affichage desktop web garde sa barre latérale. Les sources communes
`ui/connected`, les sources Mac, le serveur et les contrats API sont inchangés.
L'API datée `2026-10-14` et l'alias historique `v1` restent disponibles.

## Provenance immuable

| Élément | Valeur |
| --- | --- |
| Commit de fonctionnalités | `98ee3e37e8b19c0d2e8e92991b3fb917eb0a13b8` |
| Build web | `b1722f00c0a6` |
| Ancien build | `1cce6dd6cd9d` |
| Archive conservée | `artifacts/web/chillcenter-web-mobile-98ee3e3.tar` |
| SHA256 archive | `281cf6eb6babca616004f425ac9e13332f38ef9a3af59f7793398640c81cc0ed` |
| Taille | 6 616 576 octets |
| Manifeste | 81/81 hashes valides ; 82 fichiers avec `SHA256SUMS` |
| Opérateur assets seulement | `b0baea0b2a572b65c9a79cd792276143b45092dc95523eb47815b8742c743996` |

## Preuves et périmètre

- `node --test ui/connected/*.test.js ui/connected/tests/*.test.js web/test/*.test.js` : **283/283 PASS**.
- `scripts/test-web-mobile.cjs` : vrai client construit avec API synthétique,
  320/390/430/736/844px et retour 1280px, filtres, défilement conservé,
  collections non vides, configuration, FR/EN, cibles tactiles, ambiances,
  vrai plein écran Chromium et retour de navigation : **PASS**.
- `scripts/test-web-parity.cjs` : régression des fiches TMDB, séries homonymes,
  sélecteur d'image, configuration/langues et exclusion IA : **PASS**.
- [Revue indépendante source et archive](../reviews/mobile-web-responsive-20261017.md) : **APPROVED_SCOPED**.
- Contrôles documentation FR/EN, liens, confidentialité et publication : **24/24 PASS**.
- [Décision ADR0026](../architecture/decisions/ADR-0026-mobile-web-presentation.md)
  et [BUG0108](../debug/BUG-0108-mobile-web-lifecycle.md) : écarts trouvés et corrigés avant livraison.

Les données des tests locaux sont fictives : elles ne prouvent pas la lecture
NAS, les performances matérielles ni la politique d'un téléphone réel.

## Déploiement et sauvegarde

Assets seuls remplacés le **2026-10-17 à 14:36:13 UTC**, opérateur rc=0.
Service resté actif, PID `5420`, zéro redémarrage.

Sauvegarde privée vérifiée avant remplacement :
`/root/chillcenter-rollback/predeploy-mobile-20261017T143553Z/web-before.tar`,
SHA256 `e340656ef3fdae44ed80aaca2e10c35a8cbdf8fb26bfade0a67f976ea470bca4`.
Ancien arbre conservé : `/root/chillcenter-rollback/web-20261017T143612Z`.
Le répertoire privé est root:root, mode 0700 ; aucun NAS parcouru ou modifié.

[Vérification indépendante post-déploiement](REL-WEB-MOBILE-001-verification.md) :
**APPROVED_SCOPED**. Les 81 hashes correspondent aux fichiers installés ; HTTPS
strict, API datée et alias v1 répondent 200. Binaire, configuration, unité,
certificat et SPKI inchangés ; aucun avertissement dans le journal borné.
L'HTTP sans port et sur le port applicatif ont aussi été vérifiés depuis le Mac.

Seule l'archive de transfert temporaire a été supprimée après vérification de
son hash. La copie locale du livrable, la sauvegarde et l'ancien arbre restent
disponibles. Les deux exécutables Mac conservés gardent leurs hashes antérieurs.

Retour arrière : après validation des cibles et de leurs hashes, remettre les
seuls anciens assets depuis cet arbre ou la sauvegarde vérifiée. Aucun rollback
de données, modification de réglages ou remplacement du binaire n'est requis.

## Essai utilisateur et limites

Recharger le client web à son adresse habituelle sur le réseau local ; si le
navigateur garde l'ancien contenu, recharger complètement la page ou rouvrir
l'onglet. Vérifier `build.json` pour identifier le build livré.

Restent à tester sur appareils réels : Safari/iPhone et Android, lecture vidéo,
rotation paysage, sortie/réentrée plein écran et zones sûres. Un navigateur peut
refuser le verrouillage ou le plein écran ; le repli reste utilisable, mais
aucune rotation forcée universelle n'est promise. Le moteur canvas ne bénéficie
pas du plein écran vidéo natif WebKit. Aucun essai de débit Pi/NAS dans ce lot.
