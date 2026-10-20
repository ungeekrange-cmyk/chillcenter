# ADR-0031 — One API version / Une seule version d’API

- Status: PROPOSED implementation, removal explicitly requested by user2026-10-20.
- Owner:/root. Supersedes only the legacy-alias retention in ADR-0011 and ADR-0029.
- Scope:Mac/server0.7.1; update web only if needed for compatibility.

## Context / Contexte

The public contract is already `/api/2026-10-14`. `/api/v1` is a second router
alias, not a separate data model, but advertises integer version1. Mac internally
constructs many v1 paths then rewrites them before sending. Server-generated
artwork paths and historical local catalogue mirrors may still contain v1 URLs.
Web networking already uses the dated prefix. Removing the route alone would
break stale cached artwork or scripts; a coordinated source audit is required.

L’utilisateur demande explicitement une seule version et la suppression des
anciennes, et autorise la mise à jour du client web si nécessaire. Cela remplace
la demande historique de conserver les alias ; pas de changement des DTO métier.

## Decision / Décision

Keep **2026-10-14** as the sole served API, sole OpenAPI base and sole identity
version. Delete the `/api/v1` router and supported-version entry. Retired routes
must return404, not redirect mutations or fall through to a static web file.
Construct dated paths in current Mac/server/domain runtime code and operational
scripts/tests. Preserve archival ADRs and release reports as historical evidence.

Where stored local data can contain an old artwork path, normalize that stored
reference to the canonical prefix at the client boundary, without another server
API or data reset. Keep this bounded read compatibility clearly distinguished
from accepting an obsolete endpoint over the network. Do not clear user data,
profile state, SQLite or artwork caches to migrate a URL.

Audit actual web requests and cached-path handling before declaring compatibility.
If a web source change is needed, bump its component to0.7.1 and deploy reviewed
web assets after backup. Do not replace a working web tree just to change a
displayed number when its networking already conforms.

## Verification and rollback / Vérification et retour arrière

Test canonical identity/OpenAPI/error envelopes, old GET/POST/HEAD routes404,
origin/Host protection and maintenance admission. Run current Mac/domain/server
tests, public contract validators and web bridge/network tests. Cover a cached
legacy artwork reference without catalogue loss or foreign-origin fetch.

Deploy only after client compatibility is proved. Existing Mac0.7.0 normally
rewrites v1 requests to the dated API already, but third-party users explicitly
calling v1 must migrate; document that intentional retirement. Retain old server
binary and any replaced web assets for rollback. No schema/config/TLS/NAS change.

Alternative retained aliases was explicitly rejected by the latest user request.
Creating a new date would add a version instead of unifying the API, so is rejected.
