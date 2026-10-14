# ADR-0013 — miroir local du catalogue et séparation des opérations

Date : 2026-10-14. Statut PROPOSED (candidat implémenté, revue indépendante en attente). Auteur/propriétaire `/root`. Branche `codex/catalog-cache-and-client-fixes`, base `ae0ce44`. Supersedes : règle de lecture réseau systématique de CC-DOC-001 §3/10 ; complète ADR0002/0004/0010/0012. Superseded_by NONE. Exigences : demande utilisateur directe du14octobre, BUG0055–0057.

## Contexte et options

Chaque navigation téléchargeait les lots et leurs affiches ; le verrou global bloquait navigation et tick EOF pendant ces I/O. Une saga attendait son image avant ses membres. Un cache JS seul disparaîtrait au redémarrage ; copier la base opérationnelle exposerait des données et chemins inutiles. Décision : miroir SQLite privé de DTO profilés, réutilisant les filtres/groupements du domaine en Rust, pas un deuxième moteur JS.

## Décision

`POST /profiles/{profile}/catalog/snapshot` vérifie une révision opaque serveur/processus/profil+commun/personnel. Inchangé : réponse courte. Sinon instantané immuable, suite par pages de100 liées à un UUID. Maximum10000 médias,32Mio sérialisés par instantané et cumul retenu32Mio,3 sessions, TTL120s avec libération autonome. Transfert actif non évincé ;429 demande d'attendre. Une session terminée peut être remplacée. Chaque page revalide le profil. Positions/préférences/vitesses ne font pas partie du miroir et ne l'invalident pas ; vus/notes/listes n'invalident que leur profil.

La capture serveur compare les epochs avant/après toutes les lectures, y compris les collections : un scan sur une seconde connexion ne peut pas publier d'anciennes lignes sous une nouvelle révision. Conflit :409 `SYNC_SNAPSHOT_CHANGED`, au plus trois tentatives côté Mac, puis conservation de la copie locale. Ce garde optimiste ne prétend pas être une transaction SQLite commune aux connexions serveur.

Le Mac publie données et manifeste en une transaction SQLite sous dossier0700 par serveur/profil. Échec = dernière copie complète conservée et signalée, sans nouvelle tentative à chaque navigation. Grilles/fiches/suggestions/séries/listes lisent localement. Vérification au démarrage, rechargement explicite, retour de focus temporisé et après modifications. Les caches des profils supprimés sont purgés après liste autoritative valide ; liens symboliques refusés, autres serveurs/profils préservés, échec signalé. Écritures en ligne/CAS existant, jamais rejouées hors ligne. Un catalogue modifié entraîne une nouvelle copie complète des métadonnées, pas encore un delta par élément.

Affiches originales/miniatures JPEG persistées :8Mio/source,512Kio/aperçu, plafond2Gio par profil. Préchargement séquentiel hors verrou lecteur ; reprise au démarrage, images présentes non téléchargées. Clés issues d'identifiants et révisions, jamais de chemins serveur. Cache reconstructible sans vidéos ni secrets. Anciennes versions d'images conservées jusqu'au plafond ; rétention plus fine à prévoir si nécessaire.

Images et tick/contrôles lecteur hors verrou de navigation ; autres commandes mises en file. Génération de contexte refusant les commandes périmées après changement de profil/serveur. Dialogues d'image conservent la portée de profil.

Listes : mosaïque des neuf premières vignettes par défaut, image personnalisée avec CAS/routes additives. Remplacement manuel d'affiche et suppression individuelle de liste nettoient uniquement les fichiers générés non référencés du cache contrôlé. Réserve connue : suppression de profil et imports conflictuels/remplaçants peuvent encore laisser des fichiers d'affiche serveur orphelins ; aucun nettoyage global automatique ni accès au NAS n'est engagé. Document d'archive2 seulement avec affiches de listes ; conteneur/manifeste ZIP1 conservé. Importeur accepte documents1/2 ; format1 sans extension reste lisible par ancien importeur.

## Conséquences, compatibilité et retour

API2026-10-14 et aliasv1 conservés, aucune route/ancien champ retiré. Défaut API historique10s conservé ; champ optionnel `initial_preferences` crée atomiquement un profil20s depuis le nouveau Mac. Choix15s ajouté ; anciennes valeurs conservées. Annulation ferme le lecteur ; vignette/bouton lancent la suite, indisponibilité bloque l'automatisme.

Pas de dépendance/coût cloud/auth/transcodage/NAS nouveau. Tables/triggers additifs, marqueur SQLite3 conservé ; sauvegarde privée Pi avant bascule explicitement autorisée. Ancien bundle/binaire préservés. Pour retour intégral données/archives, restaurer la sauvegarde pré-bascule serveur arrêté ; jamais copier une SQLite vivante sans WAL. Ne pas effacer les données/réglages réels du Mac pour tester.

## Validation

Navigation locale sans serveur joignable, persistance/isolation, pagination cohérente, révisions ciblées/profil supprimé, CAS affiches et archive2, DOM et lecteur natif : résultats exacts dans `docs/implementation/CATALOG-CACHE-EVIDENCE.md`. Mesures Pi à réaliser après mise à jour autorisée ; fixtures Mac ne prouvent pas les NFR matériels. Validation visuelle humaine distincte. Revue indépendante ADR PENDING ; décision fonctionnelle demandée directement par l'utilisateur, pas de reprise de découverte imposée.
