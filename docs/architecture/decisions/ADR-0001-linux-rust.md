# ADR-0001 — Serveur Linux Rust et client distinct

- Statut : `ACCEPTED` ; révision `CC-DOC-001` ; 2026-10-13. Validation : [G-02](../../gates/G-02-CC-DOC-001.md), proposition originale/hash conservés dans le manifeste historique.
- Propriétaire/auteur : `/root` ; reviewer : voir rapport indépendant.
- Baseline : Cinéfolio `90c149b797b45cba8a56dca2b74eed06c4225060` ; commit Chill Center : `NOT_CREATED`.
- Supersedes/superseded_by : `NONE` ; exigences F-01 à F-04.

## Contexte et options

Le serveur Mac a été retiré par l'utilisateur. Pi 3 B, 1 Go, NAS externe et Wi-Fi imposent de séparer catalogue/flux et décodage. Sources L-01/L-03 et S-01 à S-04 dans [SOURCES](../../SOURCES.md).

| Option | Bénéfices | Risques/coûts | Réversibilité |
| --- | --- | --- | --- |
| Service Rust Linux natif | Peu de composants, réutilisation du cœur, supervision système | Build ARM64 et montage Linux à vérifier | Élevée, API découplée |
| Conteneur Linux | Distribution homogène | Runtime et montage supplémentaires sur Pi | Moyenne |
| Application locale Cinéfolio inchangée | Aucun nouveau serveur | Ne répond pas aux profils/flux Linux ; macOS dépendant | Base conservée |

## Décision proposée

Workspace Rust ; serveur Axum/Tokio/rustls, Linux Lite 64 bits Debian 13 et systemd, sans Tauri ni FFmpeg encodeur côté serveur. Client Mac séparé ; web ultérieur. Rust 1.99.0 stable, versions compatibles verrouillées avant build. Montage NAS lecture seule géré par Linux ; base/cache local. Aucun cloud ni abonnement nouveau requis.

## Conséquences et validation

Positif : serveur sans décodage, dépendances macOS isolées. Coût : packaging de deux plateformes et réseau Wi-Fi à mesurer ; aucune garantie de trois flux par simple résolution. Sécurité : compte non privilégié, écoute LAN configurée, aucun transfert d'identifiants NAS aux clients. Observabilité : RSS, CPU, débit et états du montage, sans secrets. V-01/V-02/V-12 prouvent cible ARM64 et capacité réelle ; aucune preuve exécutée.

Migration : extraction depuis le commit de référence avec comparaison des fichiers de travail. Cinéfolio reste utilisable. Retour : ancien binaire et configuration seulement si schéma compatible, sinon snapshot pré-migration. Changement de topologie invalide contrats API, déploiement et benchmarks.

Revue : [ADR-0001-review](reviews/ADR-0001-review.md). Gate utilisateur : [TRACEABILITY](../../TRACEABILITY.md), étape 2 `APPROVED`, preuve [G-02](../../gates/G-02-CC-DOC-001.md).
