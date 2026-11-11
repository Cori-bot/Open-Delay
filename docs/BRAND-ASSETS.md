# Brand assets

`ui/public/brand/open-delay-mark.svg` is the canonical Open-Delay mark. Its three rounded bars reproduce the original in-app CSS dimensions, rotations and opacity. `BrandMark` uses the same SVG as an alpha mask, keeping it independent of text color; `Wordmark` supplies the accessible name.

The Windows PNG/ICO and Stream Deck marketplace PNGs are generated from this SVG. Run `npm ci --prefix scripts` once, then `node scripts/brand-assets.mjs` after editing the mark. The rendering dependency `@resvg/resvg-js` is MPL-2.0 licensed and used only to generate these assets; it is not bundled into the application. The Stream Deck category uses the same source SVG directly.

## Third-party service marks

The following unmodified SVGs in `ui/public/brand/services/` come from [Simple Icons](https://github.com/simple-icons/simple-icons), commit [`1bb2f6e6ea50f920de8d2f3d273ae48564a5eaee`](https://github.com/simple-icons/simple-icons/tree/1bb2f6e6ea50f920de8d2f3d273ae48564a5eaee/icons), under [CC0 1.0](https://github.com/simple-icons/simple-icons/blob/1bb2f6e6ea50f920de8d2f3d273ae48564a5eaee/LICENSE.md): YouTube, Twitch, TikTok, Kick, OBS Studio, Streamlabs and Elgato. The Elgato mark identifies the Stream Deck integration. The marks and names are trademarks of their respective owners; their use identifies compatibility and does not imply endorsement.

Restream and custom RTMP use a neutral network symbol until an appropriate distributable mark has been verified. The UI names them in text.
