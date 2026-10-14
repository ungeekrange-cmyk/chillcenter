# BUG-0039 — Langues natives limitées à anglais/français

14 octobre2026, extension du test natif US-019. RED réel Tauri : préférenceja, pistesen/fr, aucun avis de préférence indisponible. Cause confirmée : language_key native renvoyaitNone pour tout autre code queen/fr, contrairement au comparateur externe. Une préférence ignorée pouvait aussi correspondre à une piste sans langue viaNone==None.

Correction ciblée : même clé dans native etbridge, codes2/3lettres valides distincts, alias usuels et région/casse normalisés ; vide/invalideNone signifie sous-titres désactivés. Une langue valide absente suit le repli original/première piste et l'avis, jamais un choix arbitraire de sous-titre. Tests unitaires alias/en/fr/de/ja/pt/vide/invalide PASS ; nouveau parcours natif jaabsent/frambigu2externes GREEN PASS (`language-fallback-ambiguous-subtitles`). Pas de garantie sur des tags linguistiques arbitraires ou de validation auditive humaine.
