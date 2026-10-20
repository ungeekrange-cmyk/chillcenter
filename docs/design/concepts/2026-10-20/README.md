# Easter / Valentine proposals — Propositions Pâques / Saint-Valentin

Status / Statut: **DRAFT — user visual approval pending / validation visuelle attendue**.
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
text or logos. Easter: close foreground wicker egg basket and tulips to the right,
warm daylight. Valentine: close deep red roses, candles and velvet cushion to the
right, warm evening. The original generated files are preserved outside the repo.

Calendar proof: `node --test docs/design/concepts/2026-10-20/calendar.test.cjs`.
UTC civil-date arithmetic avoids DST shifts. This is a design-preview calculator,
not an accepted runtime theme implementation.
