# Validation Open-Delay 0.1

8 novembre 2026. Résultats locaux Windows x64 ; aucune plateforme publique, clé réelle ou capture de l'écran utilisateur. Les fichiers de recherche propriétaires restent hors distribution.

## Vérifications automatisées

- Rust : **41 tests réussis** — stockage global et rétention, contrôleur, générations, annulation et déduplication, protocole RTMP, publication upstream, admission LAN, ports automatiques, authentification, quotas, sauvegarde et arrêt SSE.
- Frontend : **12 tests réussis**, build TypeScript/Vite et lint React hooks réussis. Validation runtime des réponses HTTP/SSE ; timeout de l'overlay et masquage en récupération/dégradation.
- Stream Deck : **8 tests réussis**, validation et empaquetage par la CLI officielle Elgato. Aucun test matériel revendiqué.
- `cargo check -p open-delay-desktop` et Clippy sur les cibles moteur réussis. CI Windows fournie ; son exécution distante n'est pas revendiquée.
- Audit npm du frontend : zéro vulnérabilité signalée après correction de la dépendance de lint. Les composants tiers sont inventoriés dans le SBOM généré avec leurs licences/notices.

## Média réel loopback

Source : H.264/AAC 640×360 30 fps, GOP1s, B-frames0, compteur d'images et audio chirp stéréo non périodique. Publication FFmpeg standard avec connect dépassant le chunk initial ; récepteur FFmpeg strict. Aucun transcodage dans Open-Delay.

| Test | Paquets vidéo/audio uniques retrouvés | DTS régressifs | Décodage | Écart maximal des coupes A/V |
|---|---:|---:|---|---:|
| Instant, préparation3s → cible5s → direct | 796 / 1241 | 0 | Sans erreur | 16 ms |
| Overlay, annulation pendant construction → délai3s → direct | 796 / 1238 | 0 | Sans erreur | 19 ms |

Tous les payloads des deux pistes sont retrouvés dans leur source, sans correspondance ambiguë. L'écart de mapping A/V sur les intervalles stables est nul à l'arrondi près. Les coupes sont évaluées séparément des intervalles stables, pour ne pas comparer artificiellement les deux côtés d'un saut temporel.

Preuves privées : `.local/media-tests/instant-20261108-205705/` et `overlay-20261108-205704/` ; scripts reproductibles dans `tests/media_e2e.py`.

## Deux routes et reconnexion

`tests/multiroute_e2e.py` réussit sur deux vidéos synthétiques distinctes : délai groupé2s, retour live, coupure du receveur A pendant3s, progression de B pendant la panne, reconnexion A avec même génération et délai conservé, puis arrêt du publisher A et retour live de B seule.

Capture B :2263 paquets,884 vidéo. Capture A après reconnexion :222 paquets,86 vidéo. Tous les payloads vidéo correspondent à la bonne route, aucun mélange ; zéro régression DTS et zéro erreur de décodage sur ces captures. La progression d'A est observée pendant2s après reconnexion avec délai effectif≥1,8s. L'isolation des payloads audio entre routes n'est pas établie par ce test.

Preuves : `.local/multiroute-tests/20261108-210737/`. Tous les processus possédés sont terminés ; fichiers de découverte et anneaux supprimés.

## Portée actuelle

Ces essais courts valident le moteur de base et plusieurs scénarios de panne ; ils ne démontrent pas toute la matrice prévue par le plan.

Restent à qualifier : Enhanced RTMP/multipiste (actuellement refusés explicitement), codecs autres que H.264/AAC, vidéo verticale, variations GOP/B-frames approfondies, RTMPS contre serveurs de production, pannes prolongées, composition OBS réelle de l'overlay, raccourcis/tray en usage interactif et matériel Stream Deck.

Les essais8h, délais60/600s à plusieurs débits et comparaison CPU/RAM/I/O/latence face à InstantDelay ne sont pas réalisés. Aucune affirmation de parité intégrale, de synchronisation universelle ni de performances supérieures. Les installateurs locaux sont non signés ; updater désactivé.

## Complément — 9 novembre 2026

- 41 tests Rust moteur réussis ; tests média de nouveau exécutés avec FFmpeg en loopback, sans comptes ni capture d’écran : Instant et Overlay décodés sans erreur, aucun payload manquant ni DTS régressif ; écarts maximaux aux coupes audio/vidéo de 16 et 19 ms respectivement.
- Test de deux routes et de reconnexion relancé avec succès : isolation vidéo, continuité de la seconde route et retour au direct vérifiés. L’isolation des payloads audio entre routes n’est pas établie par ce scénario.
- 14 tests UI réussis, dont deux tests ciblés sur les modifications concurrentes lors de la sauvegarde des ports. Les menus partagés ont aussi été exercés dans Chrome sans interface visible, à 640 et 1100 px : souris, clavier, état désactivé et menu dans un dialogue (`npm --prefix ui run test:select`, Chrome requis ou `CHROMIUM_PATH`). Aucun test de lecteur d’écran ni d’apparence complète des sept pages n’est revendiqué.
- Ces résultats ne lèvent pas les limites listées ci-dessus : tests courts en local uniquement, aucune garantie de fonctionnement à 100 %.

## Refonte monochrome — 9 novembre 2026

- Les sept écrans ont été exercés en français et en anglais dans Chrome isolé à 320, 640 et 1100 px, sans débordement horizontal détecté ; modes de mouvement réduit, état de panne et ordre de deux commandes ont été exercés avec un moteur simulé. Ces tests ne constituent pas une vérification au lecteur d’écran ni un audit complet WCAG.
- 14 tests UI, lint et build passent. Les essais média réels Instant et Overlay ont été relancés en loopback : aucun payload non reconnu, aucune régression DTS et aucune erreur de décodage ; écarts A/V maximaux aux coupes respectivement 16 et 19 ms.
- Les commandes UI sont envoyées dans l’ordre des clics. Une requête précédente peut retarder Go Live jusqu’à son expiration (10 s), et la garantie stricte d’ordre après expiration demande une séquence vérifiée côté serveur. Ne pas présenter cette interface comme un arrêt d’urgence ou une garantie de retour immédiat.
- Les essais média sont courts et locaux ; les plateformes réelles, lecteurs d’écran, Stream Deck physique et tests longue durée restent à qualifier.


## UX — 11 novembre 2026

Lint, build TypeScript/Vite et 14 tests unitaires réussis. Vitest utilise TEMP/TMP dans `.tmp-vitest` pour éviter une erreur EPERM du répertoire temporaire du sandbox. Tests navigateur dans Chrome isolé sur localhost, 7 pages × 2 langues × 3 largeurs (320/640/1100), sans captures : absence de débordement, erreurs JS, sélection des actions Stream Deck au clavier, commandes ordonnées, récupération, parcours de configuration, étape overlay passable et focus au retour d’étape. Les appels de capture préexistants ont été retirés. Revues React/TypeScript réalisées ; périmètre des sorties hors groupe et focus corrigés. Aucun test de matériel réel ni étude utilisateur. Aucun nouveau test média, car les commandes et le moteur ne changent pas.


## Overlay piloté et présentation — 11 novembre 2026

- UI : lint, build TypeScript/Vite, 17 tests unitaires. Service : 9 tests lib et 5 intégration, y compris auth, séparation média/visuel, validation UTF-16, persistance (échec conservant l’état), arrêt/expiration.
- Navigateur isolé sans captures : 7 pages × FR/EN × 320/640/1100 ; touches Stream Deck carrées égales. `npm --prefix ui run test:overlay` vérifie config appliquée, heartbeat, test/source indépendante, arrêt, expiration, isolation des anciennes URLs, lecture obsolète, erreur de sauvegarde persistante, présentation et parcours initial. Présentation revalidée sur trois largeurs avec/sans réduction des mouvements.
- `npm --prefix ui run test:overlay:live` après build UI et service : vrai binaire Rust avec données temporaires et ports libres, authentification locale réelle, configuration enregistrée sur disque, badge reçu dans un document navigateur indépendant, arrêt et expiration. Aucun appel aux commandes média, aucune utilisation d’OBS utilisateur.
- Revues React/TypeScript/Rust réalisées ; corrections des courses polling/mutations et chargement, séparation erreur connexion/action, logique d’état partagée, IO overlay hors workers Tokio et validation texte cohérente. L’app dev Tauri se recharge sur les sources modifiées.
- Limite : une source Navigateur isolée valide le mécanisme de livraison ; la scène OBS réelle reste à tester. Pas de revendication de connexion OBS WebSocket ni de contrôle des scènes. Aucun écran utilisateur capturé.


## Distribution 0.2.0 et updater — 11 novembre 2026

- 19 tests UI unitaires ; 49 tests Rust workspace, dont 3 tests updater et un test de refus d’admission média après arrêt. Lint, Clippy workspace strict et formatage réussis. 6 tests de version/manifeste/archive réussis.
- Navigateur isolé : contrôle natif simulé avec recherche au démarrage, sorties hors groupe et état moteur inconnu bloquant l’installation, confirmation, erreurs de signature, notes rendues en texte, événements natifs et préservation du chemin RTMP personnalisé. Test de régression de réponse native obsolète pendant le téléchargement réussi.
- Bridge updater natif : manifeste/paquet HTTPS, candidat conservé côté Rust, signature obligatoire, contrôle des sorties avant/après téléchargement et arrêt propre avant lancement NSIS ; récupération moteur en cas d’échec de l’installateur. Revues Rust, React, TypeScript et sécurité réalisées.
- Installateur NSIS 0.2.0 et signature updater construits localement. Clé privée et mot de passe sous `.local/signing/`, exclus des sources. Clé publique embarquée. Aucune signature Authenticode Windows : pas de certificat d’éditeur.
- Publication sur Cori-bot/Open-Delay autorisée par l’utilisateur ; pipeline de tags stable préparé pour créer un draft, téléverser tous les artefacts et publier seulement l’ensemble complet. Une véritable installation/migration entre deux versions reste à qualifier ; elle n’a pas été exécutée sur la machine utilisateur.
