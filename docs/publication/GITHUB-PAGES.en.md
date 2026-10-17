# GitHub Pages preparation — 16 October 2026

[Français](GITHUB-PAGES.md) | **English**

Status: **prepared locally, not published**. The site is a static presentation,
not the web/PWA client and not a gateway to the home server. This preparation
requires no Pi or Mac deployment.

## Publication boundary

`scripts/build-public-site.mjs` assembles only:

| Source | Public destination |
| --- | --- |
| `site/index.html`, `api.html`, `404.html`, `styles.css`, `.nojekyll` | site root |
| `icons/branding/vector/default.svg` | `assets/logo.svg` |
| `docs/api/openapi-2026-10-14.json` | `api/openapi-2026-10-14.json` |
| `LICENSE` | `LICENSE.txt` |

A SHA-256 manifest is generated. No other repository file, configuration,
database, log or working document is copied. Site posters are fictional CSS
illustrations, not home media or TMDB images.

The build rejects symlinks in inspected paths, multiply hard-linked output
files, existing unmanaged destination files, personal paths, private network
addresses and some known key formats. It deletes nothing recursively and does
not reveal rejected values in diagnostics. This is a safeguard, **not a complete
secret or Git-history audit**. Allowed derived files are replaced on each build;
do not edit them manually or modify output paths concurrently with a build.

## Preview and checks

From the repository root:

```sh
node --test scripts/test-public-site.mjs
node scripts/build-public-site.mjs
```

Open `dist/public/index.html`. Relative paths allow hosting under `/chillcenter/`,
without a custom domain or application server. Optional local HTTP preview:
`python3 -m http.server 8080 --bind 127.0.0.1 --directory dist/public`; stop it
after use.

Optional browser check `scripts/test-public-site-browser.cjs` expects existing
Playwright (`CHILL_PREVIEW_NODE_MODULES`), a browser (`CHILL_CHROME_PATH` optional)
and an absolute `CHILL_PUBLIC_SITE_EVIDENCE` directory. It checks both colour
modes, several widths, links and the absence of remote loads. No product
dependency installation is added.

## Future publication requires a human decision

1. Approve public appearance and content. Check logo rights, links and exported OpenAPI.
2. **Separately audit repository visibility and its entire history** before
   making it public. Internal documents still contain household paths, addresses
   and operational details. Cleaning the README removes nothing from history.
   Do not implicitly change visibility or rewrite history.
3. Explicitly select the GitHub repository/plan for Pages. A private repository
   can have a public site depending on plan/settings; do not confuse site and
   source visibility. Without a suitable plan, consider a publication repository
   containing only the reviewed artifact, after approval.
4. Integrate reviewed files on the default branch, then configure
   **Settings → Pages → Source → GitHub Actions**. This has not been done.
   Configure `github-pages` environment protections and available approvers
   according to the plan.
5. Manually run `Prepare or publish public presentation`. With `publish=false`
   (default), it builds without deployment. Only after approval, select
   `publish=true` on the default branch. No `push`, PR or scheduled trigger exists.
6. After publication, verify the job's URL, navigation under `/chillcenter/`,
   API JSON and 404 page. The 404 home URL assumes the current GitHub repository:
   adapt it if the owner, repository name or domain changes.

The workflow contains no PAT: `contents: read` for building; `pages: write` and
`id-token: write` only for deployment, with a dedicated environment.
`configure-pages` does not enable Pages automatically. Official actions are
pinned to full SHAs; Node 24 LTS and Ubuntu 24.04 are used. Re-evaluate these
versions before a later publication.

Versions recorded as verified on 16 October 2026 from official GitHub references:
checkout 7.0.1, setup-node 7.1.0, configure-pages 6.0.0, upload-pages-artifact
5.0.0, deploy-pages 5.0.1. The workflow still needs actual execution on GitHub
after authorization; local tests do not prove remote environment permissions
or configuration. This translation does not claim a new upstream verification.

References: [official Pages workflows](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages),
[Pages plans and visibility](https://docs.github.com/en/pages/getting-started-with-github-pages/about-github-pages),
[Node.js lifecycle](https://github.com/nodejs/Release).

## Rollback

Before publication: abandon the branch or revert to the previous commit,
without touching delivered versions. After approved publication: rebuild an
earlier reviewed public commit's artifact and manually redeploy; disable Pages
in settings if necessary. Do not delete project data or history to remove a site.
