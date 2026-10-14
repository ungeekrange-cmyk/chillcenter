# Revue delta de sécurité — contrat OpenAPI Chill Center

## Décision

- **Verdict : APPROVED**
- **Candidat exact :** `0a7e8f300d0e1f1b0723cf91d6aade15412027f5`
- **Base du delta :** `50bbdf800a6c46c67ddd3cce74d8358286cd8eb6`
- **Relecteur indépendant :** `/root/ux_api_security_review`
- **Date :** 2026-10-14T16:28:11Z
- **Findings ouverts :** 0 CRITICAL, 0 HIGH, 0 MEDIUM, 0 LOW

Cette décision porte sur le SHA exact, pas sur une transmission implicite de l'approbation de la base. Le delta a été relu comme un nouveau candidat : six fichiers de contrat, validation et traçabilité changent ; aucun fichier Rust, UI runtime, configuration TLS, dépendance ou lockfile ne change.

## Analyse du delta

### Réponses extensibles sans relâcher les requêtes

- Tous les objets atteignables depuis une réponse OpenAPI autorisent désormais des champs additionnels, y compris les objets imbriqués. Cette règle évite qu'un ajout compatible du serveur invalide les clients tiers.
- Les schémas partagés entre requêtes et réponses sont séparés : `UuidResponse` et `PlaybackContextResponse` sont employés côté réponse, tandis que `Uuid` et `PlaybackContext` conservent leurs contraintes de requête.
- Vérification mécanique indépendante : **40 request bodies / 37 schémas de fermeture inchangés** entre base et candidat, sauf la métadonnée documentée `PreferencesInput.autoplay_seconds.default = 10`.
- `TmdbConfig` reste `additionalProperties: false`; `credential` reste `writeOnly: true`. Aucun champ de secret n'est ajouté à une réponse.
- Les 49 schémas de composant atteignables depuis les réponses ont été parcourus récursivement : **0 objet de réponse fermé résiduel**.
- Toutes les références sont locales à `#/components/...` : **0 `$ref` externe** et donc aucune résolution réseau ou substitution de schéma tierce.

L'extensibilité des réponses n'autorise pas le serveur à émettre un secret ; elle rend uniquement le validateur client tolérant aux futurs champs. Toute modification runtime ajoutant un champ sensible resterait soumise à une nouvelle revue de code/sécurité.

### Scripts de normalisation et validation

- `scripts/openapi-response-extensibility.mjs` opère sur un chemin local fixe, sans réseau, exécution de commande, entrée utilisateur ou credential. Les copies ne concernent que les schémas partagés avec une requête.
- Le normalisateur conserve les dictionnaires typés et ouvre récursivement leurs valeurs objet ; les cycles par `$ref` sont suivis avec des ensembles de visite.
- `scripts/validate-api-contract.mjs` rejette désormais tout objet fermé dans le graphe des réponses et continue à imposer des corps de requête stricts, les routes datées, l'absence de références invalides et le credential TMDB `writeOnly`.
- Les deux scripts passent `node --check`. Le normalisateur n'a pas été exécuté pendant cette revue afin de préserver le freeze ; son résultat commité a été analysé en lecture seule.

### Traçabilité

- `BUG-0048` passe à `VERIFIED_CODE` en citant explicitement le SHA et les limites restantes. Il ne prétend ni mesure Pi/RSS, ni déploiement, ni clôture physique globale.
- Les rapports du SHA précédent sont ajoutés comme historique. Ils ne contiennent pas de credential et ne remplacent pas le présent verdict exact-SHA.

## Vérifications exécutées

| Contrôle | Résultat |
|---|---|
| `git rev-parse HEAD` / `git status --short` | SHA exact, worktree propre |
| `git diff --check 50bbdf8..0a7e8f3` | PASS |
| Inventaire du delta runtime/dépendances | Aucun Rust/UI runtime/TLS/provider/manifeste/lockfile modifié |
| `node scripts/validate-api-contract.mjs` | PASS — 66 chemins, OpenAPI 2026-10-14 |
| Comparaison récursive des request bodies et de leur fermeture | PASS — 40 bodies, 37 schémas inchangés hors défaut autoplay documenté |
| Parcours récursif des réponses | PASS — 49 composants atteints, 0 objet fermé |
| Audit des `$ref` | PASS — 0 référence externe |
| Invariant TMDB request | PASS — fermé et secret `writeOnly` |
| `node --check` sur les deux scripts modifiés/ajoutés | PASS |
| Balayage ciblé clés privées/tokens usuels au SHA exact | Aucun résultat |

## Limites

- Aucun test Rust, shell Tauri, appel fournisseur, token, credential, Pi, NAS, SSH ou déploiement n'a été exécuté : le delta ne touche aucun de ces runtimes.
- Les 38 payloads validés formellement avec OpenAPI 3.1 et les 30 réponses HTTPS sont des preuves produites par une autre instance ; elles n'ont pas été rejouées ici.
- Le balayage de secrets repose sur des motifs connus, pas sur une analyse d'entropie exhaustive.
- L'approbation est limitée au SHA `0a7e8f300d0e1f1b0723cf91d6aade15412027f5`. Toute modification ultérieure du contrat, du runtime ou des validateurs nécessite une nouvelle revue delta.

## Conclusion

Le candidat corrige la contradiction entre réponses compatibles et objets OpenAPI fermés sans relâcher les requêtes ni la protection du secret TMDB. Aucune nouvelle exposition ou régression de sécurité n'est détectée. **APPROVED** pour promotion dans le périmètre LAN domestique documenté.
