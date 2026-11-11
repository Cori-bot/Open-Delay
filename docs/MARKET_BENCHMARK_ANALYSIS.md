# Étude Comparative du Marché, Audit d'Architecture et Feuille de Route Technologique : Open-Delay

**Projet :** Open-Delay  
**Version analysée :** v0.2.2 (Commit `7890d1b`)  
**Date :** 11 Novembre 2026  
**Auteurs :** Équipe d'Ingénierie & d'Audit Open-Delay  
**Statut :** Document de Référence et de Spécification Stratégique  
**Format :** Markdown Exhaustif Français (Normes Broadcast & Architecture Logicielle)

---

## Sommaire Général

- [1. Introduction et Contexte Stratégique](#1-introduction-et-contexte-stratégique)
  - [1.1 Les Enjeux Majeurs de la Diffusion en Direct](#11-les-enjeux-majeurs-de-la-diffusion-en-direct)
  - [1.2 Les Risques Réels et Catastrophiques du Streaming Non Filtré](#12-les-risques-réels-et-catastrophiques-du-streaming-non-filtré)
  - [1.3 Le Compromis Fondamental : Interactivité vs Sécurité Opérationnelle](#13-le-compromis-fondamental--interactivité-vs-sécurité-opérationnelle)
  - [1.4 Genèse et Mission d'Open-Delay](#14-genèse-et-mission-dopen-delay)
- [2. Audit Approfondi de l'Architecture Open-Delay (R1)](#2-audit-approfondi-de-larchitecture-open-delay-r1)
  - [2.1 Le Moteur Rust (Core, RTMP et Service)](#21-le-moteur-rust-core-rtmp-et-service)
    - [2.1.1 `crates/core` : Ring Buffer Circulaire Disque et Contrôleur Temporel](#211-cratescore--ring-buffer-circulaire-disque-et-contrôleur-temporel)
    - [2.1.2 Algorithme d'Éviction BTreeMap et Cache LRU Mémoire](#212-algorithme-déviction-btreemap-et-cache-lru-mémoire)
    - [2.1.3 Machine à États : Modes `Instant` vs `Overlay` et Actions](#213-machine-à-états--modes-instant-vs-overlay-et-actions)
    - [2.1.4 `crates/rtmp` : Implémentation Bas Niveau du Protocole](#214-cratesrtmp--implémentation-bas-niveau-du-protocole)
    - [2.1.5 `crates/service` : Moteur Réseau Asynchrone, Proxy RTMP et API Axum](#215-cratesservice--moteur-réseau-asynchrone-proxy-rtmp-et-api-axum)
    - [2.1.6 Cadencement Monotone et Ancrage Temporel (`TrackCadence`)](#216-cadencement-monotone-et-ancrage-temporel-trackcadence)
  - [2.2 La Couche Desktop Tauri v2](#22-la-couche-desktop-tauri-v2)
    - [2.2.1 Gestion du Cycle de Vie et Processus Embarqué In-Process](#221-gestion-du-cycle-de-vie-et-processus-embarqué-in-process)
    - [2.2.2 Intégration Système : Raccourcis Globaux et Menu Tray](#222-intégration-système--raccourcis-globaux-et-menu-tray)
    - [2.2.3 Sécurité, Permissions Strictes et Mises à Jour Signées](#223-sécurité-permissions-strictes-et-mises-à-jour-signées)
  - [2.3 L'Interface Frontend React 19 & TypeScript](#23-linterface-frontend-react-19--typescript)
    - [2.3.1 Architecture Frontend et Hook Unifié `useEngine.ts`](#231-architecture-frontend-et-hook-unifié-useenginets)
    - [2.3.2 L'Éditeur d'Overlay Dynamique (`OverlayEditor.tsx`)](#232-léditeur-doverlay-dynamique-overlayeditortsx)
    - [2.3.3 Intégration Dock OBS Responsive et Gestion Multi-Routes](#233-intégration-dock-obs-responsive-et-gestion-multi-routes)
  - [2.4 Intégrations Matérielles & Banc de Validation](#24-intégrations-matérielles--banc-de-validation)
    - [2.4.1 Plugin Officiel Elgato Stream Deck (SDK v3, Node.js 24)](#241-plugin-officiel-elgato-stream-deck-sdk-v3-nodejs-24)
    - [2.4.2 Banc de Tests Synthétiques E2E FFmpeg/FFprobe](#242-banc-de-tests-synthétiques-e2e-ffmpegffprobe)
- [3. Étude Comparative du Marché des Solutions Logicielles (R2)](#3-étude-comparative-du-marché-des-solutions-logicielles-r2)
  - [3.1 Solutions Logicielles Dédiées Spécialisées](#31-solutions-logicielles-dédiées-spécialisées)
    - [3.1.1 InstantDelay v3.0 (Lypningeuh)](#311-instantdelay-v30-lypningeuh)
    - [3.1.2 LiveDelay (skullboypl)](#312-livedelay-skullboypl)
    - [3.1.3 InstantClone (Soulhackzlol / s1moscs)](#313-instantclone-soulhackzlol--s1moscs)
    - [3.1.4 Streamdelay (Streamwall)](#314-streamdelay-streamwall)
    - [3.1.5 Plugins Internes OBS (`obs-broadcast-delay` / `obs-broadcast-censure`)](#315-plugins-internes-obs-obs-broadcast-delay--obs-broadcast-censure)
  - [3.2 Suites Logicielles de Production Broadcast](#32-suites-logicielles-de-production-broadcast)
    - [3.2.1 Fonctionnalité Native "Délai du flux" dans OBS Studio](#321-fonctionnalité-native-délai-du-flux-dans-obs-studio)
    - [3.2.2 vMix (Studio, 4K, Pro, MAX)](#322-vmix-studio-4k-pro-max)
    - [3.2.3 Telestream Wirecast](#323-telestream-wirecast)
    - [3.2.4 TriCaster (Vizrt / NewTek — Éditions Logicielles)](#324-tricaster-vizrt--newtek--éditions-logicielles)
  - [3.3 Analyse Comparative du Matériel Broadcast Traditionnel](#33-analyse-comparative-du-matériel-broadcast-traditionnel)
    - [3.3.1 Les Références Broadcast : Eventide BD600 / BD600W+ et Evertz SDI](#331-les-références-broadcast--eventide-bd600--bd600w-et-evertz-sdi)
    - [3.3.2 Transposition des Concepts Clés : Dump, Cough, Panic et Squeeze](#332-transposition-des-concepts-clés--dump-cough-panic-et-squeeze)
    - [3.3.3 Le Stream Deck : Le Pupitre Broadcast Physique Universel](#333-le-stream-deck--le-pupitre-broadcast-physique-universel)
  - [3.4 Cas d'Usage Réels et Scénarios Opérationnels](#34-cas-dusage-réels-et-scénarios-opérationnels)
    - [3.4.1 Anti-Stream Sniping (Esport, Battle Royale et Poker en Ligne)](#341-anti-stream-sniping-esport-battle-royale-et-poker-en-ligne)
    - [3.4.2 Sécurité Physique, Vie Privée et Anti-Doxxing / Anti-Swatting (IRL)](#342-sécurité-physique-vie-privée-et-anti-doxxing--anti-swatting-irl)
    - [3.4.3 Modération et Conformité TOS/DMCA (Trolls Vocaux et Musique)](#343-modération-et-conformité-tosdmca-trolls-vocaux-et-musique)
  - [3.5 Grille d'Évaluation Multicritères Exhaustive (10 Solutions x 10 Critères)](#35-grille-dévaluation-multicritères-exhaustive-10-solutions-x-10-critères)
- [4. Diagnostic Critique des Faiblesses, Manques et Anomalies d'Open-Delay (R3)](#4-diagnostic-critique-des-faiblesses-manques-et-anomalies-dopen-delay-r3)
  - [4.1 Rejet Formel d'Enhanced RTMP (AV1, HEVC/H.265 et HDR)](#41-rejet-formel-denhanced-rtmp-av1-hevch265-et-hdr)
  - [4.2 Rejet de l'Audio Multi-Piste (Twitch VOD Track & Protection DMCA)](#42-rejet-de-laudio-multi-piste-twitch-vod-track--protection-dmca)
  - [4.3 Vulnérabilité de Purge du Buffer lors d'une Micro-Coupure OBS](#43-vulnérabilité-de-purge-du-buffer-lors-dune-micro-coupure-obs)
  - [4.4 Vulnérabilité Temporelle lors du "Go Live" (Fuite d'Images en Attente de Keyframe)](#44-vulnérabilité-temporelle-lors-du-go-live-fuite-dimages-en-attente-de-keyframe)
  - [4.5 Limite des 10 Minutes et Saturation I/O Synchrone du Disque](#45-limite-des-10-minutes-et-saturation-io-synchrone-du-disque)
  - [4.6 Perte de Contrôle du Dock OBS sans Focus de Fenêtre](#46-perte-de-contrôle-du-dock-obs-sans-focus-de-fenêtre)
  - [4.7 Polling HTTP du Plugin Stream Deck et Absence d'Éléments Dynamiques](#47-polling-http-du-plugin-stream-deck-et-absence-déléments-dynamiques)
- [5. Axes d'Amélioration des Fonctionnalités Existantes (R4)](#5-axes-damélioration-des-fonctionnalités-existantes-r4)
  - [5.1 Perfectionnement de la Gestion du Délai (Montée et Descente Douces)](#51-perfectionnement-de-la-gestion-du-délai-montée-et-descente-douces)
  - [5.2 Perfectionnement de l'Éditeur d'Overlay (Catalogue de Thèmes et CSS Moderne)](#52-perfectionnement-de-léditeur-doverlay-catalogue-de-thèmes-et-css-moderne)
  - [5.3 Intégration OBS Perfectionnée via OBS-WebSocket v5](#53-intégration-obs-perfectionnée-via-obs-websocket-v5)
  - [5.4 Modernisation Réactive du Plugin Stream Deck](#54-modernisation-réactive-du-plugin-stream-deck)
- [6. Feuille de Route des Nouvelles Fonctionnalités Recommandées (R5)](#6-feuille-de-route-des-nouvelles-fonctionnalités-recommandées-r5)
  - [6.1 Mode Censure Audio Ciblée ("Bleep Censor" Sélectif)](#61-mode-censure-audio-ciblée-bleep-censor-sélectif)
  - [6.2 Mode "Catch-Up" / Rattrapage Temporel Automatique sans Distorsion de Pitch (WSOLA)](#62-mode-catch-up--rattrapage-temporel-automatique-sans-distorsion-de-pitch-wsola)
  - [6.3 Boucle Vidéo et Écran de Secours d'Urgence au Dump ("Safety Slate")](#63-boucle-vidéo-et-écran-de-secours-durgence-au-dump-safety-slate)
  - [6.4 Prise en Charge Native Enhanced RTMP v1/v2 & Codecs Next-Gen](#64-prise-en-charge-native-enhanced-rtmp-v1v2--codecs-next-gen)
  - [6.5 Relais Hybride SRT (Secure Reliable Transport) & WebRTC / WHIP](#65-relais-hybride-srt-secure-reliable-transport--webrtc--whip)
  - [6.6 Module d'IA Locale "Leak Shield" (Détection Vocale et Visuelle Automatisée)](#66-module-dia-locale-leak-shield-détection-vocale-et-visuelle-automatisée)
  - [6.7 Profils de Configuration par Jeu / Plateforme](#67-profils-de-configuration-par-jeu--plateforme)
  - [6.8 Matrice de Priorisation de la Feuille de Route (P0, P1, P2, P3)](#68-matrice-de-priorisation-de-la-feuille-de-route-p0-p1-p2-p3)
- [7. Synthèse et Recommandations Opérationnelles pour Open-Delay](#7-synthèse-et-recommandations-opérationnelles-pour-open-delay)

---

## 1. Introduction et Contexte Stratégique

### 1.1 Les Enjeux Majeurs de la Diffusion en Direct

L'avènement du streaming en direct sur des plateformes telles que Twitch, YouTube Live, Kick et TikTok Live a profondément transformé le divertissement numérique. À la différence de la télédiffusion linéaire traditionnelle, la proposition de valeur fondamentale du live streaming réside dans **l'immédiateté de l'interaction** : un spectateur envoie un message dans le tchat, et le créateur réagit vocalement ou visuellement quelques secondes plus tard. Pour maximiser cette symbiose, les plateformes et les encodeurs ont consacré d'immenses efforts d'ingénierie à la réduction de la latence de transport, atteignant couramment 1 à 3 secondes via le protocole RTMP optimisé (Low-Latency HLS ou WebRTC).

Cependant, cette suppression de l'inertie temporelle transforme l'antenne du streamer en un environnement à haut risque opérationnel, où la moindre fraction de seconde d'inattention ou de malveillance extérieure peut avoir des répercussions immédiates, irréversibles et destructrices.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       LE DILEMME FONDAMENTAL DU STREAMER                     │
├──────────────────────────────────────┬──────────────────────────────────────┤
│        ULTRA-BASSE LATENCE           │           SÉCURITÉ & DÉLAI           │
│        (1 à 3 secondes)              │         (30 à 180 secondes)          │
├──────────────────────────────────────┼──────────────────────────────────────┤
│ • Interaction instantanée tchat      │ • Protection anti-sniping absolue    │
│ • Engagement maximal de l'audience   │ • Neutralisation des fuites (Doxxing)│
│ • Réactions aux dons en direct       │ • Éjection des insultes vocales (TOS)│
│                                      │                                      │
│ RISQUES CRITIQUES :                  │ INCONVÉNIENTS MAJEURS :              │
│ ❌ Triche et sabotage de parties     │ ⚠️ Tchat décalé par rapport au live  │
│ ❌ Doxxing, SWATting et agressions   │ ⚠️ Réactions aux subs différées      │
│ ❌ Bannissement de chaîne (ToS/DMCA) │ ⚠️ Complexité de bascule technique   │
└──────────────────────────────────────┴──────────────────────────────────────┘
```

---

### 1.2 Les Risques Réels et Catastrophiques du Streaming Non Filtré

L'absence d'un sas de sécurité temporel expose les créateurs de contenu à trois catégories de menaces documentées et récurrentes :

#### 1. Le "Stream Sniping" et la Triche Compétitive (Esport)
Dans les jeux vidéo compétitifs (Battle Royale comme *Call of Duty: Warzone* ou *Apex Legends*, FPS tactiques comme *Valorant* et *Counter-Strike 2*, jeux de stratégie comme *League of Legends*, ou tables de poker en ligne), des adversaires se connectent simultanément à la diffusion du joueur pour voir son écran en temps réel.
- **Conséquences** : Les adversaires connaissent la position exacte du streamer, l'état de sa santé, ses armes, ses munitions, ses temps de recharge (*cooldowns*), son économie, ainsi que les cartes dissimulées de sa main de poker. Le streamer perd systématiquement ses parties, son classement compétitif (*MMR*) s'effondre, son expérience de jeu devient toxique et ses spectateurs assistent à un spectacle faussé.

#### 2. Les Atteintes à la Vie Privée et à la Sécurité Physique (Doxxing et Swatting)
Le streaming sédentaire (sur PC) et le streaming d'extérieur (*IRL - In Real Life*) multiplient les occasions de fuites accidentelles d'informations confidentielles :
- **En streaming PC** : Un raccourci clavier `Alt-Tab` malencontreux, une notification d'e-mail intempestive, l'affichage inopiné d'un gestionnaire de mots de passe, d'une interface bancaire, d'une commande de livraison de nourriture affichant l'adresse du domicile, ou d'un fichier de configuration révélant un jeton API secret.
- **En streaming IRL** : Des individus malveillants identifient l'emplacement géographique du créateur via les reflets dans les vitrines, les plaques de rue, les enseignes de magasins ou les immatriculations de véhicules.
- **L'escalade du "Swatting"** : Des spectateurs malveillants contactent les forces d'intervention de police (SWAT, BRI, GIGN) sous un faux prétexte d'urgence terroriste ou criminelle au domicile du streamer, provoquant des interventions armées en plein direct avec un risque mortel pour le créateur et sa famille.

#### 3. Le Bannissement de Chaîne pour Non-Respect des Règles Communautaires (TOS et DMCA)
Les plateformes comme Twitch et YouTube appliquent une politique de tolérance zéro envers les contenus haineux, le harcèlement, la nudité et les violations de propriété intellectuelle :
- **Attaques par proxy dans les jeux vocaux** : Dans les lobbies multijoueurs non filtrés (*Rust*, *Call of Duty*, *GTA RP*), des "stream snipers" vocaux hurlent délibérément des insultes racistes, homophobes, antisémites ou des menaces de mort dès qu'ils identifient un streamer en direct.
- **Piratage visuel** : Affichage d'emblèmes personnalisés, de pseudos obscènes ou de photos explicites dans le jeu.
- **Frappes de droits d'auteur (DMCA Strikes)** : Diffusion fortuite d'une musique commerciale lors du passage d'une voiture dans un jeu vidéo ou dans un commerce en streaming IRL.
- **Impact financier** : Un seul manquement peut entraîner la suspension immédiate du compte pour une durée de 24 heures, 7 jours, 30 jours, ou un bannissement définitif (*perma-ban*), anéantissant du jour au lendemain le modèle économique et l'activité professionnelle du créateur.

---

### 1.3 Le Compromis Fondamental : Interactivité vs Sécurité Opérationnelle

La problématique du streamer ne consiste pas à choisir unilatéralement entre un délai permanent de 2 minutes ou un direct absolu à 0 seconde. Elle réside dans **l'agilité temporelle** :
- **Pendant les phases d'action intense ou de risque** (lancement d'une partie compétitive de 40 minutes, saisie de coordonnées bancaires sur un site web, déplacement dans la rue en streaming IRL), le diffuseur a besoin d'un **délai de protection hermétique** (de 30 secondes à plusieurs minutes).
- **Pendant les phases calmes et communautaires** (discussion d'après-match, débriefing, séance de *Just Chatting*, lecture des dons), le diffuseur a impérativement besoin d'un **retour immédiat au direct (0 seconde)** pour échanger avec son audience sans latence.
- **En cas d'incident critique imprévu** (insulte proférée dans le chat vocal il y a 5 secondes, ou fenêtre sensible ouverte par erreur), le créateur a besoin d'un **bouton d'éjection d'urgence (*Panic Dump*)** capable de faire disparaître les secondes incriminées de la mémoire tampon avant qu'elles ne soient expédiées sur les serveurs de diffusion publics, tout en maintenant le flux en direct actif.

---

### 1.4 Genèse et Mission d'Open-Delay

Face à ce constat, les streamers étaient historiquement contraints de choisir entre des solutions payantes par abonnement mensuel onéreux et instables (comme *InstantDelay*), des plugins expérimentaux nécessitant des compétences poussées d'administration système, ou les réglages rigides natifs d'OBS Studio obligeant à couper entièrement la diffusion pour modifier le délai.

**Open-Delay** est né avec l'ambition de fournir une solution logicielle universelle, 100 % locale, gratuite (*source-available*), ultra-sécurisée et sans dépendance au cloud. En s'insérant comme un **proxy de streaming local intermédiaire** entre l'encodeur (OBS Studio) et les plateformes de diffusion (Twitch, YouTube, Kick), Open-Delay redonne au streamer le contrôle total sur l'axe du temps de son flux.

---

## 2. Audit Approfondi de l'Architecture Open-Delay (R1)

L'audit technique du code source d'Open-Delay (commit `7890d1b`, v0.2.2) révèle une architecture logicielle moderne, modulaire et résiliente, conçue en Rust avec une couche de présentation native Tauri v2 et une interface utilisateur réactive React 19 / TypeScript.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                 ARCHITECTURE GLOBALE DU SYSTÈME OPEN-DELAY                  │
└─────────────────────────────────────────────────────────────────────────────┘

 [ ENCODEUR OBS STUDIO ] 
        │ (RTMP Local Ingest via loopback TCP)
        ▼
 ┌───────────────────────────────────────────────────────────────────────────┐
 │                   APPLICATION DESKTOP TAURI v2 (WINDOWS)                  │
 │  ┌─────────────────────────────────────────────────────────────────────┐  │
 │  │                         MOTEUR RUST (SERVICE)                       │  │
 │  │                                                                     │  │
 │  │   ┌─────────────────────┐             ┌─────────────────────────┐   │  │
 │  │   │     CRATES/RTMP     │             │       CRATES/CORE       │   │  │
 │  │   │  • Handshake v3     │             │  • Machine à états      │   │  │
 │  │   │  • Framing Chunks   │             │  • TrackCadence (DTS)   │   │  │
 │  │   │  • Décodeur AMF0    │             │  • Action::GoLive       │   │  │
 │  │   └──────────┬──────────┘             └────────────┬────────────┘   │  │
 │  │              │                                     │                │  │
 │  │              ▼                                     ▼                │  │
 │  │   ┌─────────────────────────────────────────────────────────────┐   │  │
 │  │   │          STOCKAGE HYBRIDE CIRCULAIRE (crates/core)          │   │  │
 │  │   │  ┌────────────────────────┐     ┌────────────────────────┐  │   │  │
 │  │   │  │   Cache RAM LRU        │     │  Ring Buffer Disque    │  │   │  │
 │  │   │  │   (64 Mo préalloué)    │◄───►│  history-<uuid>.ring   │  │   │  │
 │  │   │  │   Accès sub-millisecond│     │  (2 Gio préalloué)     │  │   │  │
 │  │   │  └────────────────────────┘     └────────────────────────┘  │   │  │
 │  │   └──────────────────────────────┬──────────────────────────────┘   │  │
 │  │                                  │                                  │  │
 │  │                                  ▼ (Playback / Replay différé)      │  │
 │  │   ┌─────────────────────────────────────────────────────────────┐   │  │
 │  │   │       SERVEUR D'API AXUM + RELAIS SORTANT (crates/service)  │   │  │
 │  │   │  • RTMPS Upstream sécurisé (TLS 1.3 Rustls) vers Twitch/YT  │   │  │
 │  │   │  • Serveur HTTP Loopback (Auth Token UUIDv4, CSP, Rate Limit│   │  │
 │  │   │  • Flux d'événements SSE (/api/v1/events)                   │   │  │
 │  │   └──────────────────────────────┬──────────────────────────────┘   │  │
 │  └──────────────────────────────────┼──────────────────────────────────┘  │
 │                                     │                                     │
 │  ┌──────────────────────────────────▼──────────────────────────────────┐  │
 │  │                   INTERFACE GRAPHIQUE (WEBVIEW TAURI)               │  │
 │  │   • React 19 / TypeScript / Vite / Motion                           │  │
 │  │   • Hook unifié useEngine.ts (SSE + Fallback Polling)               │  │
 │  │   • Composant Éditeur d'Overlay temps réel                          │  │
 │  │   • Contrôles de diffusion et sélecteurs de presets                 │  │
 │  └─────────────────────────────────────────────────────────────────────┘  │
 └───────────────────────┬─────────────────────────────────┬─────────────────┘
                         │                                 │
     (HTTP Local Discovery via runtime.json)               │ (Source Navigateur OBS)
                         ▼                                 ▼
         ┌───────────────────────────────┐ ┌────────────────────────────────┐
         │ PLUGIN ELGATO STREAM DECK     │ │ DOCK NAVIGATEUR & OVERLAY OBS  │
         │ (Node.js 24 / SDK v3)         │ │ (http://127.0.0.1:PORT/overlay)│
         └───────────────────────────────┘ └────────────────────────────────┘
```

---

### 2.1 Le Moteur Rust (Core, RTMP et Service)

#### 2.1.1 `crates/core` : Ring Buffer Circulaire Disque et Contrôleur Temporel
Le cœur de gestion du stockage temporaire est implémenté dans le module `crates/core/src/history.rs` au sein de la structure `DiskHistory`. Contrairement aux solutions naïves qui multiplient les écritures de petits fichiers MP4 ou TS fragmentés (générant une dégradation rapide des SSD et des risques d'usure des cellules NAND), Open-Delay utilise une approche par **fichier circulaire unique préalloué** :

- **Préallocation déterministe** :
  À la création de l'historique (`history.rs:76-81`), le fichier `history-<uuid>.ring` est immédiatement calibré à sa taille maximale sur le disque via l'appel système :
  ```rust
  // crates/core/src/history.rs:78-80
  file.set_len(capacity)?;
  ```
  La capacité par défaut est fixée à 2 Gio (`DEFAULT_DISK_BYTES = 2 * 1024 * 1024 * 1024`, `config.rs:5`), et peut être étendue dans les réglages jusqu'à 32 Gio (`config.rs:144`). Cette réservation d'espace contigu garantit qu'aucun crash par manque d'espace disque imprévu ne survient au cours de la diffusion.
- **Cycle de vie et nettoyage automatique des orphelins** :
  L'implémentation du trait `Drop` sur `DiskHistory` (`history.rs:387-398`) supprime automatiquement le fichier `.ring` à la fermeture normale du service. De plus, pour gérer les arrêts brutaux de Windows (coupure secteur, écran bleu), la fonction `clean_abandoned_history` (`crates/service/src/lib.rs:210-233`) inspecte le dossier de données au démarrage et purge tous les fichiers `history-*.ring` orphelins non verrouillés.

#### 2.1.2 Algorithme d'Éviction BTreeMap et Cache LRU Mémoire
La gestion des adresses d'écriture et de lecture dans le fichier circulaire repose sur un algorithme précis :
- **Curseur Circulaire** : Le pointeur d'écriture `self.cursor` progresse de manière linéaire. Lorsque `cursor + payload.len() > capacity`, le curseur boucle à 0 (`history.rs:118-120`).
- **Suivi des Régions Occupées via BTreeMap** :
  La structure `regions: BTreeMap<u64, (u64, usize)>` associe l'offset physique dans le fichier à un tuple `(sequence, size)`. Avant d'écrire un nouveau paquet média, le moteur calcule l'intervalle d'octets qui sera écrasé par le curseur. La méthode `evict_front()` (`history.rs:353-368`) identifie les paquets obsolètes ou partiellement écrasés et purge chronologiquement leurs entrées dans la file de métadonnées `entries: VecDeque<PacketMeta>`.
- **Cache LRU Mémoire Vive (RAM)** :
  Pour minimiser les entrées/sorties physiques et préserver la durée de vie des SSD, un cache en mémoire de 64 Mo par défaut (`DEFAULT_CACHE_BYTES = 64 * 1024 * 1024`, `history.rs:57-61`) conserve les paquets média les plus récents dans un `HashMap<u64, Arc<[u8]>>` synchronisé par un `VecDeque<u64>`. Lorsqu'un lecteur requiert un paquet présent dans le cache mémoire, il est renvoyé instantanément par clonage de pointeur atomique (`history.rs:176-181`), sans toucher au disque dur.
- **Indexation Temporelle et Recherche de Keyframes** :
  La recherche du point de coupure ou de reprise s'effectue via la méthode `seek_keyframe` (`history.rs:193-210`), qui remonte la file de métadonnées pour localiser l'image clé (*IDR Keyframe*) immédiatement antérieure au timestamp désiré (`p.keyframe && !p.configuration && p.timestamp_ms <= target_ms`). La méthode `configurations` (`history.rs:249-267`) renvoie les derniers paquets d'initialisation de décodeur (SPS/PPS pour le H.264 et AudioSpecificConfig pour l'AAC) afin de garantir une synchronisation parfaite du décodeur distant lors d'un saut temporel.

#### 2.1.3 Machine à États : Modes `Instant` vs `Overlay` et Actions
Le contrôleur (`crates/core/src/controller.rs`) arbitre l'état du flux et applique les règles de transition :
- **Mode `Instant`** (`config.rs:28`) :
  Le buffer accumule les données en tâche de fond. Dès que la durée disponible dans l'historique atteint la consigne configurée (`available >= target`), le contrôleur ordonne un saut temporel immédiat (`TimelineActionKind::Seek`) à l'instant `latest_source_ms - delay_ms` (`controller.rs:651-664`).
- **Mode `Overlay`** (`config.rs:29`) :
  Si le flux est actuellement en direct (`effective == 0`), le passage en différé ne tronque pas le flux. L'état `overlay_wait = true` est enclenché (`controller.rs:563-568`) : le direct continue d'être transmis vers la plateforme pendant que l'overlay web informe les spectateurs de la constitution du délai. Dès que le temps d'attente est écoulé (`waited >= delay`), le flux bascule de façon continue vers les données différées.
- **Politique de Non-Dégradation Silencieuse** :
  Si un problème réseau ou une insuffisance de débit empêche de maintenir le délai requis (`available + 2000ms < effective`), le contrôleur refuse catégoriquement de renvoyer silencieusement le streamer vers le direct sans son accord. Il positionne `degraded = true` et émet `TimelineActionKind::Stop` (`controller.rs:616-629`), coupant la diffusion pour protéger l'anonymat du créateur plutôt que de divulguer du contenu protégé.
- **Action de Retour au Direct (`Action::GoLive`)** :
  L'action `GoLive` réinitialise immédiatement le délai désiré à zéro (`desired = 0`), incrémente la révision temporelle et commande un saut vers la prochaine trame clé live (`controller.rs:668-683`).

#### 2.1.4 `crates/rtmp` : Implémentation Bas Niveau du Protocole
Le module `crates/rtmp` constitue une pile protocolaire pure Rust, sans dépendance C et sans recours aux bibliothèques C++ de FFmpeg :
- **Handshake RTMP v3 (`handshake.rs`)** :
  Implémente l'échange standard de paquets de 1 536 octets (C0/S0, C1/S1, C2/S2) requis par le protocole RTMP pour négocier la session avec OBS et les serveurs distants.
- **Découpage incrémental en Chunks (`chunk.rs`)** :
  La structure `ChunkDecoder` (`chunk.rs:45-260`) décode les 4 formats d'en-tête de chunks RTMP (Type 0 à Type 3), supporte les Basic Headers sur 1 à 3 octets, et gère rigoureusement les estampilles temporelles étendues (*Extended Timestamps* sur 32 bits, `0x00ffffff`).
- **Sécurité et Bornage Mémoire Défensif** :
  Pour interdire tout risque d'attaque par déni de service (*DoS*) ou d'épuisement mémoire par des paquets corrompus, le décodeur impose des limites strictes : taille maximale de message bornée à 16 Mo (`MAX_MESSAGE = 16 MiB - 1`), buffer partiel limité à 32 Mo (`MAX_PARTIAL_BYTES = 32 MiB`), et nombre de flux de chunks simultanés plafonné à 128 (`MAX_CSID_COUNT = 128`).
- **Sérialisation AMF0 (`amf0.rs`)** :
  Encode et décode l'ensemble des types de données AMF0 (`Number`, `Boolean`, `String`, `Object`, `EcmaArray`, etc.) avec une protection contre les attaques par dépassement de pile en profondeur maximale de récursion (`MAX_DEPTH = 32`) et une limite de 100 000 éléments (`MAX_VALUES = 100_000`).

#### 2.1.5 `crates/service` : Moteur Réseau Asynchrone, Proxy RTMP et API Axum
Le crate `crates/service` orchestre la concurrence réseau sous le runtime asynchrone **Tokio** :
- **Verrouillage d'Instance Unique (`lib.rs:116-124`)** :
  L'exécutable acquiert un verrou exclusif sur le fichier `%LOCALAPPDATA%/Open-Delay/instance.lock` via l'appel `fs2::FileExt::try_lock_exclusive`. Cela empêche toute corruption d'état par le lancement accidentel de deux instances concurrentes.
- **Allocation Dynamique de Ports Sans Conflit (`lib.rs:193-207`)** :
  Le service tente d'abord d'écouter sur les ports configurés par l'utilisateur. Si un port est déjà réservé par un autre logiciel (conflit `AddrInUse`), le système s'associe automatiquement au port libre `0` attribué par le noyau Windows. Les ports finaux réels sont publiés immédiatement dans le fichier `runtime.json` pour la découverte locale.
- **Protection Anti-Usurpation de Clé de Stream (`engine.rs:316-320`)** :
  Lorsqu'OBS s'authentifie auprès du serveur RTMP local, la clé de stream transmise dans la commande `publish` fait l'objet d'un hachage cryptographique **SHA-256** (`publish_digest`). La clé brute n'est jamais consignée dans les fichiers journaux ni écrite sur le disque. Une route active refuse toute connexion concurrente qui ne présenterait pas la même empreinte SHA-256 (`same_publish_name`, `engine.rs:464-469`).
- **Serveur d'API Axum Sécurisé (`api.rs:50-151`)** :
  L'API locale et la distribution de l'UI sont régies par un middleware de sécurité strict :
  - Filtrage drastique des en-têtes `Host` (limité strictement à `127.0.0.1:{port}` ou `localhost:{port}`).
  - Blocage des requêtes externes via `Sec-Fetch-Site: cross-site`.
  - Rate limiting local plafonné à 120 requêtes par seconde par adresse IP de bouclage.
  - Authentification obligatoire par jeton de session cryptographique aléatoire **UUIDv4** (`X-OpenDelay-Token`), renouvelé à chaque démarrage du processus.

#### 2.1.6 Cadencement Monotone et Ancrage Temporel (`TrackCadence`)
Lorsqu'un décalage temporel ou un retour direct est exécuté, les horloges temporelles du flux d'origine subissent une rupture de continuité. Si des paquets RTMP présentaient des estampilles temporelles régressives (DTS inférieur au paquet précédent), les serveurs de Twitch et YouTube déconnecteraient immédiatement la diffusion.
Pour éradiquer ce risque, Open-Delay intègre la structure `TrackCadence` (`engine.rs:146-168`) :
- Elle calcule la période moyenne entre trames pour les flux vidéo et audio.
- Lors de chaque transition, la méthode `anchor_output` (`engine.rs:1287`) recalcule une nouvelle ligne d'ancrage temporelle monotone strictement croissante pour la sortie, garantissant une stabilité absolue du flux vers les serveurs cibles, sans micro-saccades ni rejets d'ingest.

---

### 2.2 La Couche Desktop Tauri v2

#### 2.2.1 Gestion du Cycle de Vie et Processus Embarqué In-Process
L'application de bureau Windows repose sur le framework **Tauri v2** (`desktop/src/main.rs`). Contrairement aux architectures Electron qui lancent un environnement Node.js lourd en arrière-plan, Open-Delay démarre son moteur de service réseau **in-process** au sein d'un runtime Tokio asynchrone orchestré par Tauri (`desktop/src/main.rs:253-261`). Cette intégration garantit qu'aucun processus démon orphelin ne peut subsister dans le Gestionnaire des tâches de Windows après la fermeture du logiciel.

#### 2.2.2 Intégration Système : Raccourcis Globaux et Menu Tray
- **Interception de la Fermeture et Minimisation Systray** :
  La fermeture de la fenêtre principale est interceptée par l'événement `CloseRequested` (`desktop/src/main.rs:347-364`). Si l'option de fermeture complète n'est pas cochée, l'application se réduit discrètement dans la barre des tâches de notification de Windows (*Systray*).
- **Protection Anti-Coupure en Plein Direct** :
  Si l'utilisateur tente de quitter l'application alors qu'un flux est en cours de diffusion vers une plateforme, une boîte de dialogue modale native bloquante avertit l'utilisateur : *"Un stream est actif. Quitter coupe la sortie."* (`desktop/src/main.rs:205-217`).
- **Raccourcis Clavier Système Globaux (`tauri-plugin-global-shortcut`)** :
  L'application permet d'assigner 4 raccourcis claviers globaux Windows opérationnels même lorsque le streamer est en plein jeu vidéo : `start_delay`, `go_live`, `toggle_overlay`, et `custom_delay`. L'appui sur le raccourci invoque directement la méthode interne du contrôleur Rust, assurant un temps de réponse inférieur à 5 millisecondes.

#### 2.2.3 Sécurité, Permissions Strictes et Mises à Jour Signées
- **Politique de Sécurité du Contenu (CSP)** :
  Le fichier `desktop/tauri.conf.json` impose une directive CSP drastique :
  ```json
  "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost http://127.0.0.1:* http://localhost:*; frame-src 'self'; object-src 'none'; base-uri 'self'"
  ```
  Aucun script externe, police tierce ou iframe distante ne peut être exécuté dans la webview.
- **Mises à Jour Cryptographiquement Signées par Minisign (`updates.rs`)** :
  Le système de mise à jour automatique vérifie l'intégrité de l'exécutable téléchargé à l'aide d'une clé publique **Minisign** embarquée dans le binaire. De surcroît, la méthode `ensure_idle` (`updates.rs:130-142`) bloque impérativement toute tentative de redémarrage ou d'application de patch si une diffusion est en cours.

---

### 2.3 L'Interface Frontend React 19 & TypeScript

#### 2.3.1 Architecture Frontend et Hook Unifié `useEngine.ts`
L'interface utilisateur (`ui/`) est conçue avec **React 19.2**, **TypeScript 5.9**, **Vite 7** et la suite de composants headless `@base-ui/react`.
Le pilotage de l'application est centralisé dans le hook personnalisé maître `useEngine.ts` :
- **Synchronisation Hybride Réactive** : Le hook écoute en priorité le flux d'événements temps réel SSE (`/api/v1/events`). Si la connexion SSE est interrompue, un mécanisme de repli adaptatif bascule automatiquement en polling HTTP (1 seconde lorsque l'onglet est actif, 5 secondes lorsqu'il est masqué via `document.hidden`).
- **File d'Attente Séquentielle de Commandes (`commandQueue`)** :
  Pour éradiquer tout risque de collision d'actions causé par des clics rapides (ex: préparer un délai puis cliquer immédiatement sur retour au direct), une file d'attente asynchrone (`commandQueue`, lignes 80-87) sérialise l'envoi des requêtes et garantit leur traitement dans l'ordre chronologique strict.
- **Validation Runtime Rigoureuse (`validation.ts`)** : Toutes les données JSON reçues du backend sont validées à l'exécution par des schémas de typage stricts avant d'être injectées dans l'arbre d'état React.

#### 2.3.2 L'Éditeur d'Overlay Dynamique (`OverlayEditor.tsx`)
L'éditeur d'overlay permet de configurer en direct l'apparence visuelle de la source navigateur intégrée dans OBS :
- **Personnalisation Complète** : Choix du thème graphique (Clair, Sombre, Minimaliste, Néon, etc.), des couleurs primaires/secondaires, du rayon de bordure, de la position sur l'écran (9 ancrages), de la typographie, des animations de pulsation et des textes personnalisés.
- **Mode Test Minuté Sans Risque** : Le composant intègre un bouton de simulation permettant de déclencher l'affichage de l'overlay pendant 5 secondes pour tester son rendu dans OBS sans altérer la diffusion réelle du stream.
- **Détection de Présence de Source OBS** : Le gestionnaire mesure la pulsation cardiaque (*heartbeat*) de la source navigateur OBS et affiche un indicateur visuel confirmant si la source est activement connectée ou fermée.

#### 2.3.3 Intégration Dock OBS Responsive et Gestion Multi-Routes
- **Adaptation aux Docks Étroits** : L'interface web intègre des règles CSS modernes basées sur les *Container Queries* (`@container`) et *Media Queries*, lui permettant de s'afficher élégamment dans un dock OBS Studio vertical de seulement 320 pixels de largeur.
- **Gestion Indépendante des Destinations Multi-Plateformes** : Open-Delay permet de configurer jusqu'à 32 routes distinctes (Twitch, YouTube, Kick, TikTok, Restream). L'utilisateur peut choisir de soumettre l'ensemble des plateformes au même délai global, ou d'appliquer un délai uniquement à certaines destinations sensibles.

---

### 2.4 Intégrations Matérielles & Banc de Validation

#### 2.4.1 Plugin Officiel Elgato Stream Deck (SDK v3, Node.js 24)
Le dossier `integrations/streamdeck` renferme le plugin officiel Elgato Stream Deck, développé sous le SDK v3 et exécuté par le runtime Node.js 24 :
- **Découverte Automatique de Port** : Le plugin lit le fichier `%LOCALAPPDATA%/Open-Delay/runtime.json` au démarrage pour identifier l'adresse IP et le port dynamique du service, puis s'authentifie via `/api/v1/session`.
- **Actions Disponibles** :
  1. `Set Delay` : Active le délai spécifié ou revient au direct si la consigne est à 0.
  2. `Go Live` : Annule immédiatement toute préparation et force le direct.
  3. `Toggle` : Bascule entre le direct et le dernier délai mémorisé.
  4. `Status` : Affiche l'état courant de la route assignée.
- **Retour Visuel sur les Touches LCD** : Affiche les mentions dynamiques `"LIVE"`, `"{N}s"`, `"BUILD"`, `"DEG"`, `"REC"` ou `"OFF"`.

#### 2.4.2 Banc de Tests Synthétiques E2E FFmpeg/FFprobe
La fiabilité du moteur média d'Open-Delay fait l'objet d'un banc de test automatisé rigoureux (`tests/media_e2e.py`) :
- **Génération Média de Synthèse Déterministe** : Génération d'une mire vidéo 1080p60 avec compteur d'images incrusté et de deux signaux sonores synthétiques non périodiques (chirps).
- **Validation Paquet par Paquet** :
  - Calcul et vérification des sommes de contrôle SHA-256 sur les fragments de flux.
  - Validation de la stricte monotonie des horloges DTS (`dts_regressions == 0`).
  - Mesure des écarts de désynchronisation A/V aux points de coupure (`cut_offsets`).
  - Décodage intégral de la sortie avec FFprobe pour s'assurer de l'absence totale de macroblocs, d'artéfacts ou d'avertissements de décodeur.

---

## 3. Étude Comparative du Marché des Solutions Logicielles (R2)

Le marché des outils de gestion de délai et de sécurisation de diffusion pour streamers se compose de solutions logicielles dédiées, de suites de production broadcast et de fonctionnalités intégrées aux logiciels de streaming.

### 3.1 Solutions Logicielles Dédiées Spécialisées

#### 3.1.1 InstantDelay v3.0 (Lypningeuh)
InstantDelay est la solution commerciale la plus visible auprès des créateurs de contenu sous OBS et Streamlabs Desktop :
- **Architecture & Modèle Économique** : Application Windows propriétaire à code fermé (*closed-source*). Elle fonctionne selon un modèle freemium comprenant un essai gratuit de 30 jours, puis exige un abonnement récurrent de **5,99 € par mois** ou **49,99 € par an**. Ce modèle d'abonnement pour une simple fonction de temporisation locale suscite de vives critiques au sein de la communauté.
- **Modes de Fonctionnement** :
  - *Mode Instant* : Maintient en permanence un buffer glissant (15s à 600s) et permet de basculer instantanément dans le différé sans interruption de flux.
  - *Mode Overlay* : Le direct continue d'être émis pendant que le buffer se remplit, avec affichage d'un décompte sur un overlay web, puis le flux bascule dans le passé une fois la durée atteinte.
- **Intégrations** : Plugin Stream Deck officiel (v3.2.0 via WebSocket local), docks OBS personnalisés et vérification de la configuration d'encodage (GOP, keyframes, B-frames).
- **Faiblesses et Vulnérabilités Identifiées** :
  - *Instabilités historiques* : Fréquente apparition de l'erreur `Error 1000` au démarrage (conflits de ports et d'écoute réseau).
  - *Conditions de concurrence (Race Conditions)* : Des bascules rapides entre direct et délai provoquent des désynchronisations audio/vidéo ou des plantages du relais.
  - *Dépendance au Cloud* : Les interfaces de dock et d'overlay s'appuient sur des serveurs distants (`app.instant-delay.com`), rendant l'outil vulnérable aux pannes internet ou d'infrastructure de l'éditeur.
  - *Absence de vrai bouton "Dump"* : L'outil sait revenir au direct (*Skip to live*), mais ne propose pas d'effacer sélectivement les 15 dernières secondes du buffer tout en maintenant la diffusion différée active.

#### 3.1.2 LiveDelay (skullboypl)
Développé en Pologne par *Skullboy*, LiveDelay est une alternative open-source gratuite hébergée sur GitHub (`skullboypl/livedelay-obs`) :
- **Architecture** : Conçu comme un plugin natif pour OBS Studio avec une interface en C++ / Qt couplée à un moteur de traitement écrit en **Rust**.
- **Points Forts** :
  - Workflow broadcast structuré en trois étapes : *Arm* (remplissage du buffer en RAM), *Activate* (bascule sur la trame clé tamponnée), et *Cut* (retour au direct).
  - Dock Qt natif consommant très peu de ressources (0,1 % CPU et ~30 Mo de RAM pour 15s de buffer).
  - Fonction de démultiplexage Multi-RTMP local vers plusieurs plateformes simultanées.
- **Limitations Rédhibitoires** :
  - *Blocage de l'API Service d'OBS 31* : Les versions récentes d'OBS Studio interdisent aux plugins tiers d'ajouter des destinations de diffusion dynamiques dans les menus natifs. LiveDelay oblige donc à configurer manuellement un serveur personnalisé vers `rtmp://127.0.0.1:1935/live`.
  - *Incompatibilité avec Twitch Enhanced Broadcasting* : L'utilisation d'un serveur personnalisé désactive l'encodage client multi-résolutions de Twitch.
  - *Perte fatale du buffer en cas de crash d'OBS* : Étant intégré à l'espace mémoire d'OBS, si OBS plante ou se ferme, le contenu restant dans le buffer est définitivement perdu sans jamais être diffusé.

#### 3.1.3 InstantClone (Soulhackzlol / s1moscs)
Créé en réaction directe au passage d'InstantDelay sous abonnement payant, InstantClone est un proxy open-source écrit en Rust sous licence **GPL-3.0** :
- **Caractéristiques** : Empreinte mémoire minime, support des raccourcis clavier, API REST locale et contrôle par contrôleurs MIDI.
- **Faiblesses** : Interface utilisateur austère et minimale, absence de paquet d'installation signé pour Windows, et surtout **gestion du buffer 100 % en mémoire vive (RAM)** sans débordement sur disque dur. Pour des délais prolongés (ex: 10 à 15 minutes), l'empreinte RAM devient prohibitive.

#### 3.1.4 Streamdelay (Streamwall)
Issu du projet open-source *Streamwall* destiné aux régies vidéo distribuées :
- **Spécificités** : Fonctionne sur le protocole **SRT (Secure Reliable Transport)** en entrée avec réémission en SRT ou RTMP.
- **Innovation Censure** : Intègre une véritable API de censure permettant de basculer à la volée en état `isCensored: true`. Le moteur applique alors un filtre de pixelisation/flou sur la vidéo et remplace l'audio par un bip de censure sur le segment sélectionné.
- **Limites** : Outil strictement en ligne de commande (*CLI / headless*), sans interface graphique pour les streamers, nécessitant des compétences d'administration système.

#### 3.1.5 Plugins Internes OBS (`obs-broadcast-delay` / `obs-broadcast-censure`)
Développée par le contributeur *H0K0H*, cette suite de plugins implémente la temporisation au niveau des sources ou des scènes d'OBS :
- **Capacités** : Permet de différer une caméra ou un jeu individuellement, supporte la pagination du buffer sur disque dur pour des délais de plusieurs heures, et propose un dock avec curseur de navigation temporelle (*scrubbing*).
- **Défaut Majeur (Consommation GPU/VRAM)** : Ce plugin intercepte les trames vidéo **non compressées** dans la boucle de rendu d'OBS avant encodage matériel NVENC. En 1080p60 non compressé, une seconde de vidéo pèse entre 150 et 400 Mo en VRAM. Cela provoque une surcharge GPU colossale et des pertes massives de trames en jeu (*rendering lag*).

---

### 3.2 Suites Logicielles de Production Broadcast

#### 3.2.1 Fonctionnalité Native "Délai du flux" dans OBS Studio
OBS Studio propose une option de délai de diffusion (*Paramètres → Avancé → Délai du flux*). Cependant, cette implémentation souffre de quatre défauts rédhibitoires pour les créateurs :
1. **Verrouillage à l'allumage** : Le délai doit impérativement être configuré avant le lancement du stream. Il est strictement impossible de le modifier ou de l'enclencher à chaud.
2. **Arrêt d'urgence destructeur** : Si le streamer clique sur "Arrêter la diffusion", OBS propose soit de vider le buffer vers Twitch (diffusant la fuite que l'on voulait dissimuler), soit de couper brutalement la connexion (interrompant la session sans retour au direct).
3. **Absence totale de Dump et de Censure** : Impossible de purger un segment ou d'insérer un bip sonore.
4. **Stockage 100 % en mémoire vive** : Risque d'encombrement de la RAM sur les longues sessions.

#### 3.2.2 vMix (Studio, 4K, Pro, MAX)
Suite de référence pour les régies événementielles sur Windows :
- **Gestion du Délai** : Propose une entrée *Video Delay* et un système d'enregistrement en continu pour le ralenti (*vMix Replay*).
- **Workflow de Sécurité Complexe** : vMix ne possède pas de bouton Dump natif sur sa sortie de streaming. Les techniciens doivent configurer un rebouclage NDI local de la sortie master, y appliquer un filtre de délai de 30 secondes, et associer des macros complexes pour basculer sur un carton d'attente en cas d'urgence.
- **Inconvénients** : Coût financier prohibitif (de 700 $ à 1 200 $ en achat définitif, ou 50 $/mois), et consommation matérielle trop lourde pour un streamer opérant sur une seule machine.

#### 3.2.3 Telestream Wirecast
- Ne dispose d'aucun délai global sur la sortie master de diffusion. Les réglages de temporisation ne servent qu'à la synchronisation des sources d'entrée. Coût de 599 $ à 799 $.

#### 3.2.4 TriCaster (Vizrt / NewTek — Éditions Logicielles)
- Exploite la technologie historique du **Time Slip sur fichiers ouverts (*Growing Files*)** via le module *IsoCorder* et un lecteur DDR. Solution professionnelle coûtant de 5 000 € à plus de 30 000 €, totalement hors de portée d'un créateur indépendant.

---

### 3.3 Analyse Comparative du Matériel Broadcast Traditionnel

#### 3.3.1 Les Références Broadcast : Eventide BD600 / BD600W+ et Evertz SDI
Dans l'industrie de la radio et de la télédiffusion, la gestion des imprévus du direct est assurée depuis des décennies par des équipements matériels dédiés, dont le légendaire processeur **Eventide BD600** (et sa version réseau BD600W+) :
- **Spécifications** : Châssis rack 19" 1U, entrées/sorties audio XLR analogiques symétriques (+26 dBu) et numériques AES/EBU, mémoire tampon de 80 secondes à 24-bit 48 kHz, et ports GPIO pour boutons poussoirs d'antenne. Coût public : **3 999 $ à 5 400 $ USD**.
- **Inadéquation avec le Streaming Web** : Ces appareils traitent des signaux bruts en bande de base (SDI/AES). Ils ignorent totalement le RTMP, le H.264 et les architectures logicielles PC.

#### 3.3.2 Transposition des Concepts Clés : Dump, Cough, Panic et Squeeze
Bien que le matériel soit inadapté, l'ingénierie broadcast définit les concepts fonctionnels essentiels à transposer dans Open-Delay :
- **Le Bouton "Dump" (Éjection)** : Efface une durée définie (ex: 8 à 15 secondes) du buffer tout en poursuivant la diffusion continue du reste de l'historique sans rupture de flux.
- **Le Bouton "Cough / Sneeze" (Toux)** : Coupe temporairement l'entrée audio/vidéo sans décaler le buffer, idéal pour tousser ou s'adresser discrètement à son modérateur.
- **Le Mode "Panic"** : Si le tampon arrive à épuisement après plusieurs purges successives, le système commute automatiquement sur une boucle de secours (jingle, carton d'attente) pendant la reconstitution du buffer.
- **L'Algorithme "Squeeze / Catch-Up"** : Accélère imperceptiblement la lecture (+5 % à +10 % sans altération de la hauteur de voix) pour reconstituer progressivement le délai de sécurité après une purge sans saut brutal.

#### 3.3.3 Le Stream Deck : Le Pupitre Broadcast Physique Universel
Le boîtier physique **Elgato Stream Deck** constitue la transposition moderne parfaite des consoles broadcast. Grâce à ses touches LCD dynamiques et ses encodeurs rotatifs (Stream Deck +), le streamer peut déclencher des purges d'urgence ou surveiller l'état de son délai sans quitter son jeu des yeux ni effectuer de `Alt-Tab`.

---

### 3.4 Cas d'Usage Réels et Scénarios Opérationnels

#### 3.4.1 Anti-Stream Sniping (Esport, Battle Royale et Poker en Ligne)
- **Scénario** : Durant une compétition sur *Valorant*, le streamer active un délai de 120 secondes. Dès la partie terminée, il active le retour au direct (0s) pour interagir avec son chat et féliciter ses coéquipiers.

#### 3.4.2 Sécurité Physique, Vie Privée et Anti-Doxxing / Anti-Swatting (IRL)
- **Scénario** : En marchant dans la rue, un individu agressif interpelle le streamer ou tente de divulguer son identité. Le streamer appuie sur le bouton "Dump 30s" sur sa pédale Stream Deck, effaçant le passage avant qu'il ne parvienne aux spectateurs, et s'éloigne en toute sécurité.

#### 3.4.3 Modération et Conformité TOS/DMCA (Trolls Vocaux et Musique)
- **Scénario** : Dans un salon de jeu en ligne, un joueur crie une insulte bannie par Twitch. L'opérateur presse "Bleep Last 5s", insérant un signal de censure sur l'audio sans couper le flux vidéo ni interrompre la partie.

---

### 3.5 Grille d'Évaluation Multicritères Exhaustive (10 Solutions x 10 Critères)

| # | Solution Analysée | Type Architectural | Protocoles Supportés | Fonction Dump / Panic | Gestion Mémoire / Disque | Intégration OBS | Support Stream Deck | Modèle Économique & Prix | Atout Majeur | Limite Critique |
|---|---|---|---|---|---|---|---|---|---|---|
| **1** | **Open-Delay** *(v0.2.2)* | Application autonome (Tauri / Rust) + Proxy RTMP | RTMP/RTMPS (H.264 / AAC) | Mode Instant (0s) + Replay (Dump partiel en roadmap) | Disque circulaire borné (2 Gio) + Cache RAM (64 Mo) | Dock personnalisé responsive + Overlay navigateur | Plugin officiel SDK v3 (HTTP local) | Gratuit (*Source-Available*) | Stockage hybride ultra-robuste, gratuit, ports dynamiques | Pas d'Enhanced RTMP ni d'audio multi-pistes dans la v0.2 |
| **2** | **InstantDelay** *(v3.0.0)* | Application Windows fermée + Proxy RTMP | RTMP multi-destinations | Retour direct (0s), pas de vrai Dump partiel | Tampon mémoire vive (RAM) principal | Dock OBS web + Overlay navigateur | Plugin officiel (WebSocket local) | Freemium payant (Essai 30j puis ~5,99 €/m) | Finition graphique soignée, détection de mauvaise config OBS | Abonnement mensuel impopulaire, code fermé, dépendance cloud |
| **3** | **LiveDelay** *(skullboypl)* | Plugin OBS natif (C++/Qt) + Cœur Rust | RTMP multi-destinations | Séquence *Arm / Activate / Cut* | Tampon RAM par destination | Panneau Qt natif dans OBS | Raccourcis et profils Stream Deck | Open-source gratuit (GitHub) | Dock Qt ultra-léger (0,1% CPU), démultiplexage local | Incompatible Twitch Enhanced Broadcasting, perte buffer si OBS plante |
| **4** | **InstantClone** *(Soulhackzlol)* | Proxy autonome en Rust | RTMP | *Arm / Activate / Cut* | Tampon 100% mémoire vive (RAM) | Dock et overlay web très sommaires | Raccourcis clavier et MIDI | Open-source gratuit (GPL-3.0) | Multi-destination local, réemballage audio | Interface austère, pas de pagination disque, pas d'installateur signé |
| **5** | **Streamdelay** *(Streamwall)* | Serveur proxy autonome | SRT en entrée, SRT / RTMP en sortie | Vrai bouton Dump via API (`isCensored: true`) | Tampon mémoire configurable | Aucune intégration graphique | Via requêtes HTTP ou WebHooks | Open-source gratuit (GitHub) | Censure chirurgicale (floutage vidéo + bip audio), résilience SRT | Absence totale d'interface graphique (CLI pur), réservé aux techniciens |
| **6** | **OBS Studio Natif** | Fonction interne dans OBS | RTMP, SRT, HLS, WHIP | Aucun (Arrêt d'urgence destructeur) | Buffer FIFO 100% RAM | Intégré dans OBS (Paramètres Avancés) | Raccourci standard Démarrer/Arrêter | Open-source gratuit (GPL-2.0) | Présent d'office sur tous les postes de streaming | Impossible à modifier à chaud, purge dangereuse du buffer à l'arrêt |
| **7** | **obs-broadcast-delay** *(H0K0H)*| Plugin filtre de source/scène OBS | Graphe de rendu interne OBS | Censure manuelle ou IA (NSFW/Audio) | Hybride configurable (RAM, VRAM, Disque) | Dock interactif "Diffusion" | Via raccourcis natifs OBS | Open-source gratuit (GitHub) | Stockage disque illimité, contrôle fin type magnétoscope | Consommation VRAM/GPU massive (vidéo brute non compressée) |
| **8** | **vMix** *(Studio, 4K, Pro)* | Suite de production broadcast PC | NDI, RTMP, SRT, SDI | Contournement par Macro + Carton d'attente | Buffer RAM et disque vidéo dédié | Rebouclage NDI / Entrée externe | Plugin officiel et raccourcis Stream Deck | Commercial propriétaire (700 $ à 1 200 $ ou 50 $/m) | Écosystème broadcast ultra-complet, ralenti multi-caméras | Pas de dump natif en sortie de stream, coût prohibitif |
| **9** | **Wirecast** *(Telestream)* | Suite de production logicielle | RTMP, SRT, NDI | Aucun sur le master de diffusion | Tampon de synchronisation des entrées | Aucune | Raccourcis matériels | Commercial propriétaire (599 $ à 799 $) | Interface de production multi-caméras propre | Aucun délai global de sécurité avec purge, très cher |
| **10**| **Eventide BD600 / BD600W+** | Processeur matériel broadcast 1U | Signaux physiques (AES/EBU, XLR, SDI) | Vrai Dump (1/2s à 10s), Sneeze, Panic, Squeeze | Mémoire flash / DSP temps réel dédié | Aucune (Matériel physique indépendant) | Pupitre broadcast physique ou GPIO | Matériel propriétaire (3 999 $ à 5 400 $ USD) | Fiabilité absolue à 100 %, algorithme Squeeze/Catch-up | Coût astronomique, incompatible avec les flux réseau RTMP des PC |

---

## 4. Diagnostic Critique des Faiblesses, Manques et Anomalies d'Open-Delay (R3)

L'audit approfondi du code source révèle plusieurs limitations techniques bloquantes, vulnérabilités et anomalies de conception qui brident actuellement l'expérience utilisateur et la sécurité opérationnelle d'Open-Delay.

### 4.1 Rejet Formel d'Enhanced RTMP (AV1, HEVC/H.265 et HDR)
L'évolution majeure du streaming moderne est l'avènement de la norme **Enhanced RTMP (v1 et v2)**, développée par l'alliance Veovera et intégrée en standard dans OBS Studio depuis les versions 29 et 30.
Or, dans le code de classification des paquets d'Open-Delay (`crates/service/src/engine.rs:577-593`), le moteur impose :
```rust
// crates/service/src/engine.rs:580-585
rtmp::VIDEO => {
    if message.payload.len() < 5
        || message.payload[0] & 0x80 != 0
        || message.payload[0] & 0x0f != 7
    {
        return Err(Fault("Only H.264 AVC video is supported by this build"));
    }
```
- **Ligne 581** : `message.payload[0] & 0x80 != 0` vérifie le bit de poids fort `IsExHeader`. Tout paquet Enhanced RTMP est **immédiatement rejeté avec une erreur fatale**.
- **Ligne 582** : `message.payload[0] & 0x0f != 7` impose le codec ID 7 (H.264/AVC).
- **Conséquence critique** : Il est strictement impossible d'utiliser Open-Delay avec les codecs vidéo de nouvelle génération (**AV1** et **HEVC/H.265**), pourtant indispensables pour diffuser en 1440p ou 4K sur YouTube Live à bas débit, ou pour exploiter le HDR 10 bits.

---

### 4.2 Rejet de l'Audio Multi-Piste (Twitch VOD Track & Protection DMCA)
Pour se prémunir des suspensions de chaîne pour violation de droits d'auteur musical (DMCA), les créateurs configurent quasi-systématiquement deux pistes audio dans OBS :
- **Piste 1 (Live)** : Microphone + Jeu + Musique.
- **Piste 2 (Twitch VOD Track)** : Microphone + Jeu (sans musique), enregistrée sur les serveurs de Twitch pour le replay.

Dans Open-Delay :
1. `crates/service/src/engine.rs:594-601` limite strictement l'audio au format AAC simple :
   ```rust
   // crates/service/src/engine.rs:596-598
   if message.payload.len() < 2 || message.payload[0] >> 4 != 10 {
       return Err(Fault("Only AAC audio is supported by this build"));
   }
   ```
2. `crates/service/src/engine.rs:400` bloque tout flux additionnel :
   ```rust
   // crates/service/src/engine.rs:400-402
   if message.stream_id != stream_id {
       return Err(Fault("Multiple message streams per route are unsupported"));
   }
   ```
- **Conséquence critique** : Open-Delay détruit la configuration multi-piste Twitch VOD. Tout streamer insérant Open-Delay sur sa chaîne s'expose directement à des réclamations DMCA et des coupures de son sur ses replays Twitch.

---

### 4.3 Vulnérabilité de Purge du Buffer lors d'une Micro-Coupure OBS
Que survient-il en cas de micro-déconnexion réseau entre OBS et Open-Delay (par exemple un freeze d'une seconde de l'encodeur matériel ou une relance rapide) ?
Dans `crates/service/src/engine.rs:446-448` :
```rust
// crates/service/src/engine.rs:446-448
if let Ok(mut history) = service.history.lock() {
    history.remove_route(&route.id, generation);
}
```
- **Vulnérabilité critique démontrée** : Dès qu'OBS se déconnecte, `remove_route` purge immédiatement et intégralement tout le buffer accumulé pour cette route.
- Lors de la reconnexion automatique d'OBS 2 secondes plus tard, Open-Delay incrémente la génération de session (`generation += 1`) et recommence avec **un historique strictement vide (délai effectif ramené à 0s)**.
- **Le streamer perd toute sa protection de délai et se retrouve diffusé en direct à son insu !**

---

### 4.4 Vulnérabilité Temporelle lors du "Go Live" (Fuite d'Images en Attente de Keyframe)
Examinons la logique de traitement du retour au direct en cas d'urgence (`Action::GoLive`) :
1. `crates/service/src/engine.rs:1051-1055` positionne l'état en `Pending::Live`.
2. Dans `crates/service/src/engine.rs:1416-1418` :
   ```rust
   // crates/service/src/engine.rs:1416-1418
   // Continue the current segment while waiting for a future live keyframe.
   if !matches!(pending, Pending::Live { .. }) {
       return Ok((None, false));
   }
   ```
- **Faille de sécurité majeure démontrée** :
  Tant que la trame clé (*IDR Keyframe*) suivante n'est pas reçue d'OBS (ce qui prend entre **1 et 2 secondes** selon l'intervalle de GOP configuré dans OBS), **le moteur continue de lire et d'émettre les paquets retardés du buffer** !
- Si le streamer appuie sur "Go Live" pour purger une bévue survenue il y a 2 secondes alors qu'il utilisait un délai court, **la bévue continue d'être émise sur Twitch pendant le temps d'attente de la trame clé** avant que le saut ne s'opère. L'absence d'un gel d'écran ou d'un écran noir d'urgence immédiat crée une fenêtre temporelle de fuite.

---

### 4.5 Limite des 10 Minutes et Saturation I/O Synchrone du Disque
- **Plafond rigide de 10 minutes** :
  Dans `crates/core/src/config.rs:4`, le délai maximal est borné :
  ```rust
  // crates/core/src/config.rs:4
  pub const MAX_DELAY_MS: u64 = 600_000;
  ```
  Cette limitation artificielle bloque l'adoption d'Open-Delay pour les tournois majeurs d'esport (CS2, Valorant, Rainbow Six, tournois de poker) qui exigent fréquemment **15 à 30 minutes de décalage**.
- **Opérations I/O Synchrones Bloquantes** :
  Dans `crates/core/src/history.rs:146-147` et `187-188`, les opérations de lecture et d'écriture disque utilisent `std::fs::File` de manière bloquante sous un mutex :
  ```rust
  file.seek(SeekFrom::Start(self.cursor))?;
  file.write_all(payload)?;
  ```
  Le cache RAM de 64 Mo est saturé en moins de 58 secondes sur un flux à 8.5 Mbps. Dès que le délai dépasse 1 minute, 100 % de la lecture est déportée sur le disque. Sur des disques durs lents ou des SSD fortement sollicités par un jeu vidéo moderne, ces appels I/O synchrones bloquent le thread de travail Tokio pendant plusieurs dizaines de millisecondes, induisant des saccades (*stuttering*) et des micro-pertes de paquets sur le flux sortant.

---

### 4.6 Perte de Contrôle du Dock OBS sans Focus de Fenêtre
Dans `ui/src/App.tsx:164`, l'application documente elle-même une limite structurelle de son intégration dans OBS :
> *"Les raccourcis du navigateur fonctionnent seulement lorsque le dock a le focus."*
- Lorsqu'un streamer joue à un jeu vidéo en plein écran, le dock OBS n'a jamais le focus actif de la souris ou du clavier. Les raccourcis web intégrés au dock sont totalement inopérants pendant la partie.

---

### 4.7 Polling HTTP du Plugin Stream Deck et Absence d'Éléments Dynamiques
Dans `integrations/streamdeck/src/plugin.mjs:73` :
- Le plugin interroge le backend par un appel HTTP `fetch` répété toutes les 500 millisecondes (`setTimeout(poll, 500)`).
- Il n'utilise pas le flux d'événements temps réel SSE (`/api/v1/events`), générant une latence de mise à jour visuelle de 250 à 500 ms sur les touches LCD.
- Le plugin n'exploite pas les encodeurs rotatifs du **Stream Deck +** ni son écran tactile pour afficher une jauge de remplissage dynamique.
- Le bouton Go Live / Panic se déclenche sur un simple appui fugace, sans protection contre les fausses manipulations (*missclicks*).

---

## 5. Axes d'Amélioration des Fonctionnalités Existantes (R4)

### 5.1 Perfectionnement de la Gestion du Délai (Montée et Descente Douces)
Actuellement, le passage au délai se fait par saut brutal (*Instant*) ou par attente visualisée (*Overlay*).
- **Mode Montée Douce Continue (*Smooth Ramp-Up*)** :
  Ralentissement imperceptible de la cadence de lecture du flux de sortie (ex: diffusion à 0.95x de la vitesse réelle). Le délai s'accumule de manière invisible pour les spectateurs, sans coupure et sans nécessiter d'overlay public si le streamer souhaite cacher la présence d'un délai anti-sniping à ses adversaires.
- **Élévation du Plafond de Délai** :
  Porter `MAX_DELAY_MS` de 10 minutes à **30 minutes**, voire **60 minutes**, avec adaptation dynamique du budget d'allocation disque.

---

### 5.2 Perfectionnement de l'Éditeur d'Overlay (Catalogue de Thèmes et CSS Moderne)
L'éditeur actuel (`ui/src/components/OverlayEditor.tsx`) gagne à être modernisé pour offrir une valeur esthétique broadcast :
- **Catalogue de Thèmes Graphiques Prédéfinis** :
  - *Esport Modern* : Typographie bold, anneau de progression SVG circulaire, indicateurs d'état colorés.
  - *Minimalist Dark* : Badge compact semi-transparent avec effet de flou d'arrière-plan (*backdrop-filter: blur*).
  - *Broadcast TV* : Bandeau inférieur complet avec affichage de l'heure du direct et logo personnalisable.
- **Animations de Transition Fluides** : Remplacement des apparitions franches par des fondus animés et des micro-pulsations dynamiques lors de la phase de remplissage du buffer.

---

### 5.3 Intégration OBS Perfectionnée via OBS-WebSocket v5
Le dock OBS actuel étant passif, l'intégration de la bibliothèque officielle **OBS-WebSocket v5** (intégrée en natif dans OBS depuis la v28) débloque une automatisation complète :

```
┌─────────────────────────────────────────────────────────────┐
│                 OBS STUDIO (HÔTE DE DIFFUSION)              │
│  ┌───────────────────────┐       ┌───────────────────────┐  │
│  │   OBS-WebSocket v5    │◄─────►│    Dock Open-Delay    │  │
│  │     (Port 4455)       │       │    (UI Bidirectionnelle│ │
│  └───────────┬───────────┘       └───────────────────────┘  │
│              │                                              │
│              ▼                                              │
│   Gestionnaire de Scènes OBS                                │
│   (Bascule automatique de scène lors d'un Panic Dump)       │
└──────────────┼──────────────────────────────────────────────┘
               │ Flux RTMP local
               ▼
┌─────────────────────────────────────────────────────────────┐
│                 MOTEUR OPEN-DELAY (RUST)                    │
│   Détection automatique Start/Stop & Auto-configuration URL │
└─────────────────────────────────────────────────────────────┘
```

**Bénéfices majeurs** :
1. **Zéro Configuration Manuelle de Port** : Open-Delay communique via WebSocket avec OBS pour mettre à jour automatiquement l'URL du serveur personnalisé avec le port dynamique local attribué.
2. **Synchronisation Automatique de Scène lors d'un Panic Dump** : Lors d'un Dump d'urgence, Open-Delay commande instantanément à OBS de basculer sur la scène *"Pause / Incident Technique"*.
3. **Asservissement du Cycle de Vie** : Open-Delay démarre et arrête automatiquement ses services dès qu'OBS commence ou interrompt la diffusion.

---

### 5.4 Modernisation Réactive du Plugin Stream Deck
- **Architecture Événementielle Réactive SSE** :
  Remplacement du polling HTTP 500 ms par la connexion directe au flux Server-Sent Events (`/api/v1/events`), abaissant la latence d'affichage sous les **10 millisecondes**.
- **Support Natif du Stream Deck +** :
  - Utilisation de la molette crantée (*Dial*) pour incrémenter ou décrémenter le délai (+5s / -5s par cran).
  - Affichage d'une jauge de niveau de buffer graphique sur l'écran tactile LCD.
- **Sécurisation Anti-Fausse Manipulation sur le Panic Button** :
  - Option **"Hold to Dump"** : Exiger un maintien de la touche pendant 1.5 seconde avec barre de progression circulaire sur la touche avant déclenchement.
  - Option **"Double Press to Dump"** : Exiger une double-pression rapide sous 400 ms.

---

## 6. Feuille de Route des Nouvelles Fonctionnalités Recommandées (R5)

### 6.1 Mode Censure Audio Ciblée ("Bleep Censor" Sélectif)
- **Problématique résolue** : Lorsqu'un coéquipier prononce une insulte bannie par Twitch sur Discord, le streamer est actuellement forcé de couper tout son live ou de déclencher un Dump visuel brutal.
- **Fonctionnement technique** :
  - L'opérateur appuie sur une touche raccourcie **"Bleep Last 5s"**.
  - Le moteur Open-Delay localise les 5 dernières secondes écoulées dans le buffer.
  - **La vidéo reste parfaitement intacte**.
  - Sur la piste audio, les paquets AAC correspondants voient leurs échantillons remplacés soit par un silence absolu, soit par une tonalité sinusoïdale standard de 1 000 Hz.
  - Le stream continue sans aucune interruption visuelle pour les spectateurs, mais le propos répréhensible est neutralisé.

---

### 6.2 Mode "Catch-Up" / Rattrapage Temporel Automatique sans Distorsion de Pitch (WSOLA)
- **Problématique résolue** : Après la fin d'une manche compétitive (délai de 3 minutes actif), le streamer souhaite revenir au direct pour échanger avec son audience sans tronquer brutalement la fin de son action.
- **Fonctionnement technique** :
  - L'activation du mode **"Catch-Up"** accélère la lecture média de 5 % à 15 % (facteur 1.05x à 1.15x).
  - **Algorithme audio WSOLA (*Waveform Similarity Overlap-Add*)** : L'échantillonnage temporel de l'audio est comprimé sans altérer la hauteur tonale (la voix du streamer ne subit aucun effet d'accélération aiguë de type "voix de dessin animé").
  - Le flux résorbe le retard accumulé en quelques minutes et recolle au direct de manière totalement imperceptible pour l'audience.

---

### 6.3 Boucle Vidéo et Écran de Secours d'Urgence au Dump ("Safety Slate")
- **Problématique résolue** : Lors d'un incident critique (fuite d'écran avec mot de passe affiché), le retour immédiat au direct peut paniquer le streamer qui est encore en train de fermer ses fenêtres sensibles.
- **Fonctionnement technique** :
  - Lors d'un déclenchement de Panic Dump, le flux ne saute pas vers le direct en cours mais commute instantanément sur une **boucle vidéo d'urgence pré-encodée** (animation de transition, carton d'attente "Nous revenons dans un instant").
  - Le créateur bénéficie d'un répit configurable (15 à 30 secondes) pour sécuriser son bureau avant que le flux direct ne soit rétabli.

---

### 6.4 Prise en Charge Native Enhanced RTMP v1/v2 & Codecs Next-Gen
- **Spécifications** :
  - Reconnaissance du flag `IsExHeader` (`0x80`) dans le premier octet vidéo.
  - Support des codecs vidéo modernes via FourCC : **AV1** (`av01`), **HEVC/H.265** (`hvc1`), et **HDR** (Rec. 2020 10-bit).
  - Support de l'audio multi-pistes : ingestion et retransmission séparée de la **Piste 1 (Master)** et de la **Piste 2 (Twitch VOD Track)**, neutralisant le risque de suspension de chaîne DMCA.

---

### 6.5 Relais Hybride SRT (Secure Reliable Transport) & WebRTC / WHIP
- **Spécifications** :
  - Ingestion de flux SRT entrants en provenance d'OBS ou de régies nomades (streaming IRL).
  - Tolérance aux pertes de paquets réseau et élimination du phénomène de blocage de tête de ligne (*Head-of-Line Blocking*) inhérent à TCP.
  - Option d'émission ultra-faible latence via le protocole WHIP.

---

### 6.6 Module d'IA Locale "Leak Shield" (Détection Vocale et Visuelle Automatisée)
- **Fonctionnement 100 % Local (Confidentialité et Souveraineté Totales)** :
  1. **Analyseur Audio Temps Réel (Whisper Tiny INT8 / Silero VAD)** :
     - Écoute du microphone local sur le processeur (CPU) ou les cœurs Tensor.
     - Déclenchement automatique du "Bleep Censor" dès qu'un mot figurant sur la liste noire personnalisée du streamer (adresse postale personnelle, nom de famille, propos toxiques ToS) est détecté.
  2. **Analyseur Visuel d'Écrans Sensibles (OCR / Vision Légère)** :
     - Détection des signatures graphiques de fenêtres critiques (client Discord avec messages privés, fenêtres de configuration Windows, champs de formulaires bancaires ou de mots de passe).
     - Déclenchement préventif d'un masquage ou d'un saut de délai.

---

### 6.7 Profils de Configuration par Jeu / Plateforme
Création d'un sélecteur de profils complets en un clic dans l'interface :
- **Profil "Esport Compétitif"** : Délai de 3 minutes, mode Instant, overlay minimaliste, OBS-WebSocket lié à la scène "Pause Tactique".
- **Profil "Streaming IRL"** : Entrée protocolaire SRT, délai de 45 secondes, mode Bleep Censor actif, bouton Panic avec boucle de secours.
- **Profil "Tournoi de Poker"** : Délai de 15 minutes, capacité disque de 8 Gio préallouée, contrôle d'intégrité de flux strict.

---

### 6.8 Matrice de Priorisation de la Feuille de Route (P0, P1, P2, P3)

| Réf. | Intitulé de la Fonctionnalité | Catégorie Architecturale | Complexité Technique | Valeur Streamer | Priorité |
|---|---|---|---|---|---|
| **F-01** | Support **Enhanced RTMP (AV1, HEVC/H.265 & HDR)** | Codecs & Protocoles | Moyenne (3-4 j) | **Critique (YouTube Live & 4K)** | **P0 (Immédiate)** |
| **F-02** | Support **Audio Multi-Piste (Twitch VOD Track)** | Audio & DMCA | Moyenne (3 j) | **Critique (Anti-Ban Twitch)** | **P0 (Immédiate)** |
| **F-03** | **Persistance de Session lors d'une Micro-Coupure OBS** | Résilience Média | Faible (1-2 j) | **Vitale (Sécurité Anti-Fuite)** | **P0 (Immédiate)** |
| **F-04** | **Coupure d'Urgence Instantanée lors du Dump (Zero-Leak)** | Résilience Média | Faible (1 j) | **Vitale (Sécurité Anti-Fuite)** | **P0 (Immédiate)** |
| **F-05** | Intégration **OBS-WebSocket v5** (Sync Scènes & Auto-Port) | Intégrations & UX | Moyenne (3-4 j) | **Très Haute (Zéro-Clic)** | **P1 (Majeure)** |
| **F-06** | Extension du Délai Maximal à 30 Minutes | Gestion Buffer | Très Faible (0.5 j) | **Haute (Esport Officiel)** | **P1 (Majeure)** |
| **F-07** | Modernisation **Stream Deck** (SSE, Molettes LCD, Hold-to-Panic)| Intégrations | Moyenne (2-3 j) | **Très Haute (Ergonomie)** | **P1 (Majeure)** |
| **F-08** | Mode **Censure Audio Ciblée ("Bleep Censor")** | Innovation Fonctionnelle | Moyenne-Élevée (4-5 j)| **Rupture Marché** | **P1 (Majeure)** |
| **F-09** | Mode **"Catch-Up" / Rattrapage Audio WSOLA** | Traitement Signal | Élevée (5-6 j) | **Très Haute (Confort Live)** | **P2 (Secondaire)** |
| **F-10** | **Boucle Vidéo et Écran de Secours au Dump** | Innovation Fonctionnelle | Moyenne (3 j) | **Haute (Professionnalisme)** | **P2 (Secondaire)** |
| **F-11** | Prise en Charge des Protocoles **SRT & WebRTC / WHIP** | Réseau & Protocoles | Élevée (6-8 j) | **Haute (Streaming IRL)** | **P2 (Secondaire)** |
| **F-12** | Détection Automatisée par **IA Locale ("Leak Shield")** | Innovation / IA Locale | Élevée (8-10 j) | **Différenciateur Unique** | **P3 (Futur)** |

---

## 7. Synthèse et Recommandations Opérationnelles pour Open-Delay

L'audit technique approfondi et l'analyse comparative du marché confirment que le projet **Open-Delay possède les fondations architecturales les plus solides et saines de l'écosystème** :
1. **Un Moteur Rust performant et souverain**, affranchi des lourdeurs de runtime Node.js client et des licences commerciales récurrentes.
2. **Un système de stockage hybride exemplaire** (anneau circulaire sur disque préalloué de 2 Gio couplé à un cache LRU en mémoire vive de 64 Mo), garantissant une protection absolue contre les dépassements d'espace disque.
3. **Une intégration de bureau moderne sous Tauri v2 et React 19**, associant réactivité, sécurité défensive (CSP stricte, tokens de session cryptographiques) et docks OBS soignés.

### Les Recommandations Opérationnelles Immédiates

Pour s'imposer définitivement comme la référence mondiale du délai de stream et surclasser InstantDelay, l'équipe de développement doit concentrer ses efforts sur les chantiers suivants :

1. **Déverrouiller immédiatement les codecs modernes (Priorité P0)** :
   - Modifier la méthode `classify` dans `crates/service/src/engine.rs` pour reconnaître le flag `IsExHeader` d'Enhanced RTMP et intégrer les FourCC `av01` (AV1) et `hvc1` (HEVC).
   - Prendre en charge l'audio multi-pistes pour préserver la piste Twitch VOD Track indispensable à la protection contre les strikes DMCA.
2. **Éliminer les deux vulnérabilités de sécurité critiques (Priorité P0)** :
   - Corriger la purge brutale du buffer dans `crates/service/src/engine.rs:446` lors des micro-déconnexions d'OBS, afin de préserver l'historique et la sécurité du créateur lors d'une reconnexion d'encodeur.
   - Corriger la fuite d'images lors de l'action `GoLive` (`engine.rs:1416`) en instaurant un gel d'image ou un écran noir immédiat pendant l'attente de la trame clé live.
3. **Transposer les innovations broadcast indispensables (Priorité P1)** :
   - Introduire le mode **Censure Audio Ciblée ("Bleep Censor")** permettant de biper ou couper les 5 dernières secondes sonores sans affecter le flux vidéo.
   - Développer le mode **"Catch-Up"** basé sur l'algorithme WSOLA pour recoller au direct sans coupure brutale.
   - Connecter le dock à OBS via **OBS-WebSocket v5** pour offrir une expérience zéro-clic et une synchronisation automatique de scènes d'urgence lors des dumps.
4. **Sublimer le pilotage physique Stream Deck (Priorité P1)** :
   - Remplacer le polling HTTP par les événements temps réel SSE et exploiter les encodeurs rotatifs du Stream Deck + pour un contrôle analogique du délai.

Grâce à cette feuille de route stratégique, Open-Delay ne sera plus seulement une alternative gratuite à InstantDelay : il deviendra **le standard absolu et incontournable de la sécurisation de diffusion en direct pour l'ensemble des créateurs de contenu et des structures esportives**.
