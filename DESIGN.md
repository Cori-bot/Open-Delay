---
name: Open-Delay Signal
description: A calm local control room for live streaming delay.
colors:
  background: "#101011"
  surface: "#181819"
  raised: "#222224"
  border: "#38383b"
  text: "#f5f5f3"
  muted: "#b5b5b9"
  accent: "#f1f1ee"
  warning: "#e9c17d"
  danger: "#ffaaa1"
  live: "#9bd4ac"
typography:
  headline:
    fontFamily: "Geist Variable, Geist, system-ui, sans-serif"
    fontSize: "clamp(25px, 2.5vw, 31px)"
    fontWeight: 650
    lineHeight: 1.16
    letterSpacing: "-0.035em"
  title:
    fontFamily: "Geist Variable, Geist, system-ui, sans-serif"
    fontSize: "16px"
    fontWeight: 650
  body:
    fontFamily: "Geist Variable, Geist, system-ui, sans-serif"
    fontSize: "14px"
rounded:
  control: "8px"
  card: "12px"
  stage: "14px"
spacing:
  compact: "8px"
  standard: "16px"
  section: "24px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "#171718"
    rounded: "{rounded.control}"
    height: "42px"
  button-secondary:
    backgroundColor: "{colors.raised}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    height: "42px"
  card:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.card}"
---

## Overview

**Creative North Star: Signal.** Une régie personnelle pour choisir où se trouve le direct dans le temps. L’état effectif se lit en premier ; les commandes restent accessibles au clavier. Le signe Open-Delay à trois barres inclinées est partagé avec Windows et Stream Deck ; les marques des services proviennent de fichiers locaux documentés dans `docs/BRAND-ASSETS.md`.

**Key Characteristics:**
- Surfaces sombres neutres, texte blanc cassé et couleurs discrètes réservées aux états.
- Repère d’historique fondé sur les mesures réelles du moteur.
- États, erreurs et récupération lisibles avec du texte, sans dépendre de la couleur.

## Colors

Les fonds gris presque noirs, les surfaces neutres légèrement relevées et les traits discrets composent l’interface. Le blanc cassé marque les actions et la sélection ; le vert, l’ambre et le rouge sont réservés aux états, toujours accompagnés d’un libellé explicite. Les couleurs authentiques des plateformes et celles choisies pour les overlays restent distinctes.

## Typography

Geist Variable sert les titres et l’interface. Le corps est de 14 px et les titres de page s’adaptent entre 25 et 31 px. Les valeurs mesurées utilisent des chiffres tabulaires.

## Layout

Navigation stable, régie principale et cartes de destinations. La réglette combine l’historique et le délai réellement remontés par le moteur ; elle masque sa position lorsque les sorties ont des délais différents. À 720 px et moins, la navigation et les commandes s’adaptent ; les formulaires secondaires s’empilent à 768 ou 600 px selon leur structure.

## Elevation & Depth

La hiérarchie repose surtout sur les niveaux de surface et des bordures. Les notifications utilisent une ombre diffuse pour rester lisibles au-dessus du contenu.

## Shapes

Panneaux aux coins arrondis, champs et commandes plus compacts. Les repères de statut sont complétés par du texte.

## Components

Le panneau de contrôle expose l’état de sortie, le délai mesuré, la consigne, l’historique disponible et les commandes de direct. Les pages secondaires utilisent des panneaux, champs et dialogues cohérents. Base UI fournit dialogues et menus, Lucide les icônes d’action et Sonner les confirmations ponctuelles. Les transitions respectent `prefers-reduced-motion`.

## Do's and Don'ts

- Do distinguer clairement consigne et délai effectif.
- Do laisser les erreurs et la récupération visibles hors des notifications temporaires.
- Do préserver les logos authentiques et la provenance des marques.
- Don't inventer d’activité, de connexion ou de mesure média.
- Don't masquer le retour au direct pendant la préparation.
- Don't journaliser les clés de stream ou les placer dans les adresses de contrôle.
