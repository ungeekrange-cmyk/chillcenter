# Release 0.7.2 / Livraison 0.7.2

## Immutable inputs / Entrées immuables

- Mac/server/web source: `a38d4b16616ebd6735fa4aab0b518e28d767079e`, version 0.7.2, sequence 3.
- Integration carrier: `5cf1bbe70f1dd4ee98d67ae691334377df265b73`; runtime and packaging inputs remain those of a38d4b1.
- Themes remain 0.7.1, source `c603f29db2936ee40f7bf66484187b4e237d9bfd`, existing package and signature reused byte-for-byte.
- Reviewed deployment operator: `3d27456a3e2c46fb2dc8937b4e00370f85d54470`; SHA-256 `12a87083be5c18cdbabade89f8a248beb7952defe2f219d25138d0b4d4e56d0c`.
- Server ELF SHA-256: `cdde3ed064b79258ac17de3734c6750dd844e0f150bfcad8652f9d7e9b4cadea`.
- Mac DMG SHA-256: `1d80517bdf1f5e4e079c14bc7ad6398b83de6347fc582fccf204140416b364f5`.
- Web build: `ad12a307191b`. Signed manifests, provenance and SHA256SUMS bind the complete asset inventory.

## Scope and approval / Périmètre et accord

The user explicitly approved GitHub publication with the Mac DMG, followed by a private backup and server/web replacement on the household Pi. No NAS, application settings, TLS identity, systemd drop-ins, helper activation or scheduler change is authorized. No database migration is needed (schema 3, same dated API).

L’utilisateur a autorisé la publication GitHub puis le remplacement du serveur et du web après sauvegarde privée sur le Pi. NAS, réglages, identité TLS, cloisonnement systemd et services d’installation/planification sont conservés. Aucune migration de base n’est nécessaire.

## Gates / Contrôles

See `docs/reviews/BUG-0127-{code,tests,security}.md`, `docs/reviews/REL-0.7.2-operator-security.md` and `docs/integration/REL-0.7.2-review.md`. The latter must approve the immutable signed artifacts before publication or deployment. Operator changes only validate the declared inclusive server compatibility range; backup, recovery and preservation guards are unchanged.

Focused source checks: 147 server tests passed (one historical RSS test ignored), 198 Node checks passed, 23 bilingual-documentation checks passed, Clippy passed, isolated Chrome settings critical path passed. Real metadata-only signed release lookup passed locally without privileged installation authority. Native user GUI installation is not re-exercised in this release.

## Rollout and rollback / Déploiement et retour arrière

1. Publish the three component tags/releases and umbrella v0.7.2 from the exact artifact source, without a new themes tag. Independently check public assets, signatures and digests.
2. Run the reviewed combined legacy operator only after fresh strict-TLS and idle guards. Create a root-private consistent backup while the service is stopped, preserve old binary/web and a durable transition journal, replace only server/web and restart.
3. Independently verify hashes, versions, protected fingerprints, consistent data, rollback readiness and bounded update checks for server/web/themes. No media playback or metadata modification is necessary.
4. On failed health/invariants, use the operator’s durable recovery to restore the previous server/web pair and service; never overwrite unknown bytes. Preserve backup and journal for independent restoration verification.

La sauvegarde privée et l’ancienne paire serveur/web sont conservées. Un échec de santé ou d’intégrité impose le retour arrière prévu par l’opérateur, puis un contrôle indépendant. Les résultats réels sont consignés dans les rapports d’exécution et de vérification ; cette procédure ne vaut pas preuve d’un déploiement déjà effectué.

## Limitations / Limites

Mac: Apple Silicon, macOS 27+, ad-hoc signed, not Apple-notarized. Legacy Pi: manual update discovery works without the privileged helper, but automatic installation/scheduling remains unavailable until separately configured. Existing low/medium non-shipping dependency findings and the five GitHub permission alerts the user requested to retain stay open; no dismissal is implied.

## Observed final gate / Contrôle final constaté

**VERIFIED, independent post-deployment verdict APPROVED.** Four public releases
published, umbrella Latest at 2026-10-21T12:38:03Z; all 22 downloaded assets match
the signed inputs. CodeQL five-language run `39599937897` and shipping dependency
run `39599938107` passed on carrier `531bbb7` (no later runtime change).

Pi server/web 0.7.2 are active at the exact declared hashes/build. Fresh private
backup validates 1,924 manifest entries; consistent current/backup SQLite
comparison confirms all 29 application tables unchanged. Previous server/web
rollback pair is retained. Real strict-TLS server/web/themes checks return
`NO_UPDATE`, repeated check is cached, and install without the helper is refused.
See the [independent report](REL-0.7.2-verification.md).

FR : publication et installation réelles vérifiées ; NAS, réglages et TLS
conservés. L'incident de suivi SSH/récupération pendant la sauvegarde est
documenté, sans inventer de code de sortie terminal. Le service est sain et
stable ; [BUG0128](../debug/BUG-0128-operator-transport-and-recovery-overlap.md)
reste ouvert pour sécuriser la prochaine orchestration, pas pour modifier la
version applicative livrée. Aucun nouveau test du lecteur ou GUI Mac revendiqué.
