# ADR-0005 — Confiance TLS et exception après expiration

- Statut : `ACCEPTED` ; révision `CC-DOC-001` ; 2026-10-13. Validation : [G-02](../../gates/G-02-CC-DOC-001.md), proposition originale/hash conservés dans le manifeste historique.
- Propriétaire/auteur : `/root` ; reviewer : voir rapport indépendant.
- Baseline : `90c149b797b45cba8a56dca2b74eed06c4225060` ; commit Chill Center : `NOT_CREATED`.
- Supersedes/superseded_by : `NONE` ; exigences F-18–19 ; risque `SEC-001`.

## Contexte et options

Certificat auto-généré, approbation initiale, rotation annuelle et validité 18 mois exigés. L'utilisateur a accepté le risque de remplacement automatique après expiration sur son réseau familial. Sources : échange cité dans F-19, S-10/S-11.

| Option | Bénéfices | Risques/coûts | Réversibilité |
| --- | --- | --- | --- |
| Pin SPKI par serveur, clé stable et fallback après expiration | Vérification possible dès handshake, répond au besoin | Clé privée long terme ; fallback peut accepter un imposteur | Moyenne, pins révocables |
| CA locale stable, nouveaux certificats uniquement signés par elle | Renouvellement vérifiable | Ne permet pas nouveau certificat inconnu accepté après perte de CA | Moyenne |
| Certificat public | Vérification système | DNS/renouvellement externe et contraintes inutiles pour LAN | Élevée |

## Décision proposée

TOFU initial explicite ; magasin local par `server_id`/adresses et pin de la SPKI approuvée. Même clé de service pour un nouveau certificat tous les 12 mois, valable 18 mois : clé stable, certificat renouvelé. Au handshake, comparer SPKI et vérifier SAN/période/structure/signatures TLS, puis actualiser la validité connue. Pas de déclaration HTTP ou canal pré-handshake ajouté. Clé différente : confirmation explicite si l'ancien certificat enregistré reste valide ; acceptation automatique pour cette entrée s'il a expiré, événement visible et nouveau pin enregistré. API/vidéo/sous-titres utilisent ce même verifier Rust.

## Conséquences et validation

Sécurité : `SEC-001` demeure accepté fonctionnellement, pas supprimé par le chiffrement. DNS/IP et `server_id` ne prouvent pas l'identité dans le fallback. Même dans ce cas, vérifier SAN, nouvelle validité, structure et signatures TLS ; nouveau certificat expiré refusé même si SPKI connue. Clé long terme protégée par droits `0600`, compte de service et sauvegarde séparée, jamais export familial. Compromission/perte : remplacer clé/certificat et réapprouver les clients encore pinés à un certificat valide ; ne pas attendre son expiration. Horloge fiable ; tester boot après extinction et client absent au renouvellement. Aucun coût CA/cloud.

V-09 : simulation du calendrier, pins, signatures invalides et même politique du lecteur. Logs de rotation/acceptation sans secret. Migration : nouveau magasin de confiance distinct de Cinéfolio. Rollback : restaurer certificat et calendrier cohérents seulement si valides, clients éventuellement à réapprouver ; jamais accepter globalement pour dépanner. Nouveau mode de confiance doit superséder cet ADR et invalider les preuves TLS.

Revue initiale : [ADR-0005-review](reviews/ADR-0005-review.md), `CHANGES_REQUESTED`. Revue du delta : [ADR-0005-review-r2](reviews/ADR-0005-review-r2.md). Gate utilisateur : [TRACEABILITY](../../TRACEABILITY.md), étape 2 `APPROVED`, preuve [G-02](../../gates/G-02-CC-DOC-001.md) ; risque fonctionnel déjà explicitement accepté, implémentation non vérifiée.
