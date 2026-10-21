# Easter / Valentine proposals — Propositions Pâques / Saint-Valentin

Status / Statut: **BACKGROUND APPROVED, animation preview pending / FONDS VALIDÉS,
animation à présenter**. Easter final revision and Valentine background approved
explicitly by the user2026-10-20. No runtime integration yet.
Open `events.html`; switch event, light/dark and civil year. Nothing in this directory
is loaded by the application, shipped web client or theme pack.

Ouvrir `events.html` ; choisir événement, clair/sombre et année. Ce dossier ne modifie
ni l’application, ni le client web livré, ni le pack de thèmes.

- Easter / Pâques: Western Gregorian Easter Sunday, 30 calendar days beforehand
  through Sunday inclusive, seasonal return Monday. Recomputed annually, never a
  fixed April date. Pâques occidentales recalculées annuellement ; retour le lundi.
- Valentine / Saint-Valentin: January 31–February 14 inclusive (14 days beforehand),
  seasonal return February15. Du31janvier au14février inclus ; retour le15février.
- Automatic event overrides seasonal ambiance; manual selection and disabled
  ambiances still win. L’événement automatique prime sur la saison, pas sur le choix
  manuel ni la désactivation. Existing Halloween/Christmas behavior is unchanged.
- Right-anchored illustrations; light Easter sage/ivory, dark forest/sage;
  Valentine light warm terracotta/ivory rather than pink, dark deep burgundy.
  Illustrations ancrées à droite, palette sauge/crème ou bordeaux/terre cuite.

Assets: generated with the built-in image generation tool on2026-10-20, using the
existing Christmas living room only as a style/composition reference. No characters,
text or logos in the background. Easter final: enormous colorful eggs, no flowers
or floral motifs, chocolate on the table, warm daylight; initial unapproved image
retained as `assets/easter-initial.png`. Valentine: deep red roses, candles and velvet cushion to the
right, warm evening. The original generated files are preserved outside the repo.

Calendar proof: `node --test docs/design/concepts/2026-10-20/calendar.test.cjs`.
UTC civil-date arithmetic avoids DST shifts. This is a design-preview calculator,
not an accepted runtime theme implementation.
Primary reference: [US Naval Observatory — Gregorian Easter rules](https://aa.usno.navy.mil/faq/easter)
and [2027 calendar, Easter March28](https://aa.usno.navy.mil/calculated/observances/christian?year=2027),
checked2026-10-20. The preview therefore starts February26 in2027, not a fixed
March/April date reused for later years.

## Sidebar animations / Animations du panneau gauche

### Revision2026-10-21 — solid silhouettes / silhouettes unies

User feedback: the outlined rabbits are rejected; each rabbit must be a plain,
single-color shape. Changed **only the preview** to `assets/rabbit-silhouette.svg`:
no stroke, eye, nose or internal decoration. Light mode uses sage; dark mode a
single light tone. The old working reference remains untouched, not replaced by
a new model to hide a proportion defect. No new raster generation was needed:
the existing editable vector preserves the intended anatomy and is lighter.

Retour utilisateur : contours/détails refusés, forme unie demandée. La silhouette
reprend exactement les sept formes corporelles et leurs transformations du
gabarit80×64 ; aucune tête, oreille, patte ou partie du torse n’a été redimensionnée.
L’ancien gabarit reste une référence de travail, pas une anatomie validée.
SHA256 de la silhouette :
`358e7f71bec17ca0da2700af744a690c19c5ab4b76944a3254d3014d0124c508`.

Focused static proof / Contrôles ciblés :
`node --test docs/design/concepts/2026-10-20/rabbit-silhouette.test.cjs docs/design/concepts/2026-10-20/calendar.test.cjs`.
Optional isolated browser proof: `rabbit-preview.qa.cjs`, with the existing
Playwright module directory/browser and absolute output directory supplied through
`CHILL_PREVIEW_NODE_MODULES`, `CHILL_PREVIEW_BROWSER`, `CHILL_RABBIT_EVIDENCE`.
It checks one connected silhouette, one source fill (raster antialias rounding
±1/255), transparency, both directions/modes at five hop phases, unchanged
64×51.2px scale and reduced motion. Screenshots and the comparison page are
inspected separately; passing technical checks alone is not visual approval.

Executed2026-10-21: static/calendar tests6/6 PASS; isolated Chromium QA PASS
(one connected shape,20 pose checks, reduced motion and Valentine unchanged).
The equal-scale comparison and assembled light/dark sidebar screenshots were
visually inspected: contiguous flat silhouettes, unchanged relative anatomy,
no outline or internal details. Local evidence: `rabbit-comparison.png`,
`easter-solid-light.png`, `easter-solid-dark.png` in the private QA output
directory. User approval of this new silhouette is still pending.

Exécution2026-10-21 :6/6 tests statiques/calendrier et20 contrôles de pose
réussis ; captures comparatives et panneaux clair/sombre réellement inspectés.
Silhouettes unies et proportions relatives conservées. Validation visuelle
utilisateur encore attendue avant intégration.

The preview check also exposed a more-specific mirrored-rabbit selector that
overrode `animation:none` in reduced-motion mode. The targeted selector is now
covered by the same reduced-motion browser assertion. No application/player,
Mac bundle, API, server, web package, Pi or NAS change belongs to this revision.

Ten light CSS petals fall for Valentine, and two rigid SVG rabbits move with a
subtle hop for Easter. Pointer-transparent, clipped to the sidebar, no timers,
and `prefers-reduced-motion` pauses movement. Preview only, not deployed.
Dix pétales CSS et deux lapins SVG, sans interception des clics ; les préférences
de mouvement réduit sont respectées. Aucun code du client web livré n’est changé.

Character working reference / Référence de travail (not user-approved):
`assets/rabbit-reference.svg`, SHA256
`a9acb6bdf74f2d6f2ee294826e7f07be6486c8e1bfbe3fac0d4631ffee2dfe32`.
The comparison at `rabbit-comparison.html` was rendered and inspected at the same
80×64 source-unit scale: contact, suspension and reflected direction retain the
same skull22units, torso40×26 and ears24units. Production preview uses uniform
0.8× display scale (64×51.2px), never per-part resizing. Only whole-body translation
and reflection are interpolated: anatomy does not vary between keyframes.
Hidden legs in profile remain non-verifiable; this is a stylized working model,
not a claim of anatomical realism or user approval. Actual assembled sidebar was
also rendered and inspected; no head/body scaling defect observed in those poses.

La référence et les poses assemblées ont été comparées à échelle identique ;
seuls des déplacements rigides et une réflexion horizontale sont animés.
Les membres cachés restent non vérifiés. Aucun modèle approuvé préexistant
n’a été remplacé pour masquer un écart. Les captures de contrôle sont locales
et ne constituent pas une validation d’intégration dans l’application.
