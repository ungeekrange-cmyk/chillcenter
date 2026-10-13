# Independent ADR Review

- **ADR**: `ADR-0005` at `CC-DOC-001`
- **Branch / Base Commit / Reviewed Commit**: `NOT_CREATED` / Cinéfolio `90c149b797b45cba8a56dca2b74eed06c4225060` / `NOT_CREATED`
- **Artifact Hash / Reviewed At UTC**: `536aaed14827a2dcb36a6eb3f53a61b735467247278872327798b7424d9ebb03` / `2026-10-13T15:00:47Z`
- **Author Instance**: `/root`
- **Reviewer Instance**: `/root/architecture_review`
- **Verdict**: `CHANGES_REQUESTED`

- [x] Le problème, le choix utilisateur et le risque `SEC-001` sont explicités sans réouvrir le périmètre.
- [x] Trois options sont évaluées équitablement.
- [x] L'exception reste limitée à l'ancien certificat expiré et au serveur configuré.
- [ ] Le chemin de récupération de la preuve de rotation est réalisable avant acceptation du nouveau certificat.
- [x] Migration, rollback prudent et observabilité sont abordés.
- [x] Les invalidations d'un changement de modèle de confiance sont indiquées.
- [x] Le risque fonctionnel a déjà été accepté, mais le gate d'architecture exact reste en attente ; l'ADR demeure `PROPOSED`.
- [x] Aucun lien de supersession n'est requis.

## Findings and Required Changes

| ID | Evidence | Risk | Required change |
| :--- | :--- | :--- | :--- |
| `ADR5-F-001` | `ARCHITECTURE.md`/ADR-0005 prévoient qu'un client absent, encore piné sur l'ancien certificat non expiré, vérifie une déclaration de rotation signée alors que le serveur actif sert déjà le nouveau certificat. Aucun transport pré-handshake, extension de certificat, chaîne signée ou chevauchement de service n'est défini. | Le client rejette le handshake avant de pouvoir télécharger la déclaration HTTP ; la rotation normale devient impraticable et bascule vers une confirmation manuelle ou attend l'expiration risquée. | Définir un mécanisme concret vérifiable avant acceptation HTTP (par exemple successeur embarqué et signé dans un objet accessible au vérificateur, chaîne/extension précisément validée, ou protocole de chevauchement explicite). Lier `server_id`, adresses/SAN, ancien et nouveau SPKI, fenêtres temporelles et anti-rejeu ; ajouter clients absents/redémarrages/rotations multiples à `V-09`. |

L'acceptation utilisateur du fallback après expiration est respectée et ne doit pas être redemandée ; ce finding concerne uniquement la continuité normale avant expiration.
