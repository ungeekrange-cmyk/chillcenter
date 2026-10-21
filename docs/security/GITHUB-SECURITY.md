# GitHub security controls / Contrôles GitHub

Date2026-10-21, coordinator/root, repository `ungeekrange-cmyk/chillcenter`.
Explicit user request to enable and inspect GitHub Advanced Security. Existing
PUBLIC visibility and default branch main retained; no paid subscription enabled.

Observed API settings after activation:

- Secret scanning enabled; repository push protection enabled.
- Dependabot vulnerability alerts and automated security fixes enabled.
- CodeQL default setup, extended query suite, weekly schedule plus normal
  repository triggers; Actions, C/C++, JavaScript/TypeScript, Python and Rust.
- Non-provider secret patterns and validity-check settings remained disabled
  despite the request; do not claim those optional features were enabled.

Initial run39578834412 completed successfully on preexisting main121cef8, all
five language analyses with empty error fields. Sixteen alerts were qualified
independently in `REL-0.7.1-security-final.md`. Two source hardenings are included:
fixed component identifiers instead of CLI regex construction, explicit TLS1.2
minimum in disposable fixture. Fourteen other alerts are documented false
positives, not silently suppressed. Any GitHub dismissal needs the explicitly
requested permission and retained per-alert reasoning. New source scan pending.
No open secret or Dependabot alert at the initial check; this is not proof that
all possible credentials or unsupported advisory ecosystems are covered.

## Verified follow-up — 2026-10-21

Run39583548692 on main `b11a480` completed successfully in all five languages.
The two real source findings (alerts1/8) were auto-closed after the fixes.
The user authorized dismissal of fourteen independently qualified false
positives, with a justification retained on each alert. The five later
permission findings17–21 remain **open**, as explicitly requested by the user.
No dismissal authorization is inferred for future alerts.

Dependabot subsequently reported one real medium advisory,
`GHSA-wrw7-89jp-8q8g` on `glib0.18.5`; it remains **open**, not dismissed.
The compatible GTK dependency chain does not yet permit the patched0.20
series. It is absent from both shipped Rust target graphs. See
[dependency controls / suivi FR](DEPENDENCIES.md) and
[English assessment](DEPENDENCIES.en.md) for scope, owner and reassessment.
Weekly Cargo/Actions update proposals and a read-only target-scoped CI guard
are added; no auto-merge, broad advisory ignore list or deployment permission.
The local guard and its negative tests pass; the first hosted run must still
be observed before claiming GitHub execution succeeded.

FR : les cinq alertes de permissions restent ouvertes selon le choix humain.
L'avis glib reste réel et suivi ; les paquets Mac/serveur n'embarquent pas cette
dépendance. Ces contrôles ne prétendent ni corriger glib amont ni couvrir tous
les avis futurs. Aucune clé privée ou adresse du foyer ajoutée au dépôt.

FR : protections demandées activées sans changer la visibilité. L'analyse
GitHub complète les revues locales, elle ne les remplace pas. Les constats sur
fixtures ou chemins bornés sont justifiés individuellement ; aucune alerte
réelle ne doit être masquée pour rendre un tableau vert. Le prochain rapport
consigne le SHA réellement analysé, les résultats et les limites.

References: [GitHub security settings](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/enabling-features-for-your-repository/managing-security-and-analysis-settings-for-your-repository),
[CodeQL default setup](https://docs.github.com/en/code-security/how-tos/find-and-fix-code-vulnerabilities/configure-code-scanning/configure-code-scanning).
