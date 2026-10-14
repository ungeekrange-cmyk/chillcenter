# BUG-0022 — Téléchargement d'archive sans longueur de corps

2026-10-14 · auteur `/root/admin_contract_fix` · état : FIXED_PENDING_INDEPENDENT_REVIEW.

## Symptôme et reproduction

Environnement : macOS arm64, Rust 1.99.0, branche `codex/admin-contract-fix`, source `d678bd4e0f7c8f08f2a90b63413b6fdc6c6390da`. Fixture QA privée isolée pour l'export; aucun NAS, archive du foyer, donnée utilisateur ou clé brute n'a été utilisé.

L'export serveur est prêt (un téléchargement direct de la fixture a retourné 5 994 octets), mais le transport client natif refuse une réponse ZIP sans `Content-Length` et l'interface signale `CONNECTION_FAILED` pendant export/enregistrement. La régression `archive_api::tests::export_route_streams_checked_portable_zip_without_secrets_or_paths` était RED : après l'état `ready`, le téléchargement HTTP 200 n'avait pas l'en-tête `Content-Length` et l'assertion a échoué.

## Hypothèse et cause confirmée

H1, confirmée par lecture et test : `download` ajoutait le type, la disposition et `no-store`, mais créait un `ReaderStream` sans fournir la longueur que le client attend. Le job d'export et son fichier staging étaient corrects; le défaut était limité à la réponse de téléchargement.

## Correction et régression

Le handler ouvre d'abord le ZIP service-owned, lit ses métadonnées depuis ce descripteur déjà ouvert, refuse un objet non régulier ou supérieur à `MAX_COMPRESSED`, puis fournit cette taille exacte en `Content-Length`. Le même descripteur est lu au plus sur cette longueur par `ReaderStream::with_capacity(..., 64 KiB)`; aucune lecture de métadonnée par chemin après ouverture n'est ajoutée.

GREEN : le test d'export existant vérifie désormais que `Content-Length` est exactement `ready.bytes`, en plus du ZIP portable déjà contrôlé, et passe (1/1). `cargo check --offline --locked -p chill-server` passe aussi dans le même cache/target isolé. La fixture QA fixe un fichier privé pour le seam IPC + flux HTTPS; elle ne prouve pas le sélecteur macOS réel.

## Retour et portée

Retour logiciel : revenir au parent du commit candidat retire uniquement l'en-tête, la validation du descripteur et la borne de lecture. Aucune relaxation TLS/client, aucun chemin arbitraire, aucune permission NAS ou modification du quota/import/staging n'est introduite. Exigences concernées : F-17, ADR-0009 et les limites d'archive de `docs/architecture/CONTRACTS.md`. Cette note ne constitue pas une approbation indépendante.
