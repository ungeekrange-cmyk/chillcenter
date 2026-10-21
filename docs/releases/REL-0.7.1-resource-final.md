# Contrôle final ressources — Bonjour Mac/serveur v0.7.1

**Audit ciblé en lecture seule, 21 octobre 2026.** Runtime examiné :
`fdb80b502db45d10c5384104912f5f246678c32a`; HEAD documentaire courant :
`3a58d16`. Le diff `fdb80b5..3a58d16 -- apps crates themes` est vide : aucune
conclusion ci-dessous ne transforme les octets runtime, web ou thèmes. Les
modifications déjà présentes sous `docs/design/concepts/` ont été préservées.

## Verdict

**Aucun correctif comportementalement neutre, à forte valeur, n'est justifié
pour v0.7.1.** Le contrôle ne bloque pas le paquet préparé et ne vaut ni preuve
de découverte LAN réelle ni autorisation de publication/déploiement. Le blocage
opérationnel inchangé est `BUG-0119` / ADR-0032 : l'unité Pi actuelle refuse
AF_NETLINK, nécessaire à l'énumération des interfaces lors d'une écoute
wildcard.

## Bonjour / mDNS

| Domaine | Preuve dans le runtime | Conclusion ressources |
| --- | --- | --- |
| Client Mac | `discovery_bridge::browse` prend un sémaphore avec `try_acquire_owned`, refuse le second clic au lieu de mettre une recherche en attente, puis exécute une tâche bloquante isolée de **4 s**. | Au plus une recherche de l'interface à la fois ; elle ne prend pas le verrou des commandes UI/lecteur. |
| Collecte client | `browse` limite le délai à **5 s**, accepte au plus **16** serveurs, déduplique les UUID et plafonne chaque résultat à **8** adresses. Les champs texte sont bornés à 96/64 octets. | La mémoire détenue par le résultat applicatif est bornée par ces constantes ; pas de cache applicatif persistant ni d'écriture disque dans ce chemin. |
| Fin de recherche | `BrowseGuard::drop` appelle `stop_browse` puis `shutdown`, sur succès comme sur sortie anticipée. | Pas de daemon de navigation intentionnellement laissé après la recherche ; trafic/cycles sont bornés par la fenêtre de 4 s côté UI. |
| Annonce serveur | L'annonce ne démarre qu'après le bind TCP ; un seul `Advertiser` est conservé pendant le processus serveur. Son `Drop` désenregistre puis arrête le daemon. Un échec UDP/5353 conserve le HTTPS et la connexion manuelle. | Cycle de vie applicatif explicite, sans retry/boucle applicative observée. L'annonce reste naturellement long-vivante tant que le serveur l'est. |

Le comportement interne de `mdns-sd` (cache de records, cadence des paquets,
threads, RSS/CPU) n'est pas exposé par le code local et aucune mesure sur Mac ou
Pi n'a été exécutée dans cet audit. Il serait incorrect de le déclarer borné ou
de chiffrer son coût sur cette seule inspection. Le test focalisé
`CARGO_TARGET_DIR=/private/tmp/chillcenter-resource-audit-target cargo test --locked --offline -p chill-discovery`
a passé : **5/5** tests, sans test LAN réel.

## Octets distribués observés

| Livrable exact présent | Taille | Constat |
| --- | ---: | --- |
| `artifacts/linux/v0.7.1-fdb80b5/chill-server` | 16 041 056 octets | ELF AArch64 PIE, dynamiquement lié et déjà `stripped`; SHA-256 conforme au dossier de livraison : `8cf144…2491c`. |
| `artifacts/macos/v0.7.1-fdb80b5/Chill Center.app` | 90 084 KiB (`du`), 188 fichiers | L'exécutable fait 26 802 208 octets ; les plus gros éléments restants sont les bibliothèques de lecture (`libavcodec` 9,69 Mo, `libx265` 7,59 Mo, `libshaderc` 7,56 Mo, etc.). Le profil release active déjà Thin LTO, une seule unité de codegen et `strip = true`. |
| DMG v0.7.1 | 47 258 423 octets | SHA-256 conforme : `a752…8bfd`. |

La taille du bundle est exprimée en KiB par `du`; elle ne doit pas être comparée
arithmétiquement aux octets du DMG. Aucun baseline homologue v0.7.1 ni analyse
de liens morts n'a été produite : supprimer ou changer les codecs/librairies
serait un changement fonctionnel et n'est pas recommandé ici.

## Thèmes et aperçu web

Les deux fonds PNG runtime sont les postes image dominants : Noël
**2 174 063** octets et Halloween **2 076 802** octets, soit **4 250 865**
octets. La web app existante embarque les mêmes deux fichiers sous
`branding/events/` et les met tous deux dans `SHELL` du service worker. Ils sont
donc téléchargés/mis en cache au préchargement de l'app-shell, même hors de leur
saison. Une recompression gzip ne réduit pratiquement pas ces PNG (2 091 900 et
1 989 863 octets) : l'emballage HTTP gzip n'est pas une optimisation utile.

Cela représente un coût réel de premier cache/stockage web, mais le web reste
explicitement v0.7.0 inchangé dans ce lot et aucun avant/après n'est inventé.
Retirer ces fonds du pré-cache ou les charger à la demande changerait la
garantie offline et le comportement au premier affichage; réencoder les images
change le rendu. Ce sont donc des options de produit à mesurer et faire valider
dans une version ultérieure, **pas** des recommandations neutres pour v0.7.1.

## Suite autorisée

Conserver les artefacts et le runtime tels quels. Après l'éventuelle autorisation
ADR-0032 et avant de conclure sur le coût réel Bonjour, mesurer séparément sur le
Pi et un Mac : RSS/CPU au repos et pendant une recherche, nombre/octet de paquets
multicast sur 4 s, puis disparition du daemon après recherche. Comparer à une
référence collectée avec les mêmes réseaux/équipements; ne pas déduire ces
mesures des tests unitaires.

