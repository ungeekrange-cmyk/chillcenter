# Contributing

[Français](CONTRIBUTING.md) | **English**

Read `AGENTS.md` and the project context. Work on an isolated branch; preserve
other contributors' configuration, changes and delivered versions.

Every fix describes expected behaviour, reproduction steps and executed tests.
Comment on invariants and non-obvious reasons, rather than explaining each
line. Avoid broad rewrites and unnecessary dependencies.

## Checks

```sh
cargo fmt --all -- --check
cargo test --workspace --all-targets --locked
node --test ui/connected/filters.test.js ui/connected/filters.dom.test.js ui/connected/catalog-loading.dom.test.js ui/connected/sync.dom.test.js ui/connected/tests/catalog-query.test.js
node --test scripts/test-public-site.mjs
node --test scripts/test-bilingual-docs.mjs
node scripts/build-public-site.mjs
git diff --check
```

Start with tests covering the change and record pre-existing failures
separately. Synthetic data proves neither real playback nor Pi/NAS capacity.

API contracts must remain compatible with existing Mac and web clients.
Do not publish a page, release or repository without explicit authorization.

Never commit or attach keys, user databases, real configuration, private
certificate keys or personal screenshots. Use anonymized examples. Internal
documents and Git history require an audit before making the repository
public; Pages uses its own allowlist of files.
