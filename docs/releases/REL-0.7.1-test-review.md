# Revue indépendante des tests — découverte LAN Mac/serveur 0.7.1

- **Mode** : `EXECUTION_REVIEW` (`DEV_STANDARD`, gate source exact)
- **Branche / source revue** : `codex/mac-server-discovery` / `ad64644f27a45f111ce6bc07b4f3c79f654262d5`
- **Base** : `50733ecf749225449c1aac5a6064125c3a9d249c`
- **Reviewer** : `/root/discovery_test_review`, distinct de l'auteur et sans modification du code ou des tests
- **Environnement** : macOS 27.0.1 (26A434), arm64 ; Rust/Cargo 1.99.1 ; Node 24.13.0
- **Verdict** : **APPROVED** pour le gate automatisé et statique borné de cette source exacte. La vraie GUI macOS et l'interopérabilité annonce/navigation mDNS avec un serveur 0.7.1 restent des gates runtime `NOT_RUN` avant remise ; ce verdict n'autorise ni déploiement Pi ni publication.

## Preuves reproductibles

| Commande | Sortie | Durée murale | Résultat | Retries / limites |
| :--- | :---: | :---: | :--- | :--- |
| `CARGO_TARGET_DIR=/private/tmp/chillcenter-v07-mac-tests-target cargo test -p chill-discovery --locked --offline` | 0 | 8,44 s | **5 réussis**, 0 échec, 0 ignoré ; doc-tests 0 | Aucun retry. Compile et exerce le parseur, les filtres LAN, la limite de 8 adresses et les sélections wildcard/concrète. |
| `CARGO_TARGET_DIR=/private/tmp/chillcenter-v07-mac-tests-target cargo test -p chillcenter-desktop discovery_bridge --locked --offline` | 0 | 12,60 s | **1 réussi**, 0 échec, 37 filtrés | Aucun retry ; commande IPC limitée à `browse`, champs supplémentaires et action `connect` refusés. |
| `node --test tests/native_discovery.dom.test.js` depuis `apps/desktop/` | 1 | 0,007 s | 0 assertion exécutée ; 6 erreurs `ENOENT` | Invocation initiale au mauvais cwd, conservée comme observation d'outillage. Le test charge volontairement `apps/desktop/src/native_discovery.js` depuis la racine. Aucun défaut candidat déduit. |
| `node --test apps/desktop/tests/native_discovery.dom.test.js` depuis la racine | 0 | 0,025 s (harness 83,51 ms) | **6 réussis**, 0 échec/skip/todo | Retry unique corrigé uniquement par le cwd requis. |
| `node --test docs/design/concepts/2026-10-20/calendar.test.cjs` | 0 | 0,002 s (harness 69,93 ms) | **4 réussis**, 0 échec/skip/todo | Concepts séparés : aucun branchement calendrier/thème produit. |
| `CARGO_TARGET_DIR=/private/tmp/chillcenter-v07-mac-tests-target cargo run -p chill-discovery --example discover --locked --offline` sous sandbox | 1 | 1,2 s | `Operation not permitted` à la création du socket mDNS | Restriction UDP du sandbox, pas un échec produit. |
| Même sonde, autorisée hors sandbox | 0 | 4,49 s | `No Chill Center server discovered in 4 seconds.` | Navigation réellement bornée. Le serveur LAN actif est encore 0.7.0 et n'annonce pas ce service : zéro candidat est attendu et ne prouve pas l'interopérabilité 0.7.1. |
| `git diff --quiet 50733ec..ad64644 -- ui/connected web` | 0 | <1 s | **Aucun delta web/UI partagé** | Confirme la séparation : injection Mac native uniquement ; web 0.7.0 inchangé. |

Le worktree était propre au démarrage et pendant la capture des preuves. Des modifications concurrentes hors périmètre (`ADR-0031` et concepts d'ambiance) sont apparues ensuite ; le reviewer ne les a ni examinées ni modifiées. Son seul fichier créé est ce rapport. Aucun test lecteur, aucune écriture Pi/NAS et aucun déploiement n'ont été effectués.

## Qualité des assertions et traçabilité

| Risque / contrat ADR-0030 | Couverture examinée | Résultat et portée |
| :--- | :--- | :--- |
| Navigation bornée à 4 s, cap dur 5 s | Le pont appelle `browse(Duration::from_secs(4))`; la sonde réelle sans résultat termine à 4,49 s ; la bibliothèque applique `min(MAX_BROWSE)` | **PASS source + runtime sans annonce**. Le cap 16 serveurs est visible dans la boucle mais n'a pas de test générant 17 résolutions. |
| Rejets des enregistrements non conformes | UUID, port zéro, IP publique/loopback/IPv6 link-local rejetés ; protocole/API vérifiés dans `record_from_info` | **PASS ciblé** pour les entrées testées. Les TXT protocole/API transitent par l'adaptateur `mdns-sd` et ne sont pas injectés dans un test de `ResolvedService`. |
| Annonce écoute concrète / wildcard | Adresse concrète exacte, loopback refusée ; wildcard filtre LAN, déduplique et limite à 8 | **PASS logique**. L'enregistrement et la résolution réels sur deux hôtes restent `NOT_RUN`. |
| Surface IPC minimale | `browse` seul accepté ; URL/adresse arbitraire et action `connect` refusées | **PASS** avec assertions négatives significatives. |
| Saisie manuelle masquée puis révélée | Formulaire masqué, libellé FR, révélation et focus contrôlés | **PASS DOM synthétique** ; rendu/VoiceOver/clavier réel `NOT_RUN`. |
| Retry | Le test anglais affirme l'état sans résultat et la présence du bouton `Retry`; le code lie `onclick=search` | **PARTIAL** : aucune assertion ne clique le retry ni ne vérifie un second appel IPC. À couvrir dans le QA natif avant remise ; ne pas comptabiliser comme retry E2E passé. |
| Plusieurs candidats | Aucun submit avant choix ; le premier choix fixe l'autorité HTTPS puis soumet | **PASS DOM synthétique**. |
| Candidat unique et chemin TOFU ordinaire | L'adresse découverte est préfixée `https://` puis `requestSubmit()` sur `#connect-form`; le handler partagé inchangé envoie `UiCommand::Connect`, qui conserve l'étape de confiance/approbation | **PASS de raccordement statique + DOM** ; aucun certificat réel ni écran TOFU natif n'est exercé dans ce lot. |
| Résultat tardif après saisie manuelle | L'événement `input` invalide la génération et empêche le submit tardif | **PASS DOM synthétique**. |
| Connexion enregistrée saine | Le fixture sain empêche toute soumission découverte | **PASS pour l'état sain déjà établi**. La course réelle « browse en vol puis boot sauvegardé sain » et le délai occupé de 6 s restent à exercer dans l'app native. |
| Lecture active / changement de serveur | Les gardes `player-open`, `#server-switch` et `#reject-trust` sont présentes dans le code | **NOT_RUN** dans les six tests ciblés ; aucune revendication lecteur n'est faite. |
| Concepts Pâques/Saint-Valentin | Computus, fenêtres civiles, années bissextiles et erreurs | **PASS 4/4**, proposition documentaire seulement. |

Les assertions Rust contrôlent les valeurs produites et les refus de frontières, et les assertions DOM exécutent le fichier JavaScript réellement injecté. Elles ne sont pas des preuves de rendu WebView/AppKit, d'autorisation réseau local macOS, de Bonjour entre machines, ni de TLS/SPKI sur le LAN.

## Finding et gates restants

| ID | Statut | Preuve | Action requise | Route |
| :--- | :--- | :--- | :--- | :--- |
| `TEST-F-001` | **OPEN — gate runtime avant remise, non bloquant pour le gate source** | Serveur actif 0.7.0 : navigation réelle 4 s sans candidat ; aucun bundle 0.7.1 lancé et aucune annonce 0.7.1 déployée. | Sur un Mac/bundle 0.7.1 et un serveur 0.7.1 annonçant réellement : vérifier autorisation réseau local, découverte unique/multiple, retry exécuté, course connexion sauvegardée saine, saisie manuelle tardive, choix puis écran TOFU ordinaire, et repli manuel quand multicast est indisponible. Capturer versions, adresses expurgées, chronométrage et résultat. | Intégration / vérification de release, pas le reviewer de tests |

## Disposition

Le SHA `ad64644f27a45f111ce6bc07b4f3c79f654262d5` satisfait le gate de tests ciblés de la logique de découverte, du pont IPC et du DOM synthétique dans la portée ci-dessus. Toute modification source ultérieure, y compris une unification de préfixe API, rend cette approbation exacte obsolète pour le nouveau SHA. Ne pas présenter cette revue comme un PASS GUI macOS, un PASS LAN 0.7.1, une validation du lecteur, une publication ou un déploiement Pi.
