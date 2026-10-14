# Preuve — essai en lecture seule du client web sur le Pi

Date : 14 octobre 2026 (≈21 h 45 locale). Branche `codex/web-pwa`. Cible : Pi `192.168.1.42:8443`, serveur **inchangé** (aucun déploiement, aucun scan, aucune écriture persistante). Demande explicite de l'utilisateur ; une opération parallèle de l'utilisateur sur le serveur a été signalée pendant l'essai (aucune interruption observée).

## Ce qui a été fait

| Étape | Résultat |
| --- | --- |
| Certificat récupéré par TLS, validé ensuite avec `--cacert` / `ca` (jamais `-k`) | SPKI SHA-256 `3018fb4d…40df`, identique à l'empreinte déjà consignée pour ce Pi ; CN `Chill Center`, valide du 14/10/2026 au 14/04/2028 |
| `GET /api/2026-10-14/server` | `api_version` 2026-10-14, versions `2026-10-14` et `v1`, capacités `catalog_pages`, `playback_leases`, `media_range` présentes |
| `GET /profiles`, catalogue paginé (100 par page) | 1 profil ; **2 496 titres disponibles** |
| Lectures `Range` par les démuxeurs du client web (code `web/src/`) en Node sur 21 fichiers réels répartis dans le catalogue | **21 ouverts sur 21**, aucun échec |
| Fragments réels : 1 MKV et 1 MP4, six fragments par piste, puis saut à 50 % | MKV : 1,8 Mio en 1,7 s (≈9 Mb/s), saut en 276 ms ; MP4 : 4,2 Mio en 4,0 s (≈9 Mb/s), saut en 579 ms |

Limite de la mesure de débit : un seul client (ce Mac), requêtes séquentielles, début de fichier, réseau du moment ; ce n'est ni un test de lecture continue ni une preuve de fluidité Wi-Fi à plusieurs flux.

**Écriture déclarée** : lire des octets exige un bail de lecture. 23 baux volatils (en mémoire) ont été acquis puis libérés immédiatement sur le profil existant, sans aucune progression ni modification de reprise/vu/note. Arrêt prévu à la première réponse 409 ; aucune n'est survenue.

## Inventaire (catalogue entier, par extension du chemin ; aucun titre relevé)

| Conteneur | Fichiers | Part |
| --- | ---: | ---: |
| MKV | 1 944 | 78 % |
| AVI | 369 | 15 % |
| MP4 | 183 | 7 % |

## Codecs sur l'échantillon de 21 fichiers (pas un recensement complet)

- Vidéo H.264 : majoritaire ; HEVC : 5 MKV sur 12 et 1 MP4 sur 4 (profil Main/Main10) ; **AVI : 5 sur 5 en Xvid (MPEG-4 ASP)**.
- Audio : AAC dans 11 fichiers MKV/MP4 ; **AC-3 dans 6 MKV sur 12, E-AC-3 dans 1** ; sous-titres MP4 `tx3g` ; un MKV avec VOBSUB.

## Conséquences pour la décision (ADR-0013)

1. MKV d'abord était justifié : 78 % du catalogue.
2. **Xvid/MPEG-4 ASP n'est décodé par aucun navigateur** : l'AVI du catalogue (15 %) ne pourra pas être lu par le client web sans décodeur logiciel en WASM, ce qui contredit le critère « léger et stable ». Décision utilisateur requise (message explicite et renvoi vers le client Mac, ou ADR dédié pour un décodeur).
3. **AC-3/E-AC-3 est fréquent** (≈ la moitié des MKV de l'échantillon) : la lecture dépend du navigateur (Safari oui ; Chrome selon plateforme). Sans décodeur, ces fichiers seront muets ou refusés ; mesure sur les navigateurs cibles nécessaire avant toute annonce de parité.
4. HEVC dépend du matériel/navigateur ; à tester sur les clients réels.
5. Lecture réelle dans un navigateur depuis le Pi : **non faite** — le serveur déployé refuse encore les requêtes de navigateur (ancienne garde d'origine) et ne sert pas la PWA ; il faut le nouveau binaire (non compilé ici, `cargo` absent) et une autorisation de déploiement explicite.

## Complément — relevé des en-têtes des 369 AVI (même jour, même cadre lecture seule)

Un bail volatil par fichier, acquis puis libéré aussitôt (aucune progression, aucun titre relevé) : **deux passes de 369 fichiers plus un essai sur 1 fichier, soit 739 acquisitions/libérations au total**. Une première passe a échoué sur les 369 fichiers à cause d'un défaut de **mon analyseur** (entiers RIFF lus en gros-boutiste) ; elle n'a rien modifié côté serveur, a été diagnostiquée sur un seul fichier, corrigée, puis relancée. Résultat de la passe corrigée : 368 lus, 1 illisible.

Xvid 354, DX50 13, H.264 1 ; largeur ≤ ≈ 720 px pour 366 fichiers ; audio MP3 stéréo 364, AC-3 4 ; une seule piste audio. Détail et conséquences dans [ADR-0014](../architecture/decisions/ADR-0014-web-avi-xvid-decoder-evaluation.md).

Écritures déclarées : uniquement les baux volatils (mémoire du serveur). Aucune interruption ni erreur 409 pendant le relevé.
