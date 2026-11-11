# Open-Delay UI

Original React/TypeScript interface shared by the bundled Tauri window and browser dock. No copied InstantDelay code, images, fonts or logos; all decoration is local CSS/SVG. No telemetry or external asset requests.

## Development

```powershell
npm.cmd install
$env:VITE_OPEN_DELAY_API = 'http://127.0.0.1:<allocated-api-port>'
npm.cmd run dev
```

Use the API address printed by the running Open-Delay service. There is no fixed API/RTMP port and no proxy to an unrelated local server. Native builds use `service_info` over Tauri; browser builds use their own origin. Production files are written to `ui/dist` by `npm.cmd run build`.

```powershell
npm.cmd test
npm.cmd run build
```

The frontend never starts a stream or changes OBS settings. Platform keys stay in OBS/Streamlabs. The standalone `/overlay` page uses only public redacted status; its editor preview is explicitly simulated. A disconnected or failed API is shown as unavailable rather than protected. Existing research screenshots are private reference material, not application assets.

Native commands expected: `service_info`, `retry_service({apiPort,rtmpPort})`, `apply_desktop_preferences`. Global shortcuts are registered by the shell; browser keyboard shortcuts require focus. Automatic updates and experimental codec support are not presented as validated features.
