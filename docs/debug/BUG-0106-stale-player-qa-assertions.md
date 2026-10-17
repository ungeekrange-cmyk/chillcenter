# BUG-0106 — deux faux échecs des contrats QA du lecteur

17 octobre 2026, auteur `/root`, branche `codex/qa-player-contracts`, base
`c071b42` (produit15c2d63). Statut : FIXED / VERIFIED_LOCAL_QA.

## Symptômes et diagnostic confirmé

`node --test ui/connected/player-qa.test.js` : 1/3 PASS sur la base.

- Le test des phases comptait globalement les littéraux `step="` et exigeait4.
  Le script possède3points d’entrée et3initialisations : échec3!==4, sans
  démontrer une absence de trace. Hypothèse confirmée par lecture et reproduction.
- Le test natif cherchait exactement `matches!(mode.as_str(),"player"|"next_series")`.
  Le code prépare déjà ces modes et `detail_manual` ; la recherche renvoyait−1.
  La préparation et son retour anticipé en cas d’erreur sont bien présents.

## Correction ciblée

Uniquement `ui/connected/player-qa.test.js`, pas le lecteur, les scripts QA exécutés
dans Tauri ou le lanceur Rust. Chaque fonction QA réelle est inspectée séparément :
initialisation et setter tracé, sans compter les helpers voisins. Le contrat du
lanceur vérifie les modes requis, l’ordre avant évaluation et l’arrêt en cas d’erreur,
tout en tolérant espaces/nouvelles lignes/modes supplémentaires. La préparation
native reste debug-only et doit démasquer, activer, montrer et focaliser la fenêtre.

## Preuves et contre-preuves

- Suite ciblée : **7/7 PASS**, contre1/3sur la base.
- Mutations en mémoire : ajout d’un helper sans effet ; phase non tracée refusée ;
  mode requis manquant, préparation supprimée/tardive ou focus manquant refusés ;
  formatage différent et mode supplémentaire acceptés.
- Exécution VM du préfixe réel des3modes : traces phase/temps/visibilité vérifiées.
  `player` et `next_series` attendent l’événement de visibilité sans timer ni boot
  caché. Le boot reste volontairement suspendu : aucun réseau, profil, scan ou média.
- Suite Node complète `ui/connected/*.test.js ui/connected/tests/*.test.js` :
  **130/130 PASS**, aucun ignoré. Pas de modification destinée à masquer un échec.

La preuve couvre les contrats QA, pas un nouveau test physique AppKit/vidéo/NAS.
Les validations humaines restantes du lot15c2d63 restent distinctes.
Aucun accès/déploiement Pi, fichier ou worktree du client web modifié.
Rollback : ancien bundle15c2d63 conservé, aucune migration ni réglage à restaurer.
