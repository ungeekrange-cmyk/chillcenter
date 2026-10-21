# Revue indépendante des tests — BUG0127

- **Mode :** `EXECUTION_REVIEW` (DEV_FAST)
- **Branche/source revue :** `codex/update-check-controls` /
  `a38d4b16616ebd6735fa4aab0b518e28d767079e`
- **Base :** `6ba3970b314fb96325f906a5ccec2cde188c6ada`
- **Porteur documentaire courant :** `258518d` (non candidat source)
- **Auteur/intégrateur :** `/root`; **reviewer indépendant :** `/root/v072_test_review`
- **Environnement :** macOS, Node `v24.13.0`, Cargo `1.99.1`
- **Verdict source :** **APPROVED**

Cette approbation est strictement attachée au delta source `6ba3970..a38d4b1`.
Elle n'autorise pas la publication, le tag, le déploiement Pi/web, le NAS, les
réglages, ni une validation GUI native. Le commit `258518d` est un porteur de
documentation et n'est pas assimilé à une nouvelle candidate runtime.

## Contrat et portée inspectés

Le contrat R1 (`docs/updates/CONTRACT.md`) impose des contrôles manuels
non-root, bornés et signés sans helper, sans téléchargement d'archive ni
autorité d'installation, avec au plus trois workers et coalescence de cinq
minutes. `check_available` est additif à `helper_available`; les contrôles
d'installation restent masqués sans version stable strictement plus récente et
après une vérification échouée/en cours.

Le delta vérifié ajoute le worker de contrôles séparé, expose la capacité
additive dans l'API datée inchangée, conserve les corps de requête fermés, et
couvre les clients Mac/web. `git diff --check 6ba3970..a38d4b1` est PASS.

## Preuves exécutées indépendamment

| Commande exacte | Exit | Durée | Résultat | Portée |
| :--- | :--- | :--- | :--- | :--- |
| `cargo test -p chill-server passive_checks_report_results_without_helper_or_install_intents --locked` | 0 | 1.104 s | 1 PASS, 0 échec | Contrôle passif sans helper ni intention d'installation. |
| `cargo test -p chill-server failure_or_no_update_cannot_leave_an_installable_stale_hint --locked` | 0 | 0.501 s | 1 PASS, 0 échec | Échec/absence de MAJ efface l'indice installable. |
| `cargo test -p chill-server status_remains_readable_while_a_metadata_fetch_is_waiting --locked` | 0 | 0.363 s | 1 PASS, 0 échec | Statut/API disponible pendant le fetch borné. |
| `cargo test -p chill-server component_checks_use_cached_receipts_without_install_helper --locked` | 0 | 0.657 s | 1 PASS, 0 échec | Reçu coalescé sans helper; installation toujours refusée. |
| `node --test ui/connected/updates.test.js web/test/bridge.test.js` | 0 | 0.558 s | 24 PASS, 0 échec | SemVer strict, corps same-origin fermé, bridge web daté. |
| `env CHILL_PREVIEW_NODE_MODULES=/Users/ghillaumesoubigou/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules node scripts/test-component-settings.cjs` | 0* | 12.0 s | PASS | DOM réel Chrome isolé : Mac et web, checks serveur/web/thèmes sans helper, install seulement si version détectée plus récente, échec de recheck, non-blocage catalogue, CAS et ancien serveur. |

\* Le premier lancement dans le sandbox a échoué avant chargement (`SIGABRT` /
`EPERM` Chrome). Le rerun isolé hors sandbox a réussi; ce n'est pas une
instabilité applicative et aucun appel HTTP n'était autorisé par le harnais.

Les assertions sont comportementales : elles testent la distinction des
capacités, les versions égales/antérieures, le nettoyage d'un indice obsolète,
les composants fermés et l'interactivité pendant une demande retenue. La DOM
réelle utilise toutefois un bridge synthétique et des données contrôlées; elle
ne prouve ni AppKit/consentement macOS réel, ni Pi, ni release GitHub/artefacts
publiés. La QA HTTPS locale déjà consignée dans BUG0127 complète la frontière
réseau/signature avec GitHub public, sans remplacer un test de distribution
immutable.

## Matrice risque → preuve

| Risque | Couche | Oracle | Résultat |
| :--- | :--- | :--- | :--- |
| Contrôle refusé sans helper | unité Rust + DOM Chrome | `check_available=true` laisse les checks actifs et aucun intent install n'est créé | PASS |
| Installation prématurée ou indice périmé | unité Rust + Node + DOM Chrome | SemVer strict; égal/ancien/échec/pending ne rend pas l'installation visible/exécutable | PASS |
| Régression API/web ou injection de destination | contrat Node | endpoint daté same-origin et corps réduit à `{component}` | PASS |
| Blocage du catalogue/statut | unité Rust + DOM Chrome | statut et catalogue restent utilisables pendant contrôle en attente | PASS |
| Compatibilité ancien serveur | DOM Chrome | seuls contrôles serveur disparaissent avec réponse explicitement non prise en charge | PASS |

## Limites et porte suivante

La suite serveur complète (147 PASS, 1 NFR RSS historique ignoré), Clippy, les
198 tests Node, les 23 contrôles bilingues et la QA HTTPS sont des preuves du
coordinateur, inspectées dans BUG0127 mais non toutes rejouées ici. Aucun test
de lecteur n'est requis par ce delta. Aucune GUI native, installation, Pi/NAS,
réglage utilisateur, publication ou validation de binaire/DMG signé n'est
inférée. La revue d'intégration attend la candidate de distribution immuable,
ses empreintes/signatures et les preuves de staging, sous une instance
d'intégration distincte.

## Findings

Aucun finding bloquant sur la source exacte `a38d4b1`. Tout delta runtime après
ce SHA, ou un artefact dont la provenance/empreinte ne le lie pas à ce SHA,
requiert une nouvelle revue d'intégration.

## Clôture d'intégration

La candidate intégrée `5cf1bbe70f1dd4ee98d67ae691334377df265b73` a ensuite
passé la revue d'intégration d'artefacts dans
`docs/integration/REL-0.7.2-review.md`: staging signé, provenance exacte,
wrappers, thèmes v0.7.1 réellement réutilisés et DMG monté lecture seule sont
conformes. Ce rapport de tests est donc clos; ses limites GUI native/Pi restent
inchangées.

## Delta opérateur ultérieur

Après cette revue source, `3d27456a3e2c46fb2dc8937b4e00370f85d54470` a ajouté
un delta d'opérateur Pi qui corrige l'acceptation du range SemVer signé du web.
Il n'est pas couvert par le verdict source ci-dessus. Exécution indépendante :
`python3 scripts/test-pi-v07-legacy-operator.py` — exit 0, 8 PASS, 9.377 s.
Les cas couvrent une livraison fixture complète, les reprises/rollbacks aux
frontières de remplacement, la conservation des réglages et les bornes SemVer
valides, manquantes, malformées, inversées et incompatibles. Cela prouve le
harnais local seulement; aucune connexion Pi, sauvegarde réelle, réglage ou NAS
n'a été touché. L'approbation finale reste conditionnée à la candidate
d'intégration immuable exacte et à ses artefacts attestés.
