# Revue de sécurité indépendante — découverte LAN 0.7.1

- **Candidat revu** : `ad64644f27a45f111ce6bc07b4f3c79f654262d5`
- **Base de comparaison** : `50733ec`
- **Branche figée au début de la revue** : `codex/mac-server-discovery`
- **Auteur du candidat / reviewer** : `/root` / `/root/discovery_security_review`
- **Revue effectuée à** : `2026-10-20T21:02:33Z`
- **Verdict source** : **APPROVED**
- **ADR-0030** : **APPROVED** pour ce SHA exact

Le verdict couvre uniquement la source immuable `ad64644`. Des modifications
locales non commitées sont apparues dans le worktree après le gel ; elles sont
hors de cette revue et devront recevoir une nouvelle revue sur leur propre SHA.
Aucun Pi, NAS, service, paquet, réglage ou état cloud n'a été contacté ou modifié.

## Contrôles de confiance et d'exposition

- **Authentification, autorisation et isolation** : le projet reste volontairement
  sans authentification sur le LAN familial (F-18) ; aucune route, permission
  serveur, session, cookie, CSRF, profil ou frontière de tenant n'est ajoutée.
  La découverte ne confère aucune autorité.
- **TLS et identité** : le résultat mDNS fournit uniquement une autorité IP:port.
  Le script natif la soumet au formulaire `connect` existant. Le client inchangé
  exige HTTPS, refuse identifiants/requête/fragment et chemins, valide la période,
  la signature et le SAN du certificat, conserve l'approbation SPKI, puis vérifie
  l'UUID réel via `/api/v1/server`. Aucun chemin TOFU, SAN, SPKI ou remplacement
  de clé n'est contourné.
- **IPC natif** : `DiscoveryCommand` accepte seulement `{action:"browse"}` avec
  `deny_unknown_fields`; URL, hôte, chemin, shell et fichier sont refusés. La
  capability Tauri autorise uniquement `discovery_command`, dont le handler ne
  lit pas `UiClient` et possède un sémaphore single-flight distinct.
- **Annonces malveillantes** : UUID, version, nom, port et adresses sont validés ;
  seuls UUID valides, HTTPS/API attendus, texte ASCII borné, port non nul et
  IPv4 privée/link-local ou IPv6 ULA sont retenus. Le client plafonne à 5 s,
  16 serveurs et 8 adresses par serveur. Les noms sont rendus avec `textContent`,
  sans HTML. Un pair LAN peut encore usurper/supprimer une annonce ou provoquer
  une tentative TLS bornée ; il ne peut ni faire approuver sa clé ni remplacer
  une connexion saine. La saisie manuelle reste disponible.
- **Démon et ressources** : le browse normal dure 4 s, avec hard cap 5 s et arrêt
  explicite `stop_browse`/`shutdown`; l'annonceur long-lived est désenregistré et
  arrêté au drop. `mdns-sd 0.21.6` utilise des files bornées (100 commandes,
  1 000 événements), des paquets bornés et n'accepte pas les réponses non
  sollicitées par défaut. L'échec mDNS est non fatal pour HTTPS.
- **Réseau local** : l'annonce publie seulement les adresses des interfaces LAN
  opérationnelles, non P2P, correspondant au listener ; loopback, IP publiques et
  IPv6 link-local sont exclues des candidats. mDNS reste multicast local ; il
  n'ajoute aucune écoute HTTP/TCP ni exposition Internet.
- **Navigation et URL** : l'allowlist de navigation Tauri reste limitée à
  `tauri://localhost` et `http://tauri.localhost`. La découverte ne navigue pas le
  WebView et ne construit aucune route HTTP arbitraire ; seule la connexion HTTPS
  existante est déclenchée.
- **Logs et secrets** : aucune annonce distante n'est journalisée par l'application.
  L'unique erreur serveur est générique et ne contient ni profil, chemin média,
  certificat, clé ou secret. Aucun indicateur de secret ajouté n'a été trouvé.
- **Invariants hors périmètre** : les arbres `ui/connected`, `crates/chill-client`,
  API/version/configuration/TLS/identité serveur, web et thèmes ont des hashes Git
  identiques entre la base et le candidat. Seuls Mac et serveur passent à 0.7.1.

## Dépendances et provenance

`mdns-sd 0.21.6` est épinglé exactement avec les features par défaut désactivées ;
`if-addrs 0.15.0` est verrouillé. Les deux proviennent du registre crates.io avec
checksums dans `Cargo.lock`. Licences inspectées dans les manifests du registre :
`mdns-sd` Apache-2.0 OR MIT, `if-addrs` MIT OR BSD-3-Clause. L'arbre transitif
ajouté ne contient aucune source Git. La vérification officielle de disponibilité
des versions courantes a été fournie au reviewer ; aucun scanner RustSec/licence
complet n'était installé (`cargo audit`, `cargo deny` et `gitleaks` indisponibles).

## Preuves reproductibles

| Outil / version | Commande et portée | Résultat | Limites |
| --- | --- | --- | --- |
| Git | `git diff --name-status 50733ec ad64644` et hashes des arbres protégés | PASS ; delta borné, invariants protégés inchangés | Les edits non committés ultérieurs sont exclus |
| Rust 1.99.1 / Cargo 1.99.1 | `CARGO_TARGET_DIR=/private/tmp/chillcenter-security-review-target cargo test -p chill-discovery --locked --offline` | PASS, 5/5 | Pas un essai LAN physique |
| Node 24.13.0 | `node --test apps/desktop/tests/native_discovery.dom.test.js docs/design/concepts/2026-10-20/calendar.test.cjs` | PASS, 10/10, dont 6/6 découverte native | DOM simulé, pas WebKit/macOS réel |
| Cargo | `cargo tree -p chill-discovery --locked --offline` | PASS ; arbre registry/checksummé, sans source Git | Pas de base d'advisories locale |
| Inspection statique | diff, capability Tauri, parser IPC, client TLS inchangé, sources locales `mdns-sd`/`if-addrs` | PASS | Ne remplace pas fuzz réseau hostile |
| Scan ciblé | recherche d'indicateurs de clés privées, AKIA, API keys, secrets, mots de passe et bearer tokens dans les ajouts texte | PASS | Heuristique ; pas `gitleaks` |

Les suites générales `chill-server` et `chillcenter-desktop` ont été interrompues
pendant leur compilation : aucun PASS ne leur est attribué ici. L'annonce/browse
LAN réel, l'autorisation macOS Local Network, le build Mac complet et le binaire
ARM64 Linux restent des preuves de test/release à fournir séparément. Le futur
remplacement Pi demeure autorisé uniquement binaire, après sauvegarde privée ;
aucune écriture Pi n'a eu lieu pendant cette revue.

## Constats

Aucun constat `CRITICAL`, `HIGH`, `MEDIUM` ou `LOW` ouvert. Risque résiduel
informatif : un acteur déjà présent sur le même segment multicast peut perturber
ou usurper la découverte. ADR-0030 le traite comme un indice non fiable, avec
HTTPS/SAN/SPKI/UUID obligatoires, bornes strictes et repli manuel ; ce risque ne
constitue pas une nouvelle acceptation de confiance.

