# Revue indépendante de l'architecture Chill Center

- **Artifact set**: `CC-DOC-001`
- **Author Instance**: `/root`
- **Reviewer Instance**: `/root/architecture_review`
- **Reviewed At UTC**: `2026-10-13T15:00:47Z`
- **Source Baseline**: Cinéfolio 0.5.0, `90c149b797b45cba8a56dca2b74eed06c4225060`
- **Chill Center Commit**: `NOT_CREATED` (dépôt distant vide ; dossier documentaire non committé)
- **Verdict**: `CHANGES_REQUESTED`

## Révisions gelées

| Artefact | SHA-256 |
| :--- | :--- |
| `README.md` | `b28510d65f4c4fbeb8b4f59d507e0cb5821f231a0b3ee7d44d4bd8800fcfeb99` |
| `AGENTS.md` | `b9c4d4ac6cdbe7ae98219a25c7a3370ca143c25bd9340041a334c992b95461f2` |
| `docs/PROJECT_CONTEXT.md` | `9676a0a9614337a907bc3bafdafb0b0d1c17c1da79b9c788c5f9b5c8c6766a59` |
| `docs/discovery/SCOPE.md` | `2e5572adfb3a7a482bb79fcb38ed7667190f01bcd96b3af1ac72f6f5b124504d` |
| `docs/SOURCES.md` | `8f6e6e675469f28e0299299a71336d872dcd5f15d39c7e302d1e68b76342c558` |
| `docs/architecture/ARCHITECTURE.md` | `7a1d9db0d113a0bde44cc841c752a35323c1f0e8b8ef3e7d27248921d49bd2e9` |
| `docs/architecture/CONTRACTS.md` | `195ceca00192997a93460800b88cef6d6db11acaf982c2692a92c1a85ea29939` |
| `docs/architecture/decisions/ADR-0001-linux-rust.md` | `a64870690c596abc0d6d93496fef26345051e1a4d54cb9d12dac7a751957e5f5` |
| `docs/architecture/decisions/ADR-0002-data-identity.md` | `cf359fe9556fd7dfe1f10a050eb9b55a5aaabe436305cc91d16dd60f0d281498` |
| `docs/architecture/decisions/ADR-0003-playback-lease.md` | `fd1501bd5f1dc40c28555f273bbb83cacae66d6b5858ce8bd94cf3b45c26a7e6` |
| `docs/architecture/decisions/ADR-0004-native-player.md` | `a0f660d7650e02245d1eea577ddbecf783bb248211b0ad305a8cca7a892a7264` |
| `docs/architecture/decisions/ADR-0005-https.md` | `536aaed14827a2dcb36a6eb3f53a61b735467247278872327798b7424d9ebb03` |
| `docs/architecture/decisions/ADR-0006-portable-import.md` | `f306a3e578f0cd99be79e453aee930645519d4cd784ab4ff6e392197733a57bf` |
| `docs/architecture/decisions/ADR-0007-client-ai.md` | `08546f446d19a34fac98ebd18b3604564815d4c0a53e410b0135eee628b40e5a` |
| `docs/architecture/decisions/INDEX.md` | `4019d7c2fa24608850f132be8db5cf5f931d8845c3c6b4c0dda71813f855f5ee` |
| `docs/TRACEABILITY.md` | `cdb0a94551ff7be5c29e72425e66be6f8c22ce2f086010a339d0c44b04684991` |
| `docs/VERIFICATION.md` | `d81db9d8a8a6ef2ce190de38d6bb75bed16d92aa5737d3e01ae91474b9cd0929` |
| `docs/debug/INDEX.md` | `ef0d889cf77ebf22adf87477f3ff33cedc73f6d99284c0bf92865681c88f3368` |

## Conclusion

Le dossier respecte le périmètre utilisateur déjà validé : serveur Linux uniquement, client Mac Apple Silicon maintenu, NAS via le serveur, deux à trois lecteurs 1080p au plus comme cible à mesurer, LAN familial sans authentification, fallback de certificat inconnu après expiration explicitement assumé, aucun AWS/Azure/cloud ajouté. Il distingue correctement propositions et preuves exécutées, conserve tous les ADR en `PROPOSED`, et n'avance ni stories ni implémentation avant validation de l'étape 2.

La migration/import, les ressources Pi, les limites de scan et les objectifs mémoire sont suffisamment prudents pour cette étape : staging disque, sauvegarde pré-migration, NAS intact, travaux bornés, priorité aux lectures et benchmark réel obligatoire. Ils ne justifient pas un blocage documentaire.

Quatre lacunes matérielles empêchent néanmoins l'approbation de `CC-DOC-001` en l'état : bail non lié au média courant, deadline de transfert renouvelable non tranchée, contraintes/lifecycle de l'API `stream_cb` insuffisamment contractualisées, et déclaration TLS de rotation inaccessible avant handshake pour un client absent. Elles sont détaillées dans les revues `ADR-0003`, `ADR-0004` et `ADR-0005`.

## Findings consolidés

| ID | Sévérité | Artefacts | Correction exigée |
| :--- | :--- | :--- | :--- |
| `ARCH-F-001` | HIGH | ADR-0003, `CONTRACTS.md`, `ARCHITECTURE.md`, `VERIFICATION.md` | Lier bail/flux/événements au média et à sa révision ; invalider atomiquement l'ancien média à l'enchaînement. |
| `ARCH-F-002` | HIGH | ADR-0003, `CONTRACTS.md`, `VERIFICATION.md` | Figer une deadline non renouvelable de l'ancienne génération pendant `TRANSFERRING` et borner l'ACK final. |
| `ARCH-F-003` | HIGH | ADR-0004, `ARCHITECTURE.md`, `VERIFICATION.md`, `SOURCES.md` | Contractualiser l'instabilité/version minimale de `stream_cb`, ses contraintes de thread/lifetime/cancel/seek et leurs preuves d'enabler. |
| `ARCH-F-004` | HIGH | ADR-0005, `ARCHITECTURE.md`, `CONTRACTS.md`, `VERIFICATION.md` | Définir comment un client piné vérifie le successeur avant d'accepter la connexion TLS, avec anti-rejeu et liaison d'identité. |

## Verdicts ADR

| ADR | Verdict |
| :--- | :--- |
| ADR-0001 | `APPROVED` |
| ADR-0002 | `APPROVED` |
| ADR-0003 | `CHANGES_REQUESTED` |
| ADR-0004 | `CHANGES_REQUESTED` |
| ADR-0005 | `CHANGES_REQUESTED` |
| ADR-0006 | `APPROVED` |
| ADR-0007 | `APPROVED` |

Les verdicts `APPROVED` valent uniquement pour les octets et hash ci-dessus ; ils ne remplacent pas le gate utilisateur. Tous les ADR restent `PROPOSED`. Après correction, les artefacts modifiés recevront de nouveaux hash et une nouvelle revue ciblée ; aucune approbation ne doit être réutilisée silencieusement.
