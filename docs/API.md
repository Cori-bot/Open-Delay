# Contrôle local Open-Delay v1

Le port est attribué par l'OS, puis mémorisé. Lire `runtime.json` ou le `service_info` natif ; ne pas supposer un port fixe. La fenêtre Tauri est servie depuis ses fichiers intégrés.

## Authentification

`GET /api/v1/session` avec `Origin` égal à l'origine locale de l'API retourne `{token}`. Un navigateur de même origine est également admis via `Sec-Fetch-Site: same-origin`. Le jeton va dans l'en-tête `X-OpenDelay-Token`, jamais une URL ou un fichier. Host est limité aux noms loopback et au port lié. Les origines étrangères et requêtes navigateur cross-site sont refusées. Les quotas publics et authentifiés sont séparés.

## Interfaces

| Méthode | Chemin | Contrat |
|---|---|---|
| GET | `/api/v1/health` | Nom et version, public |
| GET | `/api/v1/status` | Instantané authentifié : séquence, préférences, routes, état et endpoints réellement liés |
| GET | `/api/v1/events` | SSE authentifié `event: status`, instantané initial, séquences croissantes et keepalive ; fermeture à l'arrêt |
| POST | `/api/v1/commands` | Commande identifiée, réponse `{success,reason,message,status,results}` |
| GET/POST | `/api/v1/routes` | Lecture `{routes}` ; POST d'une route complète pour ajout/modification |
| DELETE | `/api/v1/routes/:id` | Suppression hors stream ; Twitch par défaut conservée |
| GET/POST | `/api/v1/preferences` | Lecture directe ; POST complet, réponse `{success,restart_required,preferences}` |
| GET | `/api/v1/overlay-status` | État public expurgé, aucune route/clé/préférence/URL upstream |
| GET | `/api/v1/diagnostics` | Version, statut, capacités et journaux bornés expurgés |
| POST | `/api/v1/shutdown` | Arrêt propre authentifié du moteur headless |

Commande : `id` unique (1–128 caractères), `action`, `route_ids` facultatif ; `delay_ms` pour les délais et `mode` pour le mode. Actions : `set_delay`, `prepare_delay`, `go_live`, `toggle_delay`, `set_mode`, `toggle_overlay`. Délai entier 0–600000 ms ; zéro revient toujours au direct. Mode `instant` ou `overlay`.

Sans sélection explicite, les commandes média ciblent les routes connectées incluses dans le groupe. Chaque résultat de route est vérifiable séparément ; HTTP 200 n'implique pas `success:true`. Un retry avec le même identifiant/contenu ne rejoue pas l'action ; un identifiant réutilisé pour une autre action est refusé. Le cache des 1024 dernières commandes est borné et propre au lancement : ne pas rejouer automatiquement une commande ambiguë après redémarrage.

Une route est `{id,name,platform,local_app,upstream_url,enabled,include_in_group}`. Aucun champ stream key. URL RTMP(S) de serveur sans credentials/query/fragment ; identité et chemin local uniques. Modification/suppression interdite pendant une session.

Les préférences de ports peuvent différer des ports effectivement liés tant que le redémarrage demandé n'a pas eu lieu. Utiliser toujours `status.endpoints` pour générer les URLs OBS. Les sauvegardes échouées ne modifient pas l'état en mémoire.

`effective_delay_ms` mesure le délai volontaire réellement appliqué ; `actual_latency_ms` par route mesure séparément l'écart entre source reçue et média envoyé. Une consigne n'est jamais un acquittement de bascule. L'interface rejette les réponses de forme invalide et masque un overlay dont l'état ne peut plus être rafraîchi.


## Overlay piloté (0.2)

- `GET /api/v1/overlay-control` : configuration, révision, activité récente de la source et test courant ; token requis.
- `POST /api/v1/overlay-control` : configuration OverlayConfig complète, validée et enregistrée avant activation ; token requis.
- `POST /api/v1/overlay-test` : `{ "state": "live" | "building" | "delayed" | "stop", "delay_ms": 15000 }` ; test visuel borné à 5 secondes, sans commandes média ; token requis.
- `GET /api/v1/overlay-status?viewer=<uuid>` : statut public expurgé, configuration, révision et test visuel. Le heartbeat indique une source active récemment, sans certifier qu’OBS diffuse cette scène.

La source `/overlay?managed=true` reçoit styles et tests du moteur. Les URLs autonomes avec paramètres conservent leur comportement sans adopter les tests pilotés.

## Mises à jour natives

Le service HTTP ne télécharge ni n’installe les mises à jour. Le shell Windows expose les commandes Tauri `update_status`, `check_update`, `install_update` et l’événement `update-status`. Aucun URL/paquet fourni par l’UI n’est accepté. La version candidate reste dans l’état natif après vérification du manifeste HTTPS ; le paquet est vérifié cryptographiquement avant installation. Toute route connectée bloque l’installation ; le contrôle est refait après téléchargement, avant l’arrêt propre du moteur.

`desktop_status` et `desktop-status` rendent visibles les erreurs de raccourcis/démarrage Windows. Les résultats des raccourcis et du tray sont transmis par `command-result`, y compris les refus.
