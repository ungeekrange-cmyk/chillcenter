# Revue de code indépendante — BUG0127

- **Source revue :** `a38d4b16616ebd6735fa4aab0b518e28d767079e`
- **Base :** `6ba3970b314fb96325f906a5ccec2cde188c6ada`
- **Branche :** `codex/update-check-controls`
- **Reviewer :** `/root/v072_code_review` (distinct de l’auteur)
- **Mode :** `DEV_FAST`, revue de correction bornée
- **Verdict :** **APPROVED**

Cette revue porte exclusivement sur le correctif qui découple la vérification
des composants de la présence du helper root. Elle ne couvre ni publication,
ni DMG, ni installation Pi/web, ni NAS, ni réglages existants.

## Constats

Aucun constat ouvert de correction, transition d’état, concurrence,
compatibilité, maintenabilité ou élargissement de périmètre sur ce SHA exact.

## Analyse ciblée

- `crates/chill-server/src/updates_api.rs:132-169,219-227` dirige seulement
  `check` vers le chemin passif lorsqu’aucune configuration helper n’est
  présente. `install` conserve le refus `UPDATE_HELPER_NOT_INSTALLED` et les
  chemins helper/policies existants ne sont pas modifiés. Le statut expose
  séparément `helper_available` et la capacité additive `check_available`.
- `crates/chill-server/src/update_checks.rs:41-109` borne le travail à un bit
  par composant (trois au total) et s’appuie sur le quota persistant existant
  de cinq minutes. Les transitions Pending → Running → terminales sont
  explicites; une opération orpheline après redémarrage devient
  `CHECK_INTERRUPTED`, sans relancer un worker caché. Aucun verrou d’état
  n’est conservé pendant le transport ou la vérification.
- `update_checks.rs:112-165,175-236` isole les reçus d’affichage dans
  `updates/checks`; succès et absence de mise à jour réécrivent l’indice de
  façon atomique, tandis qu’échec/incompatibilité ne laisse pas un indice
  installable ancien. La découverte fixe le dépôt, vérifie la signature et les
  contraintes de compatibilité, ne télécharge aucun payload, et ne modifie ni
  le cache/registre anti-rejeu ni les intentions du helper.
- `ui/connected/updates.js:7-10,44-52` n’autorise la vérification sans helper
  que si le serveur déclare explicitement la capacité. Sauvegarde de politique
  et installation restent désactivées; Installer est caché sauf version SemVer
  stable strictement supérieure, et disparaît pendant/après un contrôle non
  concluant. Les versions restent rendues avec `textContent`.
- Le pont web conserve le corps fermé `{component}` et le même endpoint daté;
  aucune URL, tag, clé ou commande ne peut être injecté par le navigateur.
  Les versions Mac/serveur/web et séquences passent ensemble à 0.7.2/3,
  thèmes restant 0.7.1/2. La dépendance directe `semver` réutilise la version
  déjà verrouillée par le workspace.

## Vérifications du reviewer

- `git diff --check 6ba3970..a38d4b1` : PASS.
- Lecture du delta complet (21 fichiers), du contrat `docs/updates/CONTRACT.md`,
  de BUG0127, d’ADR-0029 et des routes/ponts associés : PASS de cohérence de
  périmètre et contrat.
- `cargo test -p chill-server passive_checks_report_results_without_helper_or_install_intents --locked` : PASS (1).
- `cargo test -p chill-server failure_or_no_update_cannot_leave_an_installable_stale_hint --locked` : PASS (1).
- `cargo test -p chill-server status_remains_readable_while_a_metadata_fetch_is_waiting --locked` : PASS (1).
- `node --test ui/connected/updates.test.js web/test/bridge.test.js` : PASS
  (24, 0 échec).

La revue sécurité indépendante exacte
`docs/reviews/BUG-0127-security.md` sur `a38d4b1` est **APPROVED**; elle est
consultée comme gate séparée, sans en dupliquer la certification. La revue
tests et les preuves de packaging/déploiement restent également des gates
séparés.

## Limites et disposition

Cette approbation n’est valable que pour
`a38d4b16616ebd6735fa4aab0b518e28d767079e`. Toute modification source rend ce
verdict caduc. Elle ne constitue pas une validation de la GUI native, d’un
artefact signé, d’une publication GitHub, d’un déploiement Pi/web ou d’un accès
NAS. Le rollback fonctionnel consiste à réinstaller les binaires précédents;
les binaires antérieurs ignorent le répertoire optionnel `updates/checks` et
aucune donnée utilisateur, identité TLS, politique existante ou configuration
root ne doit être restaurée.

---

## Appendice opérationnel — opérateur Pi/web 0.7.2

- **Delta opérationnel revu :** `3d27456a3e2c46fb2dc8937b4e00370f85d54470`
- **Parent :** `258518d`
- **Périmètre :** `scripts/pi-v07-legacy-server-web-operator.sh` et son
  harnais de test; le rapport d’exécution est lu comme contexte, non comme
  preuve d’une mutation Pi.
- **Verdict :** **APPROVED**

Le seul changement de comportement remplace l’égalité erronée
`required_server.min == version_cible` par une admission de plage inclusive :
`min <= version_cible <= max`. Les trois valeurs sont obligatoirement des
SemVer stables stricts `major.minor.patch`; aucune pré-release, métadonnée,
valeur manquante, type non-chaîne, composant incomplet, borne inversée ou
borne extérieure n’est admis. Ainsi, le manifeste web générique
`0.7.0..0.999.999` reste compatible avec le serveur 0.7.2 sans transformer la
borne basse en égalité.

La comparaison de l’API, des schémas, composants, version cible commune, clé,
hashes et cibles reste byte-identique hors de ce bloc. Les chemins de
préflight TLS/scan/sockets/SQLite, sauvegarde, reçu durable, arrêt/redémarrage,
remplacement atomique, contrôle de persistance et rollback/récupération ne
sont pas modifiés. Aucun paquet runtime issu de `a38d4b1` n’est reconstruit et
aucun Pi, web, NAS, réglage ou service n’est contacté par cette revue.

Vérifications du reviewer : `git diff --check 258518d..3d27456a` PASS;
inspection du diff restreint (seul bloc SemVer et test associé);
`python3 scripts/test-pi-v07-legacy-operator.py` PASS (**8/8**), incluant le
parcours de déploiement simulé, sauvegarde/rollback et les limites SemVer. Ce
verdict est attaché exclusivement à `3d27456a3e2c46fb2dc8937b4e00370f85d54470`;
tout nouveau changement d’opérateur exige une nouvelle revue.
