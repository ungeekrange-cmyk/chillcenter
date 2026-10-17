# Corrections 82–84 et parité web — 17 octobre 2026

**Français** | [English](WEB-PARITY-20261017.en.md)

## Périmètre

Branche `codex/fix-82-84-web-parity`, base Mac `3b2e28b`, intégration web
`e6e23db`. Les autres worktrees, la release GitHub v0.6, ses artefacts et son
tag restent conservés. Le candidat est numéroté **0.6.1 / build 8**, sans tag
ni publication GitHub. Aucun accès au Pi/NAS ni déploiement dans ce lot.

- BUG0082 : récupération explicite et contextualisée des conflits CAS, brouillon
  conservé ; aucune écriture rejouée sans action utilisateur.
- BUG0083 : loupe SVG proportionnée et centrée.
- BUG0084 : synchronisation SVG à centre fixe, réduction des animations respectée.
- Web : mêmes vues et configuration que le Mac, filtres, collections, séries
  homonymes séparées, outils TMDB/images, profils, langues, identité et archives.
  **Aucune IA web**, selon la réponse explicite de l'utilisateur. Les vidéos en
  relation restent locales et fondées sur les métadonnées.
- Miroir IndexedDB : deltas existants, publication atomique, repli historique
  borné, invalidation après association/image ; pas de nouvelle API serveur.

## Vérification et limites

Commandes exécutées :

```sh
node --test ui/connected/*.test.js ui/connected/tests/*.test.js web/test/*.test.js
node --test scripts/test-bilingual-docs.mjs scripts/test-public-site.mjs
node scripts/test-ui-recovery-icons.cjs
node scripts/build-web.mjs /absolute/path/to/owned/web/dist
node scripts/test-web-parity.cjs
```

Les scripts navigateur utilisent Playwright/Chrome avec API synthétique, pas le
serveur domestique : vrai DOM partagé, pont web construit, IndexedDB, sélecteur
d'image navigateur et persistance de langue. Ils ne valident pas une lecture
NAS, Safari/Firefox, les permissions réelles de téléchargement ou le rendu
humain Tauri. Les tests codecs existants tournent sur fixtures locales ; aucun
nouveau parcours lecteur général n'est imposé à l'utilisateur.

La revue indépendante a repéré un sélecteur d'épisode non représentatif et un
repli snapshot non borné ; corrections et contre-tests ajoutés avant remise.
Un premier parcours web a révélé une invalidation TMDB incomplète, corrigée
avant le passage vert. Les échecs d'outillage Chrome sous sandbox et une fixture
de langue qui écrasait la préférence au reload ne sont pas des bugs serveur.

## Artefacts et retour arrière

Le bilan final de compilation, empreintes et résultats se trouve dans
[la remise du candidat](../releases/REL-WEB-PARITY-001.md).
Le bundle v0.6 publié demeure intact. Le client web doit être servi en HTTPS
depuis le serveur (pas ouvert en `file://`) ; ses capacités codec/plein écran
restent celles du navigateur. Aucun déploiement web/Pi n'est réalisé ici.

Retour arrière : utiliser le bundle v0.6 conservé et l'ancienne distribution web.
Pas de migration de schéma serveur, de suppression de cache, de scan ni de
réécriture des vidéos. Voir [ADR0025](../architecture/decisions/ADR-0025-web-shared-ui-parity.md).
