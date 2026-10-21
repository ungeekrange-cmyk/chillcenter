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

FR : protections demandées activées sans changer la visibilité. L'analyse
GitHub complète les revues locales, elle ne les remplace pas. Les constats sur
fixtures ou chemins bornés sont justifiés individuellement ; aucune alerte
réelle ne doit être masquée pour rendre un tableau vert. Le prochain rapport
consigne le SHA réellement analysé, les résultats et les limites.

References: [GitHub security settings](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/enabling-features-for-your-repository/managing-security-and-analysis-settings-for-your-repository),
[CodeQL default setup](https://docs.github.com/en/code-security/how-tos/find-and-fix-code-vulnerabilities/configure-code-scanning/configure-code-scanning).
