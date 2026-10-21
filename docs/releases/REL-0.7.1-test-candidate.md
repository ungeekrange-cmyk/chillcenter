# Revue test et intégration indépendante — candidat 0.7.1

- **Mode / lane** : `INTEGRATION_REVIEW` / `FULL_ASSURANCE`
- **Reviewer indépendant** : `/root/v071_test_final`
- **Branche** : `codex/mac-server-discovery`
- **Base** : `aba640284cd1dcbb27bfb4d0c1bec2b9deef3c82`
- **Runtime R4 réutilisé** : `fdb80b502db45d10c5384104912f5f246678c32a`
- **Candidat final exact** : `c603f29db2936ee40f7bf66484187b4e237d9bfd`
- **Date** : `2026-10-21`
- **Verdict test / intégration source** : **APPROVED**

Ce verdict autorise l'entrée du SHA exact dans le packaging. Il n'approuve ni
un artefact futur, ni sa signature/publication, ni un déploiement, ni la
découverte sur un vrai LAN. Toute modification de source après ce SHA invalide
la présente décision.

## Historique et portée du candidat

Le candidat initial `1cf7b42826b58719aed16603f6e47d66d580cc2a` a été rejeté :
le pont natif omettait `assets/decorations` lors du parcours d'un pack externe
renderer2 valide et son plafond132 ne pouvait pas représenter128 assets plus le
manifeste et quatre répertoires. La grammaire calendrier/décoration divergeait
également entre les validateurs Rust et JS.

`f52c63016346e39ef4baed9667d2587f0a0cd67a` ferme ces écarts avec un vrai pack
externe temporaire, un plafond133, des négatifs croisés et le refus des parents
Bonjour modifiables groupe/autres. Le SHA final `c603f29` ferme ensuite la fuite
visuelle résiduelle : après succès Blob puis échec total native/HTTP, les URLs
sont révoquées et les propriétés CSS événementielles sont effacées avant le
logo de repli borné. Aucun de ces prédécesseurs n'est approuvé à la place du SHA
final.

Le serveur API et le web runtime hérités de R4 sont inchangés ; le dernier delta
`f52c630..c603f29` ne modifie ni Rust, ni pont natif, ni opérateur Bonjour. Les
versions `mac/server/web/themes=0.7.1`, les séquences2, l'API
`2026-10-14`, le format thème1 et renderer2 sont cohérents.

## Environnement et exécutions

Environnement : macOS27.0.1 arm64 (`26A434`), Git2.54.0, Node24.13.0,
Python3.9.6, Cargo/Rustc1.99.1, Chrome155.0.8059.85. Chaque preuve terminale a
été encadrée par `git rev-parse HEAD`; les contrôles au SHA `f52c630` ne sont
réutilisés au SHA final que lorsque `git diff --quiet` confirme que les sources
concernées sont identiques.

| Preuve | Résultat indépendant | Portée / qualité |
| --- | --- | --- |
| `cargo test --offline --locked --workspace` à `f52c630` | **443 PASS, 0 FAIL, 2 ignored**, code0, ~60s | Suite workspace complète. Les sources Rust/native sont octet-identiques entre `f52c630` et `c603f29`. Les deux NFR manuels ignorés ne sont pas comptés comme des passes. |
| `cargo test --offline --locked -p chillcenter-desktop theme_bridge::tests -- --nocapture` à `f52c630` | **4/4 PASS** | Construit le vrai arbre externe renderer2 de18 assets, valide le répertoire, suit le slot `current`, charge la version externe et refuse un dossier actif ajouté. |
| `node --test --test-reporter=spec ui/connected/*.test.js ui/connected/tests/*.test.js web/test/*.test.js` à `c603f29` | **323/323 PASS**, 0 skip, code0, 4.05s | Suite UI/web complète, incluant le cycle succès Blob → échec total et la révocation des anciennes URLs. |
| `CHILL_PREVIEW_NODE_MODULES=… CHILL_V05_EVIDENCE=<tmp isolé> node scripts/test-appearance-v05.cjs` à `c603f29` | **PASS**, code0, 3.38s ; 8 captures | Chrome réel sur UI connectée avec pont Tauri fixture : vrais octets PNG, SHA du manifeste, consommation CSS, cutout lecteur et nettoyage après échec total. Ce n'est pas une exécution Tauri/WebView native. |
| `python3 -m unittest -v scripts/test_pi_server_binary_operator.py scripts/test_pi_bonjour_family_operator.py` à `f52c630` | **31/31 PASS**, 0 skip, code0, 0.46s | 21 tests opérateur binaire R4 et 10 tests AF_NETLINK/Bonjour. Fichiers temporaires réels ; root/systemd simulés. Sources inchangées jusqu'à `c603f29`. |
| discovery native + calendrier + lapins + release CLI | **20/20 PASS** | 7 discovery DOM, 4 calendrier, 2 silhouette, 7 contrat release. |
| documentation bilingue | **23/23 PASS** | Alignement de la documentation opérateur et release. |
| `git diff --check f52c630..c603f29` | **PASS** | Aucun défaut whitespace dans le delta terminal. Le seul avertissement depuis la base est une ligne vide finale documentaire dans `REL-0.7.1-resource-final.md`. |

Une première exécution Rust dans le sandbox a échoué uniquement sur deux tests
loopback du lecteur avec `EPERM`. Cette preuve a été rejetée comme défaillance
d'environnement, puis la même commande a été rejouée hors de cette restriction
et a donné les443 passes ci-dessus. Ce n'est ni un retry produit ni un test
flaky observé.

## Assertions inspectées et couverture des risques

| Risque d'intégration | Conclusion |
| --- | --- |
| Renderer2 et compatibilité anciens packs | Le pack installé renderer2 est exercé depuis un arbre externe complet ; un manifeste renderer1 à six thèmes reste accepté et résolu. |
| PNG uniquement / chemins / intégrité | Les chemins sont fermés, les hashes et tailles sont vérifiés, le décodage PNG est complet et les SVG ne sont pas admis comme assets de pack. |
| Blob, CSP et rechargement | La CSP n'ajoute `blob:` qu'à `img-src`; seules les URLs émises et encore vivantes sont acceptées. Succès→succès et succès→échec total révoquent les générations attendues ; l'échec terminal efface backdrop, art et lapin. |
| Événements et calendrier | Pâques et Saint-Valentin ont des grammaires fermées et des négatifs partagés Rust/JS. Les assertions couvrent les fenêtres versionnées2026–2038 et les8 combinaisons clair/sombre. |
| Bonjour / AF_NETLINK | L'opérateur reste borné à sa drop-in, vérifie owner/mode/hash, refuse les parents0775/0777, conserve l'état en échec et restaure les familles exactes dans les tests. |
| Compatibilité serveur/web | L'API serveur et le web runtime sont inchangés face à R4 ; les identifiants de release et la séquence monotone sont testés. |
| Manifestes et signatures | Les fixtures Rust vérifient le parsing signé mac/web/thèmes, TLS et le refus d'un manifeste Mac expiré/périmé. Elles ne remplacent pas la vérification des artefacts de publication à construire. |

Les captures
`/private/tmp/chillcenter-v071-c603-test-review-appearance.U3Qcqp/` ont été
inspectées visuellement. Conformément au contrôle `character-proportions`, le
lapin réellement assemblé conserve le raster80×64 à l'échelle uniforme0,8
(64×51,2), avec seulement translation/réflexion : aucun étirement de tête,
corps ou oreilles n'est visible face à la référence de travail. Les membres
cachés ne sont pas vérifiables et ce contrôle technique ne transforme pas la
référence de projet en anatomie approuvée par l'utilisateur.

## Gates non exécutés et limites explicites

- **NOT_RUN** : application Mac GUI/AppKit/WebView native, consentement Local
  Network et découverte Bonjour sur un vrai LAN.
- **NOT_RUN** : exécution root/Linux/Pi réelle de l'opérateur AF_NETLINK,
  rollback couplé au vrai binaire et smoke post-déploiement.
- **PENDING** : build frais des DMG/archives serveur-web-thèmes, signatures de
  production, manifestes et hashes des artefacts, publication/upload/GitHub.
  Ces artefacts devront être rattachés à `c603f29` et vérifiés séparément.
- **NOT_RUN** : test GUI du lecteur et lecture vidéo répétée ; ces chemins sont
  inchangés et hors du delta ciblé.
- Les deux benchmarks NFR mémoire manuels restent `ignored`; ils ne constituent
  aucune preuve de performance pour cette approbation.

## Verdict

**APPROVED** pour l'intégration source et l'admission au packaging du commit
exact `c603f29db2936ee40f7bf66484187b4e237d9bfd`. Aucun échec fonctionnel,
régression critique, assertion vide ou skip dissimulé n'a été trouvé dans le
périmètre exercé. La promotion d'un binaire ou d'une release publique demeure
conditionnée aux gates artefact/signature, Pi/root, LAN natif et post-déploiement
ci-dessus, ainsi qu'aux revues code et sécurité indépendantes visant le même SHA.
