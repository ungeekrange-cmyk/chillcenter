# Revue code indépendante — découverte LAN Mac/serveur 0.7.1

- **Mode** : `DEV_STANDARD`, revue source exacte et non mutante
- **Base** : `50733ecf749225449c1aac5a6064125c3a9d249c`
- **Candidat revu** : `ad64644f27a45f111ce6bc07b4f3c79f654262d5`
- **Branche gelée au début** : `codex/mac-server-discovery`
- **Reviewer** : `/root/discovery_code_review`, distinct de l'auteur
- **Verdict** : **CHANGES_REQUESTED**

Le verdict porte uniquement sur le delta `50733ec..ad64644`. Les changements API
et concepts apparus ensuite dans le worktree partagé, dont `641a024`/ADR-0031,
sont exclus. Le candidat ne modifie pas `ui/connected`, `web`, le client Rust de
confiance, les réglages, les certificats ni les routes API ; web/thèmes restent
0.7.0 et Mac/serveur passent bien à 0.7.1. La découverte fournit une autorité au
formulaire `connect` existant : TOFU, SAN, SPKI et vérification UUID restent donc
dans leur chemin ordinaire.

## Constats actionnables

### CR-F-001 — HIGH / P1 — annonce publiée avant que HTTPS écoute

`crates/chill-server/src/main.rs:119-123` démarre l'annonceur, puis exécute encore
l'initialisation des staging à `:126-130`. Le serveur HTTPS n'est construit qu'à
`:163-186` et son futur n'est sondé qu'à `:187-194`. Dans `axum-server 0.8.0`,
`axum_server::bind` ne lie pas la socket : le `TcpListener::bind` se produit au
début de `Server::serve`.

Le serveur peut donc être résolu par mDNS alors que son port HTTPS refuse encore
les connexions. Le chemin Mac « candidat unique » soumet immédiatement l'adresse ;
un démarrage simultané Mac/serveur peut échouer une première fois et exiger une
action manuelle, précisément sur le chemin automatique ajouté par ce lot.

**Correction requise** : réserver/lier la socket HTTPS avec succès avant
l'enregistrement mDNS (par exemple listener pré-lié passé à `from_tcp`), ou
attendre une preuve explicite `listening` avant d'annoncer. Ajouter une régression
qui interdit toute résolution exploitable avant l'écoute et confirme le retrait
à l'arrêt/à l'échec de bind.

### CR-F-002 — MEDIUM / P1 — une réponse tardive peut remplacer un choix de serveur enregistré

`apps/desktop/src/native_discovery.js:20` invalide la génération pour la saisie/
soumission du formulaire, `#server-switch` et `#reject-trust`, mais pas pour les
boutons existants de `#saved-servers`. Ces boutons appellent directement
`connect(address)` dans `ui/connected/app.js:42`, sans événement `input` ou
`submit`. Une recherche déjà en vol peut alors attendre la fin du busy, puis
soumettre son candidat unique après un choix explicite enregistré, notamment si
ce choix échoue ou ouvre le panneau de confiance.

Il n'y a pas de contournement TLS, mais le résultat périmé remplace une intention
utilisateur et peut substituer le prompt TOFU affiché.

**Correction requise** : invalider la génération sur tout clic de serveur
enregistré (ou exposer un signal unique pour toute intention de connexion), puis
ajouter le test de course « browse en vol -> clic saved server -> réponse unique
tardive », y compris le cas où la connexion choisie atteint `trust` ou échoue.

### CR-F-003 — MEDIUM / P2 — le contrat wildcard n'est pas atteignable par le serveur

ADR-0030 `:19-20` et `crates/chill-discovery/src/lib.rs:225-250` promettent et
testent la sélection d'interfaces pour un listener wildcard. Mais
`crates/chill-server/src/config.rs:55-57` refuse toute adresse non locale, donc
`0.0.0.0` et `[::]`, et `main.rs:119-123` ne reçoit qu'une configuration déjà
validée. Le chemin wildcard est ainsi du code mort pour le binaire 0.7.1 ; son
test unitaire ne prouve aucune annonce serveur intégrée.

**Correction requise** : réconcilier ADR et implémentation sans relâcher
silencieusement la frontière LAN sans authentification. Soit le lot 0.7.1 reste
sur bind LAN explicite et l'ADR/verification l'indique, soit une exception
wildcard dispose d'une frontière réseau compensatoire approuvée et testée. Ne pas
simplement supprimer la validation historique.

### CR-F-004 — LOW / P3 — ADR-0030 absent de l'index au SHA revu

Le candidat ajoute `docs/architecture/decisions/ADR-0030-lan-server-discovery.md`
mais ne modifie pas `docs/architecture/decisions/INDEX.md`; à `ad64644`, l'index
commence encore par ADR-0029. La décision significative n'est donc pas
répertoriée selon le protocole du dépôt. Une mise à jour ultérieure du worktree
l'ajoute, mais elle n'appartient pas au SHA revu.

**Correction requise** : indexer ADR-0030 dans le prochain candidat et conserver
son statut/gate exact sans faire passer les preuves LAN `NOT_RUN` pour acquises.

## Preuves et deltas réutilisés

- Archive Git isolée du candidat exact : `git archive ad64644...`; le worktree
  partagé devenu sale n'a pas servi de source aux conclusions finales.
- `cargo fmt --all -- --check` : **PASS** sur l'archive exacte.
- `cargo test -p chill-discovery --locked` : **PASS 5/5** sur l'archive exacte.
- Tests Node découverte + calendrier : **PASS 10/10** sur l'archive exacte.
- Clippy exact complet : **NOT_RUN** (compilation interrompue pour rendre le
  verdict borné) ; un run sur le worktree devenu concurrent n'est pas réutilisé.
- Dépendances nouvelles : `mdns-sd 0.21.6` et `if-addrs 0.15.0`, versions
  courantes retournées par crates.io le 20 octobre 2026, verrouillées avec
  checksums ; aucun runtime EOL nouvellement introduit observé.
- Revue tests exacte `docs/releases/REL-0.7.1-test-review.md` : **APPROVED** pour
  le gate automatisé/statique, LAN 0.7.1 et GUI/macOS réels `NOT_RUN`. Elle note
  déjà le retry non cliqué et les courses saved boot/lecteur non exercées.
- Revue sécurité exacte `docs/releases/REL-0.7.1-security-review.md` :
  **APPROVED**, aucun contournement de confiance. Ce verdict ne ferme pas les
  défauts fonctionnels/ordre de boot ci-dessus.
- Aucun Pi, NAS, déploiement, réglage ou test source n'a été modifié par cette
  revue. Le seul fichier écrit est ce rapport.

## Verdict

**CHANGES_REQUESTED** pour `ad64644f27a45f111ce6bc07b4f3c79f654262d5`.
Les constats sont statiques et localisés : retour au builder, nouveau SHA source,
puis nouvelle revue code exacte. Le gate triple ne peut pas rejoindre malgré les
revues tests/sécurité favorables sur l'ancien SHA. Les essais LAN/permission
macOS réels restent en outre nécessaires avant remise, sans déploiement Pi
implicite.
