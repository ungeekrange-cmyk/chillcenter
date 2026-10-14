# Revue indépendante cache configurable — `624f244`

- Mode : `EXECUTION_REVIEW` + revue ADR indépendante.
- Parent / source revue : `27bd5b24b101ac7028bbd3ad8a2f81159bd40e03` / `624f2447b5c0ede6b4a053ea3fa7b82db85c0121` ; base serveur conservée `c58da7e…`/source Pi `126da0a…`.
- Extraction immuable : `/private/tmp/chillcenter-review-624f244.eRsexH`.
- ADR revue : `ADR-0010` révision `CC-CACHE-002`, SHA-256 `72d37abd8a09f368f2af9d19d071d39da7d48fe73ca8b3fc254d750bbdf3c2e5`.
- Auteur `/root`, relecteur distinct `/root/pi_integration_gate`, `2026-10-14T12:27:05Z`.
- Environnement : macOS 27.0.1 arm64 ; rustc/cargo 1.99.0 ; libmpv 0.41.0 ; FFmpeg 9.0.2 ; Node installé dans le runtime courant.

## Verdicts

- **Code/tests du SHA exact : `APPROVED`.** Les valeurs locales entières sont fermées et bornées (`16–512` Mio avant, `0–256` Mio arrière), les défauts restent `128/64`, les anciens `client.json` migrent par défaut, la sauvegarde est locale/atomique et revient à l'état mémoire précédent en cas d'échec. La valeur traverse `PlaybackConnection`/`Session` puis la création ou le reload natif. La vraie libmpv accepte des valeurs non par défaut `96/32`, leur remplacement `48/0`, et refuse `513/0` sans altérer la dernière valeur valide.
- **ADR-0010 `CC-CACHE-002` : `APPROVED` à `624f2447…`.** Les trois findings de la revue initiale sont résolus : remplacement explicite du budget historique §10 et invalidation RSS client, télémétrie/limites correctement qualifiées d'approximatives, métadonnées/succession visibles dans l'index. Le statut peut rester `PROPOSED` jusqu'à la mise à jour non sémantique du propriétaire ; ce rapport n'édite ni l'ADR ni l'index.
- **Bundle/UI réelle/RSS/NAS avec réglage configurable : `NOT_RUN`.** La charge à trois flux exécutée avec `27bd5b2` ne prouve pas la nouvelle UI ni des budgets non standards. Elle reste une preuve du défaut `128/64` uniquement dans sa portée propre.

La précédente revue `27bd5b2` demeure `code/tests APPROVED` et `ADR CHANGES_REQUESTED`; elle n'est ni effacée ni transformée rétroactivement par ce verdict.

## Preuves reproductibles

| Commande / preuve | Exit | Durée | Résultat |
| --- | ---: | ---: | --- |
| `cargo test --locked --offline -p chill-player -p chill-client` sur l'extraction immuable, hors sandbox uniquement pour deux listeners loopback | 0 | 13,89 s | 49 pass (`32 client + 17 player`), 0 fail/ignored/retry ; doctests 0. |
| Binaire de tests desktop du run final `624f244`, `/private/tmp/chillcenter-root-target/debug/deps/chillcenter_desktop-534b4c44c78529c4 --list`, puis exécution | 0 | 0,01 + 0,02 s | 20 tests listés, 20 pass, 0 fail/ignored. Le run auteur complet sur les trois paquets avait également produit 69 pass. |
| `cargo clippy --locked --offline -p chill-player -p chill-client -p chillcenter-desktop --all-targets -- -D warnings` | 0 | 11,16 s | Aucun avertissement sur les trois paquets et leurs tests. |
| `node --test ui/connected/*.test.js ui/connected/tests/*.test.js` | 0 | 0,32 s | 27 pass, dont les 3 nouveaux tests DOM cache ; 0 fail/skipped/todo. |
| Recalcul des SHA de tous les chemins de `git ls-tree -r 624f244` puis `diff -u MANIFEST.sha256` | 0 | 10,7 s | Manifeste exact (`e113e2…7bc8`). |
| `rustfmt --edition 2024 --check crates/chill-client/src/playback_cache.rs` | 0 | <0,1 s | Nouveau module conforme. |
| `git diff --check 27bd5b2..624f244` | 0 | <0,1 s | Aucun défaut d'espacement. |
| Diff de `Cargo.lock`, manifests Cargo et `crates/chill-server` | 0 / vide | <0,1 s | Aucune dépendance, lock ou source serveur modifiée. |

Un premier lancement groupé du relecteur dans `/private/tmp/video-catalog-build/target` a atteint le linker desktop puis échoué avec `ENOSPC`; aucun test produit n'avait échoué. Cette tentative est exclue des PASS, n'est pas une flakiness et n'a pas été masquée. La preuve a été reprise en lots exacts et par le binaire desktop final déjà lié, sans modifier la source.

## Qualité des assertions et traçabilité

| Risque | Assertion / oracle | Résultat |
| --- | --- | --- |
| Compatibilité d'un ancien réglage | Désérialisation sans champ cache → `128/64` | PASS |
| Valeurs invalides/contrat fermé | minima/maxima, hors bornes, négatif, flottant et champ `url` inconnu | PASS |
| Portée locale et indépendance | deux répertoires client ont des valeurs distinctes ; aucun serveur connecté/contacté | PASS |
| Durabilité/rollback en mémoire | réouverture retrouve `96/0`, save invalide ou I/O échoué conserve l'ancienne valeur en mémoire | PASS |
| UI et concurrence | commande IPC fermée unique, validation HTML, double submit bloqué, refresh ignoré pendant save, erreur visible | PASS DOM |
| Application moteur | défauts et valeurs non standard lus via les propriétés effectives de la vraie libmpv ; `0` désactive l'arrière | PASS |
| Transport/baux | tests de callback, annulation, HTTP/HLS non épinglé et horloges desktop inchangés | PASS code ; runtime matériel non rejoué |
| RSS/durée réelle | maximum configurable = 768 Mio de paquets plus overhead ; durée mpv indicative uniquement | NOT_RUN, explicitement STALE dans l'ADR/vérification |

Les assertions ne se limitent pas au modèle : elles couvrent la sérialisation disque, le contrôleur DOM livré et les propriétés libmpv réelles. Le chemin AppKit complet reste vérifié statiquement par compilation/clippy et par le passage explicite `UiClient → PlaybackConnection → PlayerHost → native_player → PlayerOptions/set_cache_limits`; aucun test automatisé de cette revue ne monte une fenêtre réelle avec la nouvelle valeur.

## Revue ADR et limites non bloquantes

- `ADR-F-001` **RESOLVED** : `ARCHITECTURE.md §10` conserve l'ancien texte pour l'historique mais désigne sans ambiguïté ADR-0010 comme budget normatif proposé ; `VERIFICATION.md` marque les anciennes preuves RSS client `STALE` et préserve les seuils Pi.
- `ADR-F-002` **RESOLVED** : les plafonds peuvent être légèrement dépassés, ne bornent pas le RSS, et `demuxer-cache-duration` est une estimation très peu fiable. L'oracle réel exige progression, temps mural, buffering, RSS et seek arrière/refetch.
- `ADR-F-003` **RESOLVED** : index avec révision, date, propriétaire/auteur, artefact remplacé et `superseded_by:NONE`. `Supersedes:NONE` dans l'en-tête de l'ADR reste cohérent pour les ADR : ADR-0004 n'est pas remplacée, seul son budget documentaire §10 est affiné.
- Les bornes hautes permettent jusqu'à `512+256 = 768 Mio` de paquets compressés par lecteur, plus décodeur/rendu/webview. C'est un choix explicite du Mac et non une promesse de performance ; mesurer le RSS avant recommandation de valeurs élevées.
- Rollback applicatif sûr vers l'ancien bundle : pas de migration serveur/NAS/TLS. L'ancien client ignore le champ inconnu ; s'il réécrit ensuite `client.json`, la valeur personnalisée peut être perdue et redevenir `128/64` lors du retour au nouveau bundle. Cette perte d'un réglage local recréable est non bloquante mais doit figurer dans la note de livraison.
- La mention UI « prochaine lecture » est cohérente : ouvrir Administration arrête le lecteur ; une session déjà créée conserve volontairement son budget pendant reprise/enchaînement. Aucun changement à chaud d'une lecture active n'est promis.

Aucun fichier source, test, ADR ou déploiement n'a été modifié pendant la revue.
