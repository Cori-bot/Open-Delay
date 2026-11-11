# Open-Delay

Application Windows de délai pour streams. Gratuite, locale, sans compte ni activation. Interface français/anglais, moteur Rust indépendant et fenêtre Tauri avec fichiers intégrés.

**Version 0.2 : H.264/AAC.** Replay Instant, changement de cible, retour live, mode Overlay annulable, routes indépendantes, éditeur d'overlay, dock OBS, raccourcis globaux et plugin Stream Deck. Les variantes Enhanced RTMP/multipiste ne sont pas encore prises en charge ; les essais longs et plateformes réelles restent à qualifier. Aucune supériorité de performances n'est revendiquée.

## Installation et mises à jour

Télécharger l’[installateur Windows x64](https://github.com/Cori-bot/Open-Delay/releases/latest) depuis les releases officielles. Les fichiers `.sig`, `latest.json` et `SHA256SUMS.txt` accompagnent chaque version.

Dans **Paramètres → Mises à jour**, l’application vérifie la dernière version au démarrage et permet de télécharger puis installer à votre demande. Chaque paquet est vérifié avec la clé publique embarquée ; toute sortie connectée, même hors groupe, bloque l’installation. Le moteur s’arrête proprement avant le redémarrage. Les réglages sont conservés.

La signature des paquets updater est distincte du certificat d’éditeur Windows : l’installateur n’a pas encore de signature Authenticode. Le traitement du stream reste local ; la recherche et le téléchargement des mises à jour contactent GitHub.

## Utilisation

1. Ouvrir Open-Delay et suivre la configuration OBS/Streamlabs.
2. Renseigner l'URL du **serveur** de destination. Conserver la clé de stream dans OBS.
3. Copier l'URL RTMP locale affichée dans la sortie OBS correspondante.
4. Démarrer la sortie, préparer l'historique, puis choisir un délai. Le preset **0 s** revient au direct et annule une préparation.
5. Facultatif : ajouter l'URL d'overlay dans une source Navigateur 1920×1080, ou l'URL du dock dans les docks navigateur personnalisés.

Chaque route nécessite sa propre sortie OBS ; Open-Delay ne duplique pas une entrée vers plusieurs destinations. Le mode Instant utilise l'historique préparé. En mode Overlay, le direct continue pendant la construction, puis le contenu antérieur est rejoué. L'overlay est une source visuelle facultative ; il n'est pas ajouté automatiquement à OBS.

## Ports et données

Pas de serveur sur un port fixe 3000. Windows attribue deux ports libres au premier lancement ; Open-Delay les mémorise et les réutilise si disponibles. En cas de collision, il réserve d'autres ports, sans interrompre l'autre application. Les URLs affichées correspondent toujours aux ports effectivement liés ; recopier les URLs OBS si un port change.

La fenêtre native charge ses assets intégrés, jamais la page d'un serveur trouvé sur localhost. L'API, le dock et l'overlay restent loopback. L'option LAN expose uniquement l'entrée RTMP, après activation explicite. Un publisher portant un autre nom ne peut pas remplacer une session active.

Données : `%LOCALAPPDATA%/Open-Delay/`. `config.json` contient réglages et serveurs, sans clé de stream. `runtime.json` contient les adresses courantes pour le plugin, sans jeton d'authentification. Un verrou protège l'instance. L'historique média est temporaire, borné globalement à 2 Gio par défaut, avec cache de 64 Mio ; supprimé à l'arrêt et nettoyé après crash au lancement suivant.

Si un délai ne peut plus être soutenu, la sortie concernée s'arrête avec une erreur explicite. Aucun retour silencieux au direct. Une fermeture de fenêtre minimise au tray par défaut ; l'action Quitter demande confirmation si un stream est actif.

## Compiler

Prérequis : Windows x64, outils de compilation C++ Visual Studio, WebView2, Rust 1.93 ou supérieur, Node.js 24 ou supérieur, npm. Les dépendances résolues sont figées dans les lockfiles.

```powershell
npm --prefix ui ci
npm --prefix ui run build
cargo test
python scripts/notices.py
cargo check -p open-delay-desktop
```

Exécuter uniquement le moteur/dock :

```powershell
cargo run -p open-delay-service -- --data-dir .local/dev --ui-dir ui/dist
```

L'exécutable affiche les adresses réellement réservées. Arrêt propre par Ctrl+C. Pour des données isolées, toujours fournir un autre `--data-dir` ; deux instances ne peuvent pas partager le même dossier.

Application native et installateur :

```powershell
cargo install tauri-cli --version 2.12.3 --locked
cd desktop
cargo tauri dev
cargo tauri build --bundles nsis
```

Les builds de distribution exigent `TAURI_SIGNING_PRIVATE_KEY` et `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Ne jamais les placer dans les sources. Le pipeline publie un installateur et un manifeste updater signés sur les tags stables `vX.Y.Z`, après validation. Voir [docs/RELEASING.md](docs/RELEASING.md). Pour un build local sans artefacts updater, fournir une configuration Tauri de développement qui désactive `bundle.createUpdaterArtifacts`.

## Tests média

FFmpeg n'est pas embarqué dans le produit. Fournir des exécutables FFmpeg/ffprobe de confiance pour le banc synthétique local :

```powershell
python tests/media_e2e.py --ffmpeg C:/outils/ffmpeg/bin/ffmpeg.exe --mode instant
python tests/media_e2e.py --ffmpeg C:/outils/ffmpeg/bin/ffmpeg.exe --mode overlay
```

Le banc crée une vidéo avec compteur et deux chirps audio non périodiques, enregistre la sortie, compare les hashes de payload, les DTS, les points de coupe A/V et le décodage. Aucun accès à l'écran, à OBS ou à une plateforme publique. Résultats dans `.local/media-tests/`. Voir [VALIDATION.md](VALIDATION.md).

## Stream Deck

Plugin original et instructions dans [integrations/streamdeck](integrations/streamdeck/README.md). Quatre actions : Set Delay, Go Live, Toggle, Status. Découverte des ports via `runtime.json`, jeton uniquement en mémoire. Stream Deck 7.1 ou supérieur et runtime Node 24. Le paquet est généré par l'outil officiel Elgato ; aucun plugin tiers requis.

## Architecture et contribution

`crates/rtmp` : handshake, chunks et AMF. `crates/core` : états, commandes, configuration et anneau disque. `crates/service` : transport, ordonnancement et API. `desktop` : shell natif. `ui` : application, dock et renderer overlay.

Voir [CONTRIBUTING.md](CONTRIBUTING.md), [DESIGN.md](DESIGN.md) et [docs/API.md](docs/API.md). Les preuves de recherche privées et ressources propriétaires sont exclues de la distribution.

## Licence

[Open-Delay Source-Available License 1.0](LICENSE) : utilisation, modification, contributions et forks gratuits autorisés ; vente du logiciel ou accès payant à ses fonctions interdits. L'utilisation par un streamer monétisé est autorisée. Cette restriction signifie **source disponible**, et non licence open source OSI. Les dépendances conservent leurs propres licences.
