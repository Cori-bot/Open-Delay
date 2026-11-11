let socket, context, settings = {};
const delay = document.getElementById('delay');
const route = document.getElementById('route');
const message = document.getElementById('message');
window.connectElgatoStreamDeckSocket = (port, uuid, registerEvent, _info, actionInfo) => {
  const portNumber = Number(port);
  let info;
  try { info = JSON.parse(actionInfo); } catch { message.textContent = 'Invalid Stream Deck action information.'; return; }
  if (!Number.isInteger(portNumber) || portNumber < 1 || portNumber > 65535
      || typeof uuid !== 'string' || !uuid || uuid.length > 256 || registerEvent !== 'registerPropertyInspector'
      || !info || typeof info !== 'object' || Array.isArray(info) || typeof info.context !== 'string' || !info.context
      || !info.payload || typeof info.payload !== 'object' || Array.isArray(info.payload)) {
    message.textContent = 'Invalid Stream Deck registration.'; return;
  }
  context = info.context;
  const actionSettings = info.payload.settings;
  if (actionSettings !== undefined && (!actionSettings || typeof actionSettings !== 'object' || Array.isArray(actionSettings))) {
    message.textContent = 'Invalid Stream Deck settings.'; return;
  }
  settings = actionSettings ?? {};
  delay.value = settings.delay_seconds ?? 15; route.value = settings.route_id ?? '';
  if (info.action === 'dev.opendelay.golive' || info.action === 'dev.opendelay.status') delay.disabled = true;
  socket = new WebSocket(`ws://127.0.0.1:${portNumber}`);
  socket.addEventListener('open', () => socket.send(JSON.stringify({ event: registerEvent, uuid })));
  socket.addEventListener('close', () => { message.textContent = 'Stream Deck connection closed.'; });
  socket.addEventListener('error', () => { message.textContent = 'Stream Deck connection failed.'; });
};
function save() {
  const seconds = Number(delay.value); const routeId = route.value.trim();
  if (!/^\d+$/.test(delay.value) || !Number.isInteger(seconds) || seconds < 0 || seconds > 600 || routeId && !/^[a-zA-Z0-9_-]{1,64}$/.test(routeId)) { message.textContent = 'Use an integer delay 0–600 and a valid route ID.'; return; }
  settings = { ...settings, delay_seconds: seconds, route_id: routeId };
  if (socket?.readyState === WebSocket.OPEN) { socket.send(JSON.stringify({ event: 'setSettings', context, payload: settings })); message.textContent = 'Saved / Enregistré'; }
}
delay.addEventListener('change', save); route.addEventListener('change', save);
