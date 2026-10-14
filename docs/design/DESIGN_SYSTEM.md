# Design System & UI Specification — CC-UX-002

## 1. Overview & Brand Archetype

Chill Center, auteur `/root`, 2026-10-14, statut **REVIEW_REQUESTED**. Reprendre l'interface Cinéfolio (`90c149b`) et le logo vectoriel approuvé. Demande : navigation latérale, recherche haute, administration basse, actions de gestion uniquement dans Administration, conseil IA distinct en sidebar, catalogue sans pages, grandes fiches, TMDB configurable/automatique et lecteur standard.

[Maquette interactive](prototype.html) sans réseau : affiches illustratives CSS, données fictives, aucune vidéo ou requête IA/TMDB. Elle ne modifie aucun serveur ni credential.

## 2. Design Tokens

### 2.1 Color Palette & Theme

| Token | Clair | Sombre | Contraste mesuré | Usage |
| --- | --- | --- | --- | --- |
| Canvas / texte | `#f5f4ee` / `#20261f` | `#111713` / `#e9e8df` | 14,03 / 14,77 | Fond et texte principal |
| Surface / secondaire | `#ffffff` / `#57634f` | `#1a211c` / `#a7aaa0` | 6,36 / 6,96 | Panneaux / métadonnées |
| Accent / texte | `#71602f` / `#ffffff` | `#d8c995` / `#202017` | 6,15 / 9,93 | Boutons principaux |
| Sidebar | `#eeefe7` | `#151c17` | Non audité globalement | Navigation |
| Bordure | `#cdd5c7` | `#344039` | Non textuel à auditer | Séparateurs |

Calcul sRGB WCAG, pas une certification AA de tous les états. Automatique/clair/sombre et ambiances saisonnières/événementielles existants conservés ; seul clair/sombre simulé ici.

### 2.2 Typography

Police système `-apple-system, BlinkMacSystemFont, Segoe UI, sans-serif`, sans téléchargement. Titres 30–34 px/600–650 ; corps 14 px ; métadonnées 12 px ; légendes 11 px. Temps tabulaires. Georgia n'est utilisée que dans les illustrations fictives.

### 2.3 Spacing & Radius

Logo 96×96 px sur bureau, 48×48 px sur vue étroite. Aucun nom ou slogan à côté ; le lettrage intégré au SVG approuvé reste intact. Base 4/8/12/16/24/32/48 px. Boutons 8 px, cartes 12 px, dialog 17 px. Sidebar 235 px, gouttières adaptatives. Transitions 150–200 ms, désactivées en reduced-motion.

## 3. Core Component Library

| Composant | Comportement et états | Accessibilité / garde-fous |
| --- | --- | --- |
| Sidebar | Logo seul agrandi, catégories, sagas, listes ; Demande de conseil et Administration en bas | Icônes nommées, sélection visible |
| Recherche | En haut ; profil à droite ; filtres/tri conservés | Labels, Cmd/Ctrl-F, clavier |
| Catalogue | Continu, aucun bouton de pagination | Grille virtualisée finale, affiches visibles chargées à la demande ; lots API internes autorisés, pas de plafond caché |
| Organisation IA | Petit bouton uniquement dans Administration, onglet Organisation IA | CLI Mac existant, périmètre, aperçu/confirmation/annulation ; jamais vidéos ni secrets |
| Demande de conseil | Entrée sidebar, envies libres, filtres Tous/Vus/Non vus et Films et séries/Films/Séries, suggestions de la bibliothèque | Filtres combinés et propres à cette demande ; états vus du profil actif ; lecture seule et ouverture des fiches ; zéro mutation, pas de bouton Appliquer |
| Administration | Recharger, compléter les fiches, organiser avec l'IA ; Serveur/Bibliothèque/TMDB/Organisation IA/Profils/Ce Mac | Trois actions retirées du catalogue ; recharger ne lance pas de scan ; scan/import ici ; premier démarrage/consentement TLS conservés |
| TMDB | Clé v3 OU jeton Bearer, champ masqué, statut et effacement | Stockage serveur seul, zéro secret en réponse/logs/export/IA/persistance client |
| Fiche | Jusqu'à 1180px/94vw, affiche ample et informations/actions | Dialog clavier/Escape, scroll si besoin, restitution focus |
| Série / saison / épisode | Compteurs saisons, épisodes et épisodes non vus sur carte et fiche ; ouverture des saisons puis épisodes, retour à la série | Compteurs du profil actif, épisodes indexés, navigation clavier ; épisode indisponible conservé mais lecture désactivée |
| Sagas / listes | Vues dédiées avec Créer et Ajouter des vidéos, sélection multiple et retour à l'ensemble des collections | Sagas de films communes au foyer, listes personnelles au profil ; sans doublon ; créations mémoire uniquement dans la maquette |
| Disponibilité | Aucun badge Disponible ; badge Indisponible seulement si nécessaire | Ne pas masquer l'entrée ou supprimer son historique ; Lire désactivé quand indisponible |
| Lecteur | Surface plein écran, barre basse superposée, icônes, −10/+30, seek, son, langues, vitesses | Masquage après 2,5s seulement en lecture ; toujours visible en pause/buffering/focus/menu ; retour pointeur/clavier |

Automatisme TMDB proposé activé après saisie volontaire de clé : exact-match seulement, ambiguïtés à confirmer, pas d'écrasement manuel, traitement borné/reprenable, réessais limités et priorité aux flux vidéo. Scan non bloqué par TMDB. Contrat secret explicite et revue sécurité avant intégration. Une clé v3 n'est pas un jeton Bearer.

« Lire » démarre dès les prérequis réseau/bail valides, sans attendre inutilement les sous-titres externes ; garder TLS, heartbeat et transfert confirmé. Plein écran respecte le ratio de vidéo (bandes normales possibles), sans étirement imposé. Moteur libmpv/FFmpeg conservé, serveur sans transcodage, client sans NAS.

## 4. Accessibility & Responsiveness Checklist

- [x] Contrastes des six paires textuelles calculés, pas d'audit AA complet revendiqué.
- [x] Prototype contrôlé à 1440/1180/760/390px sans overflow horizontal.
- [x] Labels/focus/dialog natif prévus ; simulation auto-hide/pause/fullscreen vérifiée.
- [ ] Audit clavier complet/lecteur d'écran dans Tauri et états erreur/chargement.
- [ ] Overlay réellement cliquable au-dessus de NSOpenGLView/AppKit, pas une simple capture DOM.
- [ ] Frame vidéo vs écran réel, Escape/restauration ; sous-titres lents et baux.
- [ ] Catalogue 2496/3000 et ressources ; profils/reprise/notes/IA/cache/ambiances/import sans régression.

## 5. Mandatory User Validation Gate

Gate [G-UI-PROJECT-CC-UX-002](../gates/G-UI-PROJECT-CC-UX-002.md), **PENDING**. Approbateur/date UTC/preuve : non reçus. Retour humain CC-UX-001 : « Oui bien, mais » puis déplacement des trois actions dans Administration, remplacement sidebar par Demande de conseil ; second message retire slogan et nom à côté du logo et demande son agrandissement. Ces demandes sont intégrées ici. L'accord de principe est consigné, pas transformé en approbation de la révision CC-UX-002 non encore présentée. Scope : séparation gestion/conseil, branding et navigation révisés, thème/typo conservés, fiche et contrôles. Changement substantiel de palette/navigation/composants invalide la validation ; correction technique conforme ne relance pas la discovery.

Après validation : builder distinct des diagnostics, contrat TMDB/revue sécurité, tests ciblés et vrais essais Tauri, bundle Mac puis serveur seulement si modifié. Aucun code applicatif, bundle ou service changé à cette révision.

## Changements r2 et portée

Les demandes utilisateur priment sur le placement IA proposé en r1. [Spec r1 conservée](DESIGN_SYSTEM-CC-UX-001.md), commit historique `386dbc4`. Lane de cette livraison : DEV_FAST pour la maquette/documentation uniquement ; pas de nouvelle frontière secret/IA exécutée.

Le conseil réutilisera le mode recherche en lecture seule de l'IA existante, sans endpoint d'application ni modification de saga, fiche, note, vu ou playlist. Les suggestions concernent la bibliothèque ; ne pas inventer un titre disponible. Changement de profil/serveur invalide une réponse en cours. Les données nécessaires du seul profil actif sont envoyées sur action explicite, avec information sur le fournisseur externe. La maquette montre un exemple fixe clairement identifié, pas une vraie inférence.

Ajout humain du14octobre : « dans la demande de conseil, on peut faire des filtres aussi (vu/non vu), films séries ». Défauts : Tous et Films et séries. Les deux filtres se combinent et bornent les candidats **avant** l'appel IA, puis les suggestions sont revalidées contre ce périmètre ; pas seulement une instruction au modèle. Ils n'altèrent pas les filtres du catalogue. Changer un filtre retire les suggestions périmées ; pas d'appel IA automatique, relancer explicitement Me conseiller. Aucun résultat : message clair, sans élargir les critères silencieusement. La maquette filtre seulement ses exemples fixes. Checks Playwright PASS pour films vus, films non vus, séries non vues, aucune série vue, invalidation et vue390px ; vrai conseil IA NOT_RUN.

Ajout humain suivant : compteurs de séries, saisons et épisodes navigables, création de sagas/listes avec vidéos, absence de badge Disponible. Parcours interactifs de maquette couvrent [US-008](../stories/US-008/story.md), [US-011](../stories/US-011/story.md), [US-012](../stories/US-012/story.md), [US-014](../stories/US-014/story.md) sans certifier leur intégration réelle. Exemple fictif The Expanse : 2 saisons/5 épisodes/3 non vus, dont un indisponible, clairement signalé comme jeu de démonstration et non description réelle de la série. Les compteurs concernent les épisodes présents au catalogue, pas un total TMDB théorique. Création vide ou avec sélection, ajout sans doublon, annulation ; saga commune et liste personnelle. Ni ordre backend, ni API, ni NAS modifiés ; pas de persistence après rechargement de maquette. Reprise série ne saute pas les règles d'enchaînement/baux dans l'application finale.

Aucune modification des ADR acceptés, pas de nouveau fournisseur/modèle/runtime ni de coût déclenché. Gouvernance r1 remplacée pour le placement/branding ; preuves bundle/Pi historiques inchangées et non utilisées comme validation de cette UI.

## Références et vérification

- [QuickTime Apple](https://support.apple.com/en-euro/guide/quicktime-player/qtp6cee0761b/mac) pour commandes au mouvement du pointeur ; pas de décision de remplacer libmpv par AVPlayer.
- [Authentification TMDB](https://developer.themoviedb.org/docs/authentication-application) pour clé v3/Bearer.
- `check-prototype.cjs` PASS CC-UX-002 le 2026-10-14, Chrome headless isolé avec HTTP(S) bloqué : logo seul agrandi, gestion visible uniquement en Administration, conseil sans action de mutation, ouverture des fiches depuis suggestions fictives, fiche large, lecture simulée/pause/auto-hide/fullscreen, champ factice TMDB effacé/non persisté, responsive/clair-sombre et zéro erreur JS. Captures inspectées par l'auteur, pas une revue indépendante.
- Vidéo native, vrai TMDB et vrai IA : **NOT_RUN pour cette refonte**. [Diagnostic](../debug/BUG-0045-client-ui-port.md).
