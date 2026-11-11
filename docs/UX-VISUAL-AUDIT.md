# Audit visuel — 11 novembre 2026

Audit réalisé dans Chrome **visible**, piloté avec Playwright CLI. Les pages et les formulaires utilisent un moteur réel avec des données isolées. Les états de diffusion et le dialogue de mise à jour utilisent des fixtures explicitement simulées ; aucune installation et aucun stream externe n'ont été lancés.

## Corrections issues du test

| Problème reproduit | Cause | Correction et régression |
| --- | --- | --- |
| Le choix Langue était recouvert à 375 × 667. | La barre d'enregistrement restait collante et ses actions occupaient plusieurs lignes. | Barre statique quand le conteneur fait au plus 560 px ; hit-test du champ puis ouverture réelle du menu. |
| Une destination de 64 caractères était coupée dans le menu. | Le texte d'un élément flex conservait sa largeur minimum sans retour à la ligne. | Valeurs et options peuvent se replier ; vérification du texte dans l'option puis après sélection à 320 px. |
| « MEDIUM » dépassait du preset anglais à 320 px. | Quatre colonnes laissaient trop peu de place aux libellés. | Deux colonnes sous 360 px d'espace utile ; vérification des dimensions et des rectangles de chaque texte des quatre presets. |

## Couverture

Les formats principaux sont 320 × 400, 375 × 667, 640 × 480, 721 × 400, 760 × 600, 1001 × 720, 1280 × 720, 1451 × 900 et 1920 × 1080. D'autres formats complètent les tests des états et des mises à jour.

| Passe visible | Configurations | Vérification |
| --- | ---: | --- |
| Sept pages, FR/EN | 126 | Débordement, texte des boutons, contrôles accessibles après défilement, captures. |
| Édition des six destinations, erreurs, deux gardes de brouillon, trois slides | 216 | Tous les contrôles, limites du dialogue, focus, annulation, conservation de la saisie. |
| Cinq étapes de l'assistant, FR/EN, erreur de serveur | 92 | Corps défilant, pied séparé, actions accessibles, progression et état d'attente. |
| Huit menus ouverts, FR/EN | 144 | Limites du popup, nom long, dernière option atteignable et retour du focus. |
| États de diffusion, Overlay, Stream Deck, FAQ et journaux | 228 | Neuf états de diffusion, trois sections Overlay, quatre touches/vues, journaux vides/longs/erreur. |
| Rejeu après correction des presets EN à 320 px | 19 | Les neuf occurrences du défaut « MEDIUM » sont résolues ; aucun autre écart détecté dans le rejeu. |
| Confirmation de mise à jour simulée, FR/EN | 20 | Actions non recouvertes, annulation/Escape, focus et blocage si une sortie devient active. |
| Balayage des pages et dialogues complexes | 3 332 | 83 largeurs : progression de 32 px entre 320 et 1920, seuils CSS ±1 px, extension à 2560 px ; hauteurs critiques de 320 à 900 px. |
| Seuils des conteneurs | 300 | 30 largeurs autour des transitions 360/560/600 px utiles, avec et sans place occupée par le défilement vertical. |

Les 3 332 cas du balayage combinent deux hauteurs sur toutes les largeurs et un complément autour des seuils de hauteur, sur 17 surfaces. Ils ne représentent pas le produit cartésien de toutes les largeurs et hauteurs. « Toutes les tailles possibles » est un ensemble infini : la couverture repose sur des formats représentatifs, un balayage et les seuils observés.

Les contrôles automatiques mesurent les rectangles, les débordements et l'élément présent au centre des contrôles. Des captures ont également été inspectées. Cela vérifie la mise en page testée ; cela ne remplace pas un essai utilisateur ni une vérification au lecteur d'écran.

## Preuves locales

- Galerie : `output/playwright/ux-visible/index.html`.
- Résultats détaillés : `output/playwright/ux-visible/*-result.log`.
- Captures avant/après : `presets-before-320.png`, `presets-after-320.png`, `menu-setup-0-320.png` et les captures des pages/dialogues.
- Scripts de cette session : `.tmp/ux-visible-20261111/*.cli.js` ; lancement avec `npx @playwright/cli --raw -s=ux-visible run-code --filename <script>`.

Les preuves et profils de navigateur générés sont ignorés par Git. Les trois régressions durables sont dans `ui/tests/app.e2e.mjs` et `ui/tests/select.e2e.mjs`.

Validation de code : `npm run build`, `npm run lint`, `npm run test:select` et `npm run test:app` réussis. Les changements de tests ont été revus par un sous-agent TypeScript.

La fenêtre Tauri native, une vraie scène OBS, le lecteur d'écran et le matériel Stream Deck restent hors de cette matrice navigateur. Le service de revue isolé est disponible sur `http://127.0.0.1:63192` tant que cette instance reste lancée.

## Taille minimale et zoom — vérification native complémentaire

La fenêtre Windows impose désormais une taille de contenu minimale de **960 × 640 pixels logiques**. Aucune taille maximale n'est définie ; le redimensionnement et la maximisation sont explicitement autorisés.

`zoomHotkeysEnabled: false` est explicite dans `desktop/tauri.conf.json`. La version Wry 0.57.1 utilisée transmet ce réglage aux deux propriétés natives WebView2 `IsZoomControlEnabled` et `IsPinchZoomEnabled`. Le réglage était déjà faux par défaut dans Tauri ; il est maintenant documenté directement dans la configuration.

Contrôle effectué dans l'exécutable desktop reconstruit, avec données et profil WebView2 temporaires :

- Windows annonce un minimum de contenu de 960 × 640, avec redimensionnement et bouton de maximisation actifs, à une échelle DPI de 100 %.
- Chaque raccourci Ctrl+plus, Ctrl+moins et Ctrl+0 a été vérifié séparément ; Ctrl+molette et un pincement tactile synthétique ont aussi été essayés.
- `devicePixelRatio` et `visualViewport.scale` sont restés à 1 après ces actions.
- `cargo check -p open-delay-desktop` et `cargo build -p open-delay-desktop` réussissent. Preuve locale : `.tmp/native-window-check/result.json`.

Le navigateur externe et le dock OBS gardent leurs propres réglages de fenêtre et de zoom. Ces contraintes s'appliquent à la fenêtre native Open-Delay. Elles prennent effet au lancement de la version reconstruite ; un nouvel installateur reste nécessaire pour les distribuer.
