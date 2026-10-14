# Tampon configurable — remise Mac CC-CACHE-002

2026-10-14, source exacte `624f2447b5c0ede6b4a053ea3fa7b82db85c0121`, basePi/doc `c58da7e614b54890e0ef0207f4679d9b02cfca0e`. Auteur/intégrateur/root ; mode DEV_FAST borné, deux demandes utilisateur explicites (60s avant/30s retour, configuration en Mio), pas de réouverture de discovery ni de changement serveur.

## Utilisation

Administration → **Tampon vidéo de ce Mac** : avant16–512 Mio, arrière0–256 Mio, entiers, défaut128/64 Mio. 0arrière désactive la réserve de retour. Le réglage est propre à ce Mac et appliqué au prochain lancement de lecture ; une session déjà lancée conserve son snapshot pour reprise/enchaînement. La navigation Administration arrête déjà le lecteur par le comportement existant, que ce delta ne change pas. La sauvegarde seule n'agit pas à chaud sur libmpv. Aucune préférence de profil, URL ou configuration Pi modifiée.

`cache=yes/cache-secs=60/readahead=60/donate=no`. La durée arrière n'est pas paramétrable en secondes dans mpv : sa réserve64Mio vise environ30s jusqu'à18Mbit/s et peut conserver davantage à faible débit. Au-delà, et lorsque la connexion ne fournit pas assez vite, les durées sont moindres. Budget nominal192Mio de paquets par défaut,768Mio maximum configurable, **pas une limite du RSS total**. L'avance rapportée par mpv est une estimation peu fiable, jamais une garantie temporelle. [ADR-0010](../architecture/decisions/ADR-0010-client-playback-cache.md).

## Preuves exactes

- `cargo test --locked --offline -p chill-player -p chill-client -p chillcenter-desktop`, cache/toolchain privés, targetroot, `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1` : PASS69/69,32client+17player+20desktop,14,84sbuild,0,09/3,09/0,01sexécution, exit0 au SHA624. Logprivé SHA2562379e44cfb270d9955541d44b8ff3c9a9366d392e5fef43b541cdfb53ebcabc1. Valeurs effectives libmpv128/64 puis96/32 et48/0, validation/ancienJSON/persistance/échecIO/indépendance entre clients, contrôles HTTP/HLS et garde d'échéance couverts. Les premiers sockets refusés dans le sandbox ont été retestés hors sandbox, pas contournés dans le produit.
- `cargo clippy --locked --offline -p chill-client -p chill-player -p chillcenter-desktop --all-targets -- -D warnings` : PASS ; revue indépendante reproduite.
- Node contrôleur DOM réel et régressions :27PASS/0fail (cache3tests), distinct des tests Rust. Le mock d'invoke ne devient pas preuve d'une webview physique.
- Première relance workspace/all-targets : **BLOCKED_OUTILLAGE**, linker serveur `errno28 No space left on device`, conservée comme tentative échouée. Seul cache incrémental généré6Gio dans `/private/tmp/chillcenter-root-target/debug/incremental` supprimé, sources/bundles/vidéos préservés. Après retour effectif d'espace disponible, `cargo test --workspace --all-targets --locked --offline --quiet` avec `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1` a réellement rendu exit0/PASS259tests :67catalogue+32client+50domain+17player+71server+2CLI+20desktop,0fail/ignored. Source fonctionnelle exacte624 inchangée, seulement rapports/docs non compilés ajoutés. Ce deuxième succès ne transforme pas la tentative ENOSPC enPASS et ne se déduit pas des anciens255tests.
- Bundle `scripts/package-macos.sh` release locked/offline : PASS1min47,75Mio,48dylibsarm64/41formules, plist/lipo/closure/otool/signaturead-hocPASS. Deux avertissements QA-only nonblank/glReadPixels préexistants, aucun runtime/lock changé. `--verify-bundle` : PASS API2.5, bibliothèque embarquée seulement, avant ouverture de réglages/UI/réseau.

Octets signés : binaireSHA256`aa32c0958bf9b818a7f635bf1bf65e90dc9099ab24b4275d5ec7a6392733bd9d`, libmpvSHA256`a548453541616ee85970fd31da1aeafd7c7bf501a552e63502f6d69abbac31d7`. Bundle construit dans `/private/tmp/chillcenter-cache-624f244.PqljX6/Chill Center.app`, destination nouvelle `artifacts/macos/Chill Center-624f244.app`. Publication atomiqueNO_REPLACE effectuée, hash/signature/loaderpostremisePASS et [vérification indépendante](REL-MAC-CACHE-001-post-verify.md) APPROVED_SCOPED ; aucune notarisation/DeveloperID revendiquée.

## Revues et retour

[Code/tests/ADR exact624](../reviews/chillcenter-cache-code-test-624f244.md) APPROVED, SHA256bbbe5cc009515c094e645792b620b9f1d1db97cbed0729ccab931fcfbc8bd19c ; [sécurité exact624](../reviews/chillcenter-cache-security-624f244.md) APPROVED, SHA256125728bdde5c86dbd0194ceae544cc32d7caa3d80bd9caf5f1d4dec99401910e. Auteurs distincts du builder ; source27bd5b2, son code approuvé et son ADR CHANGES_REQUESTED conservés dans les deux rapports historiques, non réécrits. L'annotation architecturale résout le budget64Mio historique ; mémoireclient ancienneSTALERSS.

Retour : fermer le nouveau bundle et rouvrir l'ancien126da0a conservé ; aucun NAS/serveur/base à restaurer. **L'ancien client ignore le nouveau champ cache et peut le perdre lorsqu'il réenregistre client.json** : réappliquer les deux valeurs après retour au nouveau client. Ce risque concerne ce réglage local, pas la bibliothèque/progression sur le serveur. Sauvegarder ses réglages localement avant un rollback si besoin, sans copier de secret dans les rapports.

## Limites explicites

Essai3flux NAS lancé sur27bd5b2 (cache fixe identique aux défauts), pas une preuve des champs configurables624 ; résultats dans [médias Pi](REL-PI-001-media-evidence.md). Pas de montage NAS client, pas de transcodage ni réinstallationPi pour ce delta. Le serveur actuel conserve son binaireR7 126da0a vérifié.

NOT_RUN : inspection humaine du formulaire release, réouverture graphique et rendu/son avec nouveaux réglages sur des Mac physiques différents, RSS webview avec cache maximal, seek30s arrière prouvant absence de refetch, résistance pression mémoire, pannes/veille physiques. Les tests natifs de propriétés et la persistance Rust sontPASS, sans transformer ces limites enPASS. Réserve Wi-Fi/charge3 à conclure selon mesures réelles, pas tous133AC/US terminés sans ces preuves.
