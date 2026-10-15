# BUG-0062 — séparateurs et suffixes audio/vidéo conservés dans les titres

2026-10-15, base `eb73711`, branche `codex/filename-parsing`. État : FIXED_LOCAL, NON_LIVRÉ.

## Symptômes et cause

Exemples fournis par l'utilisateur comme titres affichés, sans extension : `Attack the Block - - Vosta`, `Birdman () - vost DDP 5 1`. Les extensions et chemins des fixtures sont synthétiques, pas des fichiers du NAS inspectés.

H1 confirmée : le scanner ne reconnaissait ni VOSTA/VOST ni DDP, gardait les séparateurs internes et les parenthèses vides. Son extraction de la dernière année avant nettoyage pouvait prendre une année technique pour l'année de sortie. Les anciens titres déjà indexés étaient transmis tels quels à la recherche et comparés tels quels au résultat TMDB.

## Correction

- Normaliser points/underscores avant reconnaissance des tags, supprimer les séparateurs répétés et wrappers vides, couper les champs audio/vidéo reconnus avant extraction de l'année du fichier.
- Conserver les tirets des titres, les parenthèses de sous-titres et les mots ordinaires comme `French` sans contexte technique. Reconnaître aussi les codecs et langues des fixtures ; ce n'est pas une reconnaissance universelle de tous les noms possibles.
- Réutiliser `title_for_search` pour les recherches automatiques et la comparaison stricte des anciens titres. Cette fonction ne réinterprète pas les nombres d'un titre déjà enregistré comme une année : `Blade Runner 2049` reste entier. Une requête saisie explicitement reste inchangée hors trim existant.
- Conserver le refus des résultats ambigus, d'une autre année ou d'une autre catégorie. Pas de changement de l'API datée `2026-10-14`, de dépendance ni de schéma.

Aucun renommage/déplacement de vidéo, aucune réécriture des corrections manuelles ou des anciennes fiches. Un rescan protège toujours les métadonnées existantes ; le nettoyage de la requête permet de rechercher leurs titres sales sans migration destructive. L'égalité français/titre original reste un sujet distinct, non résolu ici.

## Preuves

Avant correction : trois groupes scanner échouent sur les fixtures de séparateurs, mots du titre et épisodes. Après : PASS. Deux gardes ajoutées pendant la vérification ont également échoué dans le candidat intermédiaire : underscores entre tags et suppression de `2049` dans une requête ; toutes deux corrigées avant livraison.

Commandes (runtime/cache Cargo existants, même target temporaire réutilisée) :

```sh
cargo test --locked --offline -p chill-catalog -p chill-domain --lib
cargo test --locked --offline -p chill-server --lib exact_match
cargo clippy --locked --offline -p chill-catalog -p chill-domain -p chill-server --all-targets -- -D warnings
```

Résultats : 74 catalogue + 54 domaine + 2 tests serveur PASS ; Clippy PASS sans warning. Les nouveaux cas couvrent les deux exemples, MKV/MP4/AVI, années, codecs, séries S02E03, vrais tirets et parenthèses. Aucun appel TMDB réel, scan NAS ou test lecteur effectué. Retrait du patch Git suffit pour revenir au comportement précédent : aucune donnée migrée. Pi et bundle Mac inchangés.
