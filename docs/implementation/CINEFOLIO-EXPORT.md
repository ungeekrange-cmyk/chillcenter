# Export Cinéfolio MIG-001 (candidate)

`legacy::convert_cinefolio_snapshot` accepts only a coherent SQLite backup produced by
`snapshot::coherent_snapshot`. It uses SQLite read-only handles and rejects source
schema versions above 1 before writing the new scratch Store. It never opens the
source with `Store::open`, reads settings/tokens, or accesses household data.

The conversion emits one synthetic `Cinéfolio` profile. Common catalogue metadata,
explicit saga order/artwork, and legacy playlist order are retained; watched/rating
become profile state. NAS/provider paths are excluded. Only an explicit custom image
may be copied by the caller-provided controlled-cache callback. The caller maps this
synthetic profile in the import UI; no household migration is automatic.

Bounds: 100,000 media, 1,000 collections/playlists, 1 MiB source row, 64 MiB document.
Invalid data fails the scratch conversion; source remains untouched. Archive creation
then uses the normal staged archive builder and its validation/import round-trip.
