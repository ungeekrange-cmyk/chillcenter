# Revue code indépendante — candidat 0.7.1 intégré

- **Mode / reviewer** : `FULL_ASSURANCE`, revue delta source non mutante / `/root/v071_code_final`
- **Base opérateur déjà approuvée** : `aba640284cd1dcbb27bfb4d0c1bec2b9deef3c82` / [R4](REL-0.7.1-code-review-r4.md)
- **Source runtime R4 antérieure** : `fdb80b502db45d10c5384104912f5f246678c32a`
- **Candidat initial revu** : `1cf7b42826b58719aed16603f6e47d66d580cc2a`
- **Candidat corrigé intermédiaire** : `f52c63016346e39ef4baed9667d2587f0a0cd67a`
- **Candidat final exact** : `c603f29db2936ee40f7bf66484187b4e237d9bfd`
- **Branche** : `codex/mac-server-discovery`
- **Date** : `2026-10-21`
- **Verdict historique 1cf7b42** : **CHANGES_REQUESTED**
- **Verdict intermédiaire f52c630** : **APPROVED_SCOPED** (code)
- **Verdict courant c603f29** : **APPROVED_SCOPED** (code)

Le delta R4 reste réutilisable pour l'opérateur binaire inchangé. Le nouveau
delta ajoute l'opérateur réversible `AF_NETLINK`, le rendu2 des ambiances, les
Blob URLs natifs bornés, les métadonnées0.7.1/séquence2 et les corrections de
packaging. La revue ne contacte ni Pi, NAS, GitHub, web/configuration déployés.

## Findings

### `CR-071-FINAL-001` — HIGH — le pack rendu2 signé ne passe pas sa santé native

`safe_asset()` accepte `assets/decorations/*.png`, mais le parcours du pack
installé refuse encore le dossier `assets/decorations`
(`apps/desktop/src/theme_bridge.rs:89`, `:276-305`). Le pack0.7.1 contient
précisément `assets/decorations/rabbit-silhouette.png`. Son installation atteint
ensuite `ThemeHooks::health_check`, qui appelle `validate_directory()`
(`apps/desktop/src/update_bridge.rs:1221-1243`) : la santé échoue et
`apply_staged` restaure l'ancien slot. Le seul test du pont charge le manifeste
**embarqué** et ne parcourt donc jamais l'arbre externe
(`apps/desktop/src/theme_bridge.rs:370-378`).

**Correction requise** : admettre exactement `assets/decorations` dans le
walker fermé et ajouter un test de santé du vrai arbre temporaire
`manifest.json + assets/{logos,backgrounds,decorations}`. Le test doit prouver
`validate_directory == true`, puis que `load()` sélectionne le slot externe et
matérialise les18 assets, et conserver les refus symlink/fichier non déclaré.

### `CR-071-FINAL-002` — MEDIUM — la limite annoncée de128 assets est impossible

Les deux walkers comptent fichiers **et** répertoires. Avec128 assets valides,
le manifeste et les quatre répertoires `assets`, `logos`, `backgrounds`,
`decorations` donnent133 entrées, alors que les plafonds sont132
(`apps/desktop/src/theme_bridge.rs:285-299`,
`crates/chill-updates/src/theme.rs:406-418`). Le manifeste autorise pourtant128
assets (`crates/chill-updates/src/theme.rs:13-16,47-52`). Le pack courant de18
assets n'atteint pas la limite, mais le contrat signé et ses deux validateurs
divergent à la borne.

**Correction requise** : dériver le plafond depuis `MAX_ASSETS` et le nombre
exact de répertoires/manifeste admis (133 ici), avec tests à128 PASS et129 REFUS.

### `CR-071-FINAL-003` — MEDIUM — grammaire signée différente entre installateur et renderer

Le renderer JS interdit `calendar` sur un thème statique et toute `decoration`
hors `easter` (`ui/connected/theme-pack.js:27-35`). Le validateur partagé Rust
n'interdit ni `calendar` ni `decoration` sur les six IDs statiques : sa branche
statique lit seulement `start/end/priority` (`crates/chill-updates/src/theme.rs:167-225`).
Un pack signé ainsi peut donc passer extraction et santé native, puis être refusé
par `ChillThemePack.validate()` et retomber silencieusement sur le pack embarqué.
Le manifeste0.7.1 courant respecte bien la grammaire ; le défaut porte sur le
contrat d'update accepté.

**Correction requise** : fermer la grammaire Rust comme la grammaire JS : aucun
`calendar`/`decoration` sur les thèmes statiques, `calendar` exact sur les deux
événements et `decoration` uniquement sur Easter. Ajouter les mêmes cas négatifs
aux deux validateurs.

### `CR-071-FINAL-004` — LOW — échec total de reload conserve les Blob URLs actifs

Le chemin native→fetch absolu→fetch relatif ne révoque les URLs que lorsqu'un
fetch réussit (`ui/connected/theme-pack.js:13-20,44-46`). Si les trois sources
échouent, la promesse rejette et `appearance.js` passe au SVG de secours sans
effacer `data-theme-background`, `--event-art` ou `--event-rabbit`
(`ui/connected/appearance.js:5-8,32-47`). L'ancien pack peut donc rester affiché
et ses URLs rester vivantes après un reload en erreur, contrairement au contrat
documenté « reload/error revokes them ».

**Correction requise** : définir explicitement le comportement atomique d'échec.
Si le repli visuel est choisi, appeler une remise à zéro/revocation avant le SVG ;
si l'ancien pack doit être conservé, documenter ce choix et tester qu'il reste
cohérent sans fuite au prochain succès. Ajouter le cas « native + deux fetches
échouent après un pack Blob appliqué ».

## Contrôles et deltas réutilisés

| Contrôle | Résultat / portée |
| --- | --- |
| `git rev-parse HEAD`, merge-bases | candidat exact `1cf7b42`; bases `aba6402` et `fdb80b5` confirmées |
| `git diff --check aba6402..1cf7b42` | **WARN documentaire uniquement** : ligne vide finale déjà signalée, sans effet source |
| `python3 -m unittest -v scripts/test_pi_bonjour_family_operator.py` | **9/9 PASS** ; fichiers temporaires, systemd/root simulés |
| `node --test scripts/test-component-release.mjs` | **7/7 PASS** ; regex tag fixe et compatibilité renderer2 incluse |
| `node ui/connected/theme-pack.test.js` | **PASS** ; rendu1 rétrocompatible, calendriers2026/2027/2038 et succès→succès Blob |
| tests calendrier/silhouette ciblés | **6/6 PASS** |
| Preuves fournies du candidat | 442 Rust PASS,2 NFR ignorés ;337 Node PASS ; navigateur8 modes/images hashées/cutout PASS. Réutilisées comme contexte, certification finale laissée au test reviewer. |

L'opérateur Bonjour est borné à sa drop-in, vérifie l'ensemble effectif exact,
conserve son journal sur échec et ne retire qu'un fichier owner/mode/hash conforme.
Les9 tests couvrent succès, refus de dérive, restart échoué et reprise
`prepared`; l'exécution root/Linux et le rollback couplé au vrai binaire restent
des preuves de release, pas de cette revue.

Le calendrier utilise deux tokens fermés. Pâques est recalculée annuellement par
dates civiles, Saint-Valentin est fixe et leurs fenêtres ne se chevauchent pas ;
la validation exhaustive versionnée ne couvre toutefois que2026–2038. Aucun
défaut du manifeste courant n'a été trouvé après2038 par inspection de la formule,
mais cette limite de preuve doit rester explicite.

Les captures réellement assemblées clair/sombre et la comparaison80×64 ont été
inspectées selon `character-proportions` : raster80×64 monochrome, rendu CSS
uniforme64×51,2 (facteur0,8), réflexion/translation seules, aucun étirement face
à la référence de travail. Les membres cachés restent non vérifiables et cette
référence n'est pas une anatomie approuvée par l'utilisateur. Le cutout lecteur
retire le fond événementiel ; les décorations restent confinées au panneau.

Le web0.7.0 déployé et ses octets ne sont pas modifiés par l'opération serveur ;
le nouveau pack thèmes déclare correctement `renderer_min=max=2`, tandis que le
renderer2 accepte encore les packs1. Versions `mac/server/web/themes=0.7.1` et
séquences2 sont cohérentes. Packaging final reste **PENDING** et n'est pas
requalifié par cette revue.

## Verdict et routage

`1cf7b42826b58719aed16603f6e47d66d580cc2a` est **CHANGES_REQUESTED**. Les
constats sont statiques et localisés : retour builder, puis nouveau SHA exact et
nouvelles revues code/test/sécurité. Ne pas publier les thèmes0.7.1 ni présenter
ce SHA comme candidat final. Les approbations R4 historiques restent valides dans
leur périmètre mais le gate combiné est `STALE` pour cette source.

## Revue delta intermédiaire — `f52c63016346e39ef4baed9667d2587f0a0cd67a`

Le correctif figé contient uniquement les causes localisées et leurs régressions :

| Finding historique | Disposition au SHA f52 | Preuve code |
| --- | --- | --- |
| `CR-071-FINAL-001` | **CLOSED** | `theme_bridge` admet uniquement `assets/decorations`; le test crée le pack rendu2 complet, valide le dossier, charge le symlink `current` et distingue la version externe `9.9.9` du fallback (`apps/desktop/src/theme_bridge.rs:276-407`). |
| `CR-071-FINAL-002` | **CLOSED** | plafond corrigé à128 assets + manifeste + quatre dossiers =133 dans le pont, et `MAX_ASSETS + 5` dans le validateur partagé; tout dossier supplémentaire est refusé (`apps/desktop/src/theme_bridge.rs:285-312`; `crates/chill-updates/src/theme.rs:406-448`). |
| `CR-071-FINAL-003` | **CLOSED** | Rust refuse désormais `calendar`/`decoration` sur les IDs statiques, avec négatifs Rust et JS appariés (`crates/chill-updates/src/theme.rs:167-230,542-585`; `ui/connected/theme-pack.test.js:17`). |
| `CR-071-FINAL-004` | **NON_BLOCKING / clarification** | un échec total ne publie aucune nouvelle génération Blob : conserver l'unique génération connue évite de casser les pixels déjà appliqués; le prochain succès la révoque avant publication et la destruction du document libère le reste. Le comportement est borné, sans croissance par échec. La formule documentaire « error revokes » doit être comprise comme erreur de génération partielle/succès de repli, pas comme destruction du dernier rendu valide. |

Le delta ferme aussi le risque adjacent du parent de drop-in : un dossier root
mais modifiable groupe/autres (`0775`/`0777`) est refusé avant création ou
`daemon-reload`; les permissions existantes ne sont jamais réécrites
(`scripts/pi-bonjour-family-operator.py:177-191`). Cela renforce l'invariant
« seule AF_NETLINK » sans élargir le rollback.

### Preuves indépendamment rejouées sur `f52c630`

| Commande | Résultat |
| --- | --- |
| `git diff --check 1cf7b42..f52c630` | **PASS** |
| `python3 -m unittest -v scripts/test_pi_bonjour_family_operator.py` | **10/10 PASS**, dont parents0775/0777 |
| `node --test scripts/test-component-release.mjs` | **7/7 PASS** |
| `node ui/connected/theme-pack.test.js` | **PASS**, négatifs de grammaire et cycle Blob inclus |
| `cargo test -p chillcenter-desktop theme_bridge::tests -- --nocapture` | **4/4 PASS**, vrai arbre externe inclus |
| `cargo test -p chill-updates theme::tests::metadata_refuses_gaps_ambiguous_dates_and_active_tokens -- --exact` | **1/1 PASS** |

Les preuves larges442Rust/337Node et navigateur8 modes appartiennent au
prédécesseur et restent réutilisables pour les chemins non modifiés ; les cinq
exécutions ciblées, complétées par le `diff --check`, couvrent le delta source
exact. La certification de la suite globale et des artefacts reconstruits reste
au test/release reviewer.

### Verdict intermédiaire

**APPROVED_SCOPED** pour le code exact
`f52c63016346e39ef4baed9667d2587f0a0cd67a`. Aucun finding code bloquant ne
reste ouvert dans le périmètre calendrier/rendu2, Blob/CSP, packaging de
manifestes, compatibilité web0.7.0, opérateurs binaire/AF_NETLINK et rollback.
Cette décision ne prouve ni packaging, signature, upload, exécution root/Pi,
découverte LAN réelle, ni post-déploiement. La promotion exige encore les
verdicts test et sécurité du même SHA et les contrôles release prévus.

## Revue delta terminale — `c603f29db2936ee40f7bf66484187b4e237d9bfd`

`f52c630` est bien ancêtre du candidat final. Le delta est limité à cinq fichiers
et ferme le dernier constat LOW sans modifier le manifeste, le calendrier, les
opérateurs, le serveur ou le contenu des PNG :

- `theme-pack.load()` révoque désormais l'unique génération de Blob URLs active
  lorsque le pont natif **et** les deux fetches de repli échouent ; il relance
  ensuite l'erreur, sans accepter de Blob arbitraire ni élargir la CSP ;
- le `catch` existant convertit cette erreur en pack nul, puis `apply(null)`
  supprime `data-theme-background`, `--event-art`, `--event-rabbit` et les tokens
  du pack avant d'appliquer un logo historique ;
- ce logo de repli est borné aux six IDs historiques connus. `easter`,
  `valentine` ou une valeur inattendue utilisent `branding/default.svg`, ce qui
  évite de construire un chemin arbitraire ou inexistant depuis la réponse du
  pont.

`CR-071-FINAL-004` est donc **CLOSED** : un succès natif suivi d'un échec total
révoque le Blob précédent et retire le backdrop/décoration déjà appliqués. La
révocation est idempotente (`nativeBlobUrls=[]`) et ne crée ni boucle ni
croissance par échec. Le prochain chargement peut repartir d'un état propre.

### Preuves ciblées sur `c603f29`

| Contrôle | Résultat |
| --- | --- |
| ascendance `f52c630` → `c603f29`, inventaire du delta | **PASS** ; 5 fichiers attendus, 11 insertions/3 suppressions |
| `git diff --check f52c630..c603f29` | **PASS** |
| `node ui/connected/theme-pack.test.js` | **PASS** ; succès natif→échec total, révocation du second Blob et rejet de son ancienne URL |
| `node scripts/test-appearance-v05.cjs` dans cet environnement de revue | **NON EXÉCUTABLE** ; module `playwright` absent, aucune erreur produit observée |
| preuve navigateur fournie sur le candidat | **PASS** ; huit variantes événement/tonalité plus échec total, backdrop et lapin effacés |

L'absence locale de Playwright empêche seulement une seconde exécution de la
preuve navigateur ; elle ne remplace pas son résultat fourni. Les preuves plus
larges déjà consignées restent réutilisables car ce delta ne touche que le
nettoyage terminal et son test.

### Verdict courant

**APPROVED_SCOPED** pour le code exact
`c603f29db2936ee40f7bf66484187b4e237d9bfd`. Aucun finding code bloquant ou LOW
ne reste ouvert dans le périmètre assigné. Cette décision reste distincte des
gates packaging, signature, upload, exécution root/Pi, découverte LAN réelle et
post-déploiement ; les verdicts test et sécurité doivent viser ce même SHA.
