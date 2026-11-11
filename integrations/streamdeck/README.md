# Open-Delay Stream Deck

Original plugin for Stream Deck 7.1+, Windows 10+, Node.js 24 runtime supplied by Stream Deck. Four keypad actions: Set Delay, Go Live, Toggle Delay, Status. First three support Multi Actions. Zero seconds means Go Live.

`npm ci`, `npm test`, `npm run validate`, `npm run pack`. The pack command creates `dist/dev.opendelay.streamDeckPlugin`; open that file to install through Stream Deck. No automatic installation or developer-mode changes are performed by these commands.

Start Open-Delay first. The plugin discovers the current loopback endpoint from `%LOCALAPPDATA%/Open-Delay/runtime.json`, including after automatic port changes. No fixed default HTTP port. Session tokens stay in memory and travel in headers, never URLs or settings. A separate custom headless data directory is not automatically discovered.

Optional route ID limits an action to one route; blank targets connected group routes. Display: OFF disconnected/unavailable, REC recovering, DEG degraded, BUILD preparing/transitioning, LIVE live, Ns actual deliberate delay. A refused command shows Stream Deck's alert; transport failure never triggers an automatic second toggle.

No proprietary code, assets or SDK runtime dependency. The build-time Elgato CLI validates/packages the plugin. This plugin follows the repository's source-available license; Elgato tooling retains its own license. Hardware interaction remains to be validated on a real device.
