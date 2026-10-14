# Sonde opérateur Pi / NAS

Harnais distinct des binaires livrés. Il utilise le transport épinglé et le lecteur libmpv/FFmpeg du produit. Il ne lit jamais un chemin NAS depuis le Mac. Installation réelle et scan2496 confirmés14octobre2026 ; [preuves](../../docs/releases/REL-PI-001-media-evidence.md) des troisformats, deuxHD30min et troisfluxDune/720p : capacitétriple **nonvalidée**, Dune2progression<80%. La propriétébuffered_ahead_seconds est une approximation mpv, pas une garantie60s. L'optionlocaleMio du client graphique n'est pas chargée par ce harnais ; il emploie PlayerOptions::default128/64Mio.

Avant usage : installation vérifiée sur192.168.1.42:8443, certificat public récupéré via SSH approuvé puis empreinte SHA256 SPKI vérifiée. Ne jamais extraire la clé privée. `--authorized-pi-tests` et l'empreinte exacte sont obligatoires. Aucun `-k` ni remplacement de la confiance de l'application.

```sh
cargo build --locked --offline --manifest-path scripts/pi-probe/Cargo.toml
cargo run --locked --offline --manifest-path scripts/pi-probe/Cargo.toml -- \
  --authorized-pi-tests EMPREINTE_SPKI_VERIFIEE prepare
cargo run --locked --offline --manifest-path scripts/pi-probe/Cargo.toml -- \
  --authorized-pi-tests EMPREINTE_SPKI_VERIFIEE play PROFIL_QA_ID VIDEO_ID 1800
```

`prepare` refuse un serveur ayant déjà des profils, crée trois profils `QA Pi <nonce>`, lance le scan NAS lecture seule et fournit trois identifiants opaques disponibles. Ne pas relancer cette étape aveuglément après un échec : conserver les identifiants des profils de test. Aucun import de métadonnées Cinéfolio ou mutation des fichiers vidéo.

`play` n'accepte que ces profils de test ; bail, RangeHTTPS, décodage natif headless (sorties audio/vidéo nulles), pistes intégrées, sept vitesses, seek, heartbeat/progression, reprise par second identifiant d'appareil et refus du bail libéré. Les sélections de pistes ne prouvent pas l'écoute ou le rendu humain ; aucune durée/lecture/vitesse/PASS n'est inférée d'une compilation. Le crédit vu est zéro ; seule la progression du profil de test est modifiée. Une fin prématurée ou une progression inférieure à80% du temps réel est un échec, pas une endurance réussie.

Pour ENABLER-006, identifier d'abord de vrais échantillons720p/1080p et durées suffisantes ; effectuer deux puis trois lectures réellement simultanées pendant30min, avec mesures CPU/RSS/température/mémoire/débits du Pi, hors scan/empreinte lourde. Les premiers trois items ne garantissent ni ces résolutions ni la couverture MKV/MP4/AVI ; ne pas revendiquer la matrice sans identifier les échantillons. Ce harnais seul ne mesure pas les dimensions, la mémoire du webview, l'isolation de profils, les sous-titres externes, le son/renduAppKit, EOF/enchaînement ni les pannes réseau/backup.

Après essai, supprimer uniquement les trois profils créés, avec leurs IDs notés et confirmation de suppression API ; conserver le catalogue et toutes les données du foyer. Ne pas démonter le NAS, redémarrer le Pi ou couper son alimentation pendant les tests sans accord spécifique.
