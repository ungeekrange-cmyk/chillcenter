# BUG-0122 — Pre-delivery manual operator guard defects

- Status: **FIXED_LOCAL / INDEPENDENT_REVIEWED**,2026-10-20.
- Category: newly developed deployment tooling, never used to stop/update the Pi.

Independent fixture/source reviews found unsafe recovery journal trust, missing
in-memory lease/quiescence admission, incomplete swap durability, backup-baseline
comparison mistakes and maintenance cleanup after failed rollback. Historical
R2/R3 refusals remain preserved, not reclassified as successful delivery.

Corrections through `16167979` and root test carrier `aba6402`: validate private
root/0600 journal and CLI paths/service/hash pins before recovery; nonce-bound
atomic maintenance publication; generation-bound fresh idle proof; compare backup
semantic snapshot; fsync staged metadata and both rename directories; never stop
on pre-stop busy abort or release maintenance after unverified rollback.

Versioned tests21/21 include actual temporary files/modes/links/renames/fsync
with only root UID/chown and system services mocked. Three independent R4
reviews approve exact `aba6402`. Real Linux backup/swap/recovery remains NOT_RUN;
only private upload and read-only preflight have occurred on the Pi.
