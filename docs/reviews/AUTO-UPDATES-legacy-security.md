# Revue sécurité indépendante — adoption automatique legacy / Legacy automatic adoption

- **Candidate / branch:** `50368f0aa34b8d519c31e2ecce8fa44f67bec56a` / `codex/pi-automatic-updates`
- **Base:** `7644d1dbb07e4cbaf5160c88fe1f5c2aa352aec4`
- **Runtime signé réutilisé / reused signed runtime:** `a38d4b16616ebd6735fa4aab0b518e28d767079e`
- **Reviewer indépendant / independent reviewer:** `/root/auto_adoption_security`
- **Date:** `2026-10-21`
- **Mode:** `FULL_ASSURANCE` (activation sur le Pi de production; revue bornée du plan/drop-in, sans réaudit complet du helper déjà approuvé)
- **Verdict:** **APPROVED** pour le plan exact et le drop-in; exécution autorisée seulement avec les portes ci-dessous.

Cette revue n'a modifié ni source candidate, ni Pi, NAS, GitHub, service ou
configuration distante. Son seul écrit est ce rapport. L'autorisation humaine
porte sur serveur et web automatiques à02:00 locale, après sauvegarde; thèmes,
Mac, NAS et réglages étrangers restent hors périmètre.

## Contrôles et frontières

- **Autorité et moindre privilège — PASS.** Le drop-in ajoute uniquement les
  trois récupérations root publiées et remplace `ExecStart` par le slot serveur;
  l'unité de base, son utilisateur non-root, `WorkingDirectory`, montages/NAS
  lecture seule, AF_NETLINK/Bonjour et durcissement restent en place. Le helper,
  sa configuration, sa clé publique, ses slots et son registre anti-rejeu sont
  root-owned; le serveur ne peut écrire que l'état/intention borné. Aucun sudo,
  compte, groupe, commande, URL, dépôt ou clé contrôlable depuis le LAN n'est ajouté.
- **Authentification, tenant, session, CSRF et redirects — N/A / inchangés.** Le
  choix foyer sans authentification de F-18/ADR0029 demeure; aucune route, cookie,
  session, callback, redirection ou frontière de tenant n'est modifiée. Un appelant
  LAN peut demander une opération signée ou une politique comme déjà accepté,
  mais ne peut fournir ni octets exécutables ni autorité root; le helper revérifie
  signature, compatibilité, séquence, destination et inactivité.
- **Provenance et anti-rollback — PASS.** Les octets locaux correspondent aux pins:
  helper `b9d52238…d281`, service `a1874229…a52d`, timer `cc60e261…8b0b`;
  binaire ARM64 attendu. Le plan exige Minisign externe sur les manifestes/archives,
  puis comparaison des hashes après staging root privé. Le nouveau ledger ne peut
  être créé qu'en l'absence de tout ledger antérieur et après preuve exacte des
  manifestes serveur/web0.7.2 séquence3; `pending:null`, jamais de remise à zéro.
- **Adoption legacy et isolement — PASS.** Le serveur actif et l'arbre web sont
  d'abord comparés intégralement aux artefacts signés (fichiers réguliers exacts,
  aucun extra/lien/spécial/traversal), puis copiés sans hardlink ni renommage dans
  les slots de base. Les trois racines `server/web/themes` doivent exister en
  root:root0755 avant validation systemd; thèmes conserve une racine/releases vide,
  sans `current` ni payload, bootstrap0.7.1 et politique manuelle. Cela satisfait
  les chemins/recover du helper sans activer ni modifier les thèmes.
- **Sauvegarde, concurrence et reprise — PASS sous portes.** Les snapshots sont
  root privés. La sauvegarde applicative est lancée hors ligne par le binaire
  existant sous UID/GID du service, dans un job systemd nommé, durable et unique;
  aucun slot/config/drop-in n'est muté avant sortie terminale0 et validation du
  reçu, manifeste, arbre régulier et hashes. La même exclusion sérialise
  opération, sauvegarde et récupération; absence SSH/stdout n'est jamais un
  résultat. Le plan crée le marqueur maintenance root0600, attend une pulsation
  liée à sa génération avec `quiescing=true`, `busy=false` et zéro mutation avant
  l'arrêt, puis abandonne et retire le marqueur si l'attente bornée échoue. Aucune
  récupération ne redémarre sur une sauvegarde vivante.
- **TLS, réseau, secrets et logs — PASS.** Santé uniquement en HTTPS strict avec
  copie root du certificat public existant et résolution privée fixe correspondant
  au listen/SAN; aucun `-k`, changement TLS, IP/clé privée/secret dans Git ou
  journal public. Configuration réelle, requête et transcript restent privés;
  les comptes rendus publiés ne contiennent que codes, hashes non secrets et UUID.
- **Politique et activation — PASS sous portes.** État absent: Mac garde son défaut,
  serveur/web deviennent `automatic 02:00`, thèmes `manual`. Les contrôles explicites
  serveur/web et le premier tick doivent conclure version courante/`NO_UPDATE` sans
  téléchargement ni réinstallation; ce tick consomme une seule admission locale du
  jour. Le timer persistant minutely n'est activé qu'après cette validation; le moteur,
  non le timer, applique l'horaire quotidien, DST, retry borné et busy deferral.

## Portes obligatoires avant/après mutation

1. Revalider immédiatement avant arrêt TLS strict, service actif, scan/TMDB,
   sockets et état `busy=false`; engager la quiescence existante ou garder
   atomique la transition vers l'arrêt. Refuser plutôt qu'interrompre une activité.
2. Conserver le verrou/job durable et attendre son vrai état terminal. Exiger
   sauvegarde offline exit0 et reçu complet avant toute conversion.
3. Écrire config, état et ledger par fichier temporaire root privé, `fsync`, rename
   et `fsync` parent; vérifier propriétaires/modes, trois racines et liens relatifs.
4. Comparer après installation les hashes helper/unités/drop-in et utiliser
   `systemd-analyze verify`, `daemon-reload`, propriétés effectives et chemins
   montés; ne jamais remplacer l'unité de base.
5. Démarrer le même payload0.7.2, vérifier TLS/API/version, web exact, données,
   réglages, unité et NAS non parcouru; puis `inspect`, checks et tick `NO_UPDATE`.
6. Activer le timer en dernier et faire vérifier indépendamment politiques,
   ledger, journal et prochaine admission. En cas d'échec avant timer: planification
   désactivée, quiescence prouvée, retirer seulement le nouveau drop-in et revenir
   au binaire plat conservé; ne jamais effacer le ledger.

## Preuves locales

| Preuve | Résultat | Limite |
| :--- | :--- | :--- |
| Git2.54, `git diff --check 7644d1d..50368f0`; arbre avant rapport | PASS | Trois nouveaux fichiers; pas le Pi. |
| Node24.13, `node --test scripts/test-legacy-update-slot-dropin.mjs` | **1 PASS** | Contrat lexical du drop-in, pas systemd Linux réel. |
| Cargo/Rust1.99.1, `cargo test -p chill-updates --locked` | **26 PASS**, 0 échec | Tests root Linux filtrés sur Darwin; runtime inchangé et déjà revu/publé. |
| SHA-256 + `file` sur helper/service/timer | Pins exacts; helper ELF PIE ARM64 | Pas d'exécution ARM64 locale. |
| Scan lexical secret/IP sur les trois fichiers du delta | Aucun résultat | Pas un scan d'entropie/historique. |

## Findings et limites

Aucun finding `CRITICAL`, `HIGH`, `MEDIUM` ou `LOW` n'est ouvert après ajout
explicite de la racine thèmes vide et des portes ci-dessus. La confiance dans le
helper/unités provient des revues et de la livraison signée0.7.2; ce rapport ne
prétend pas refaire leur audit complet.

**NOT_RUN:** mutation/activation Pi, sauvegarde offline finale, syntaxe systemd
réelle, premier tick/timer, installation future d'une version supérieure,
défaillance réseau/disque et coupure électrique pendant une future mise à jour.
Un succès `NO_UPDATE` ne valide ni ces cas ni un rollback réel.

