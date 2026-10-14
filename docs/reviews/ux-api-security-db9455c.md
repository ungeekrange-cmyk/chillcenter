# Revue delta de sécurité — QA catalogue et lecteur

## Décision

- **Verdict : APPROVED**
- **Candidat exact :** `db9455cf202a59be7ec17079ab3ad75cbcb07d44`
- **Base du delta :** `0a7e8f300d0e1f1b0723cf91d6aade15412027f5`
- **Relecteur indépendant :** `/root/ux_api_security_review`
- **Date :** 2026-10-14T16:59:25Z
- **Findings ouverts dans ce delta :** 0 CRITICAL, 0 HIGH, 0 MEDIUM, 0 LOW

Cette revue qualifie le SHA exact, sans transmettre implicitement le verdict du SHA précédent. Le delta contient trois rapports historiques, deux tests Node et un script d'oracle Tauri QA. Aucun code Rust, UI de production, API, worker TMDB, TLS, fournisseur, manifeste ou lockfile ne change.

## Frontière QA / release

- `ui/connected/player-qa.js` n'est référencé ni par la page ni par le runtime web de production. Il est embarqué par `include_str!` uniquement dans la branche `#[cfg(debug_assertions)]` de `apps/desktop/src/main.rs:548-603`.
- `qa_result` et `qa_deadline_probe` ne sont enregistrés que dans le handler debug ; le handler release de `main.rs:604-611` ne les expose pas.
- L'injection QA exige les variables `CHILL_UI_QA_URL` et `CHILL_UI_QA_ROOT`. Le delta n'ajoute ni permission Tauri, ni handler, ni chemin fichier/réseau.
- `qa_result` limite déjà chaque détail à 300 caractères. Les nouvelles traces commencent toutes par `TRACE ` ; elles sont traitées comme diagnostics non terminaux et ne déclenchent pas une sortie d'application.

## Journalisation et données sensibles

Les nouvelles traces contiennent uniquement :

- une phase issue d'une chaîne littérale du script ;
- `elapsed_ms`, calculé depuis le début du parcours ;
- le booléen `document_hidden`.

Toutes les invocations de `setStep` utilisent des littéraux. Aucun profil, média, URL, chemin, token, secret TMDB/IA, erreur serveur, état de lecture personnel ou payload API n'est ajouté aux traces. Le delta ne change pas les messages d'erreur terminaux préexistants. Le balayage ciblé du SHA exact ne trouve aucun motif de clé privée ou token usuel.

## Bornes et entrées

- Le correctif d'oracle appelle synchroniquement l'API existante `showPlayerControls()`, confirme l'absence de `controls-hidden`, puis conserve le rectangle et le hit-test d'origine. Il n'assouplit ni délai, ni assertion native, ni commande lecteur.
- Les traces de phase sont émises uniquement lors de transitions finies, jamais dans les boucles de polling ; leur volume est structurellement borné par le nombre de phases du scénario.
- `catalog-loading.dom.test.js` lit uniquement `app.js` via `__dirname`, dans un `vm` Node sans réseau. Les fixtures sont synthétiques et bornées à 103 éléments ; les réponses tardives sont explicitement résolues.
- Le test catalogue vérifie offsets `0/48/96`, déduplication, contexte profil/requête périmé et DOM virtualisé. Il n'introduit aucune surface d'entrée dans l'application livrée.
- `player-qa.test.js` lit seulement le script QA local et contrôle l'ordre phase → révélation → rectangle → hit-test. Il n'exécute aucun IPC, shell ou fichier arbitraire.

## Vérifications indépendantes

| Contrôle | Résultat |
|---|---|
| `git rev-parse HEAD` / `git status --short` | SHA exact, worktree propre |
| `git diff --check 0a7e8f3..db9455c` | PASS |
| Inventaire du delta runtime/dépendances | Aucun Rust/API/TLS/provider/UI production/manifeste/lockfile modifié |
| `node --test ui/connected/*.test.js ui/connected/tests/*.test.js` | **PASS — 45/45** |
| `node --check ui/connected/player-qa.js` | PASS |
| `node --check ui/connected/catalog-loading.dom.test.js` | PASS |
| Références de `player-qa.js` | Uniquement include debug et test local |
| Audit statique des appels `setStep` | Phases littérales uniquement ; aucune donnée externe journalisée |
| Balayage ciblé clés privées/tokens usuels | Aucun résultat |

## Traçabilité et limites

- Les rapports importés décrivent honnêtement les verdicts du SHA `0a7e8f3`; ils ne constituent pas une approbation du candidat courant. Aucun credential n'y est présent.
- Le diagnostic H1 établit que l'ancien hit-test pouvait viser un contrôle volontairement masqué après 2,5 s. Le correctif est limité à l'oracle QA et ne prétend pas corriger un défaut de production.
- Le timeout historique dans `glReadPixels`/Metal reste une observation distincte non résolue. Cette approbation sécurité ne remplace pas les deux parcours Tauri frais sous watchdog, ni la revue tests du SHA courant.
- Aucun Tauri réel, provider, token, credential, Pi, NAS, SSH, lecture vidéo ou déploiement n'a été exécuté par ce reviewer.
- L'approbation est limitée à `db9455cf202a59be7ec17079ab3ad75cbcb07d44`. Toute modification supplémentaire des scripts QA, handlers debug ou sources runtime nécessite une nouvelle revue delta.

## Conclusion

Le delta améliore uniquement la couverture et l'observabilité QA. Les traces sont bornées, non sensibles et confinées au build debug ; les nouvelles fixtures sont locales et bornées. Aucune régression de sécurité n'est détectée. **APPROVED** pour la gate sécurité du SHA exact, sous réserve indépendante des gates natives/tests encore en cours.
