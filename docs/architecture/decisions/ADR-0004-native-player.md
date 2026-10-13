# ADR-0004 — Lecteur Mac intégré et transport Rust commun

- Statut : `ACCEPTED` ; révision `CC-DOC-001` ; 2026-10-13. Validation : [G-02](../../gates/G-02-CC-DOC-001.md), proposition originale/hash conservés dans le manifeste historique.
- Propriétaire/auteur : `/root` ; reviewer : voir rapport indépendant.
- Baseline : `90c149b797b45cba8a56dca2b74eed06c4225060` ; commit Chill Center : `NOT_CREATED`.
- Supersedes/superseded_by : `NONE` ; exigences F-02–03, F-09–11, F-22.

## Contexte et options

Lecture externe héritée incompatible avec contrôle des pistes, reprise, exclusivité et vu. MKV/MP4/AVI, sous-titres externes et faible consommation exigent une bibliothèque de décodage plutôt qu'une réécriture Rust. Sources L-01/L-03, S-06 à S-09.

| Option | Bénéfices | Risques/coûts | Réversibilité |
| --- | --- | --- | --- |
| Tauri + libmpv + adaptateur Rust | UI réutilisée, contrôle natif des pistes | FFI, rendu Mac, licences/packaging à vérifier | Moyenne, interface lecteur isolée |
| Tauri + vidéo HTML | Intégration UI simple | Codecs et pistes limités selon webview | Élevée |
| Client entièrement natif réécrit | UI sans webview | Réécriture importante du catalogue, aucun codec fourni par l'UI | Moyenne |

## Décision proposée

Conserver Tauri et UI légère ; moteur libmpv embarqué, VideoToolbox si disponible. Rust gère interface lecteur et commandes/événements. Flux personnalisés via transport HTTPS Rust validé ; sous-titres via cache borné. `stream_cb` est instable : épingler moteur stable/ABI, vérifier client API ≥1.106 pour cancel. IO bloquantes hors UI/runtime ; pas d'appel libmpv depuis callback. Annulation non bloquante, cookies refcountés jusqu'à close et destruction du moteur. Pas d'URL brute avec TLS désactivé, pas de lecteur externe.

## Conséquences et validation

Positif : maximum de logique Rust et formats natifs ; aucun logiciel supplémentaire requis chez l'utilisateur. Négatif : codecs C/C++ et couche AppKit nécessaires, API callback instable ; faisabilité non démontrée. Enabler obligatoire : arm64, HTTPS/seek, rendu, pistes, mémoire et annulation concurrente pendant IO/fermeture. Si échec, remplacement par ADR avant autre moteur ; pas de promesse de transcodage serveur.

Contrôler versions stables, options de compilation, licences et dépendances du bundle avant distribution. Patches des décodeurs à suivre. V-03/V-09 et mesure client prouvent qualité/performance. Observabilité : décodage matériel/logiciel, underruns et erreurs de piste, sans URL sensible. Migration : adaptateur remplace ouverture externe ; rollback vers ancien bundle sans toucher au NAS, schéma serveur compatible nécessaire. Ce choix affecte enabler, contrats player et distribution Mac.

Revue initiale : [ADR-0004-review](reviews/ADR-0004-review.md), `CHANGES_REQUESTED`. Revue du delta : [ADR-0004-review-r2](reviews/ADR-0004-review-r2.md). Gate utilisateur : [TRACEABILITY](../../TRACEABILITY.md), étape 2 `APPROVED`, preuve [G-02](../../gates/G-02-CC-DOC-001.md).
