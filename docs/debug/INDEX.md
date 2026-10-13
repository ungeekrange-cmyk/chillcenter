# Journal des bugs, observations et risques

Révision `CC-DOC-001`. Ce journal sera relu avant chaque nouvelle intervention. Les risques ne sont pas des bugs démontrés et les incidents historiques ne sont pas de nouveaux défauts Chill Center.

| ID | Type | État | Preuve / effet |
| --- | --- | --- | --- |
| HIST-001 | Incident Cinéfolio historique | ROOT_CAUSE_CONFIRMED, historique | Scan NAS timeout ; accès répertoire bloqué indépendamment du scanner. Source : `Video catalog/docs/debug/BUG-NAS-ACCESS.md`, 2026-10-12, code `1d842015d78e8d3cb7b2503d66309f422cc8fdd8`. Aucune reproduction Chill Center. |
| OBS-001 | Contexte/outillage | RESOLVED | Ancien cwd `Mongoose-Posture/Video catalog` inexistant ; source déplacée dans `Perso/Video catalog`, confirmée par l'utilisateur. Utiliser chemins actuels. |
| OBS-002 | Accès dépôt | RESOLVED_WITH_CLI | Connecteur lié : 404 ; CLI d'abord bloqué réseau du sandbox puis `gh repo view` autorisé : succès, dépôt vide. Ne pas conclure que le dépôt n'existe pas. |
| OBS-003 | Source sans checkout | BASELINED | Dossier source sans `.git`, mais bundle valide, historique complet 0.5.0. Ne pas initialiser/écraser la source ; utiliser copie isolée à l'implémentation et comparer différences de travail. |
| OBS-004 | Nom de destination | RESOLVED | Utilisateur corrige `ChilleCenter` en `ChillCenter`. |
| SEC-001 | Risque accepté utilisateur | ACCEPTED_FUNCTIONAL_RISK | Nouveau certificat accepté automatiquement après expiration peut venir d'un imposteur LAN. F-19 / ADR-0005 ; vérification cryptographique normale conservée dans les autres cas. |
| RISK-001 | Performance | TO_VERIFY | Pi probablement en Wi-Fi, deux passages réseau par flux ; trois vidéos non garanties avant V-02. |
| RISK-002 | Faisabilité lecteur | TO_VERIFY | Rendu libmpv Mac et adaptateur de flux/TLS à prouver dans l'enabler ; V-03/V-09, remplacement par ADR si nécessaire. |
| RISK-003 | Identité fichier | CONTROLLED_BY_DESIGN | Empreinte partielle insuffisante pour preuve absolue ; confirmation ou preuve complète avant réassociation. F-16 / ADR-0002. |
| DESIGN-001 | Lacunes d'architecture, revue indépendante | CORRECTED_PENDING_REREVIEW | ARCH-F-001/002 : bail lié média/révision, génération suivante et deadline de transfert non renouvelable ajoutés. Pas un bug applicatif exécuté. |
| DESIGN-002 | Contraintes bibliothèque, revue indépendante | CORRECTED_PENDING_REREVIEW | ARCH-F-003 : version/ABI, lifetime et annulation stream_cb contractualisés ; enabler encore NOT_RUN. |
| DESIGN-003 | Lacune TLS, revue indépendante | CORRECTED_PENDING_REREVIEW | ARCH-F-004 : déclaration de rotation nécessitant HTTP remplacée par pin SPKI stable vérifiable au handshake ; certificat renouvelé, clé protégée long terme. |

## Format de tout futur BUG

Suivi r5 de BUG-0010 : base vectorisée validée par l'utilisateur,13 PNG réexportés depuis les SVG communs ; les12 anciens rasters canoniques sont archivés dans `icons/branding/raster-r4/`. Les comparaisons raster r3/r4 restent historiques, pas les assets courants. Sources SVG/générateur inchangés, aucun nouveau défaut applicatif revendiqué.

Assets de marque r3/r4 : [BUG-0010 géométrie du canapé](BUG-0010-branding-geometry.md) : VERIFIED pour les SVG communs autorisés par l'utilisateur, mêmes tracés sur13 variantes ; défaut principal Noël clair corrigé, branches latérales vérifiées. Audit des PNG et leurs écarts résiduels conservés dans `icons/branding/geometry-audit-r3.json`. Méthode vectorielle autorisée, propositions visuelles encore à valider.

Assets de marque, 2026-10-13 : [BUG-0009 marques fantômes](BUG-0009-branding-artifacts.md) : VERIFIED par inspection directe des PNG avant/après pour été clair et automne sombre. Pas un défaut de la galerie ou du runtime ; aucun test applicatif revendiqué pour ces retouches. Le logo approuvé et les six variantes non concernées sont inchangés.

Implémentation 2026-10-13 : [BUG-0001 fournisseur TLS](BUG-0001-tls-provider.md) et [BUG-0002 autorité HTTP/2](BUG-0002-http2-authority.md) : corrections vérifiées par tests et smoke réel de configuration ; [BUG-0003 fixture extraite](BUG-0003-extracted-fixture.md) : VERIFIED par 64 tests catalogue. Les défauts de revue du candidat domaine initial ne sont pas effacés ; nouvelles preuves dans `docs/implementation/` et rapports à consulter avant intégration finale.

Lot interface, source `c2a9d72` : [BUG-0004 type TLS reqwest](BUG-0004-reqwest-tls-type.md), [BUG-0005 rollback profil](BUG-0005-profile-context.md), [BUG-0006 mutations tardives](BUG-0006-late-scan-mutations.md), [BUG-0007 snapshot/identités](BUG-0007-scan-identity-atomicity.md), [BUG-0008 lecture SQLite](BUG-0008-api-read-errors.md) : VERIFIED par régressions, 124 tests et revues indépendantes r3. Preuves exactes dans INTERFACE-EVIDENCE ; rapports précédents conservés. Remplacement exceptionnel de clé affiché dans un bandeau persistant, erreurs API sûres/typées ; confiance provisoire annulée après connexion initiale échouée.

LOW ouverts, propriétaire /root : UISEC-002 (fsync du parent des réglages client) et SRV-SEC-005 (réconciliation après fsync serveur en échec). Non bloquants pour ce lot de développement selon revue sécurité r3, mais à traiter avant remise globale ; pas présentés comme corrigés. Pas de nouvelle acceptation de risque.

Créer `BUG-NNNN-titre.md` : date, environnement/matériel/réseau, branche/commit exacts, attendu/réel, reproduction, hypothèses numérotées avec une seule expérience falsifiable à la fois, résultat, cause confirmée ou inconnue, correction et commit, tests de régression, rollback, exigences/ADR concernés. Limite : cinq hypothèses non confirmées puis retour à l'utilisateur. Ne pas appliquer une correction spéculative à une dépendance externe indisponible.

Statuts : OPEN, INVESTIGATING, ROOT_CAUSE_CONFIRMED, FIXED, VERIFIED, CLOSED. « Non reproduit » n'est pas « corrigé ». Les preuves originales ne sont pas effacées ; une correction ajoute un lien. Ajouter ici le résumé consultable, sans enregistrer secrets ou contenu des profils.
