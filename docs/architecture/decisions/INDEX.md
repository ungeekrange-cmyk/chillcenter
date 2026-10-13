# Index des décisions techniques

Révision `CC-DOC-001`, 2026-10-13. Auteur et propriétaire : `/root`. Tous les ADR sont `ACCEPTED` par validation utilisateur de CC-DOC-001, sans prédécesseur ni successeur. La preuve est [G-02](../../gates/G-02-CC-DOC-001.md) ; les hashes historiques de revue portent sur la proposition approuvée. Les hash exacts de revue figurent dans les rapports.

| ID | Décision proposée | Exigences | Statut |
| --- | --- | --- | --- |
| [ADR-0001](ADR-0001-linux-rust.md) | Service Linux Rust, installation native et client distinct | F-01–04 | ACCEPTED |
| [ADR-0002](ADR-0002-data-identity.md) | SQLite locale, états par profil, UUID stables | F-05–06, F-14, F-16 | ACCEPTED |
| [ADR-0003](ADR-0003-playback-lease.md) | Bail exclusif et transfert confirmé | F-06–08, F-12–13 | ACCEPTED |
| [ADR-0004](ADR-0004-native-player.md) | Tauri conservé, lecteur libmpv intégré et transport Rust | F-02–03, F-09–11, F-22 | ACCEPTED |
| [ADR-0005](ADR-0005-https.md) | Confiance initiale et rotation TLS, exception après expiration | F-18–19 | ACCEPTED |
| [ADR-0006](ADR-0006-portable-import.md) | Archive portable JSON/affiches et import atomique | F-16–18 | ACCEPTED |
| [ADR-0007](ADR-0007-client-ai.md) | IA sur Mac, plans validés et appliqués par le serveur | F-14–15, F-18 | ACCEPTED |

Déclencheurs : frontière serveur/client, persistance, identité, contrat de session, dépendance native, chiffrement/exception, migration et fournisseur externe. Le calendrier et les palettes statiques sont des règles fonctionnelles déjà approuvées, documentées dans l'architecture, sans ADR distinct supplémentaire.

Après acceptation, changer une décision par un nouvel ADR avec `supersedes`, pas par une réécriture. Invalider les stories ou preuves qui consomment une décision modifiée.

Complément d'implémentation, distinct des sept décisions approuvées en G-02 : [ADR-0008](ADR-0008-bounded-media-io.md), `IMPLEMENTED_DETAIL`, I/O NAS bornées et ouverture par descripteurs, CC-PLAY-001. Ne remplace aucun ADR accepté ; pas de nouvelle validation utilisateur revendiquée.
