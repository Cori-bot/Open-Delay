import { randomUUID } from 'node:crypto';
import { OpenDelayClient, commandFor, buttonState, parseStreamDeckEvent, StatusCursor, EngineChangedError } from './client.mjs';

const args = new Map();
for (let index = 2; index + 1 < process.argv.length; index += 2) args.set(process.argv[index], process.argv[index + 1]);
const port = Number(args.get('-port'));
const uuid = args.get('-pluginUUID');
const registerEvent = args.get('-registerEvent');
if (!Number.isInteger(port) || port < 1 || port > 65535 || !uuid || uuid.length > 256 || registerEvent !== 'registerPlugin') throw new Error('Stream Deck registration arguments are invalid');

const socket = new WebSocket(`ws://127.0.0.1:${port}`);
const client = new OpenDelayClient();
const contexts = new Map();
const statusCursor = new StatusCursor();
let stopped = false;
const send = value => { if (socket.readyState === WebSocket.OPEN) socket.send(JSON.stringify(value)); };
const alert = context => send({ event: 'showAlert', context });
function render(context, state) {
  const entry = contexts.get(context);
  if (!entry || entry.title === state.title && entry.active === state.active) return;
  entry.title = state.title; entry.active = state.active;
  send({ event: 'setTitle', context, payload: { title: state.title, target: 0 } });
  send({ event: 'setState', context, payload: { state: state.active ? 1 : 0 } });
}
async function refresh() {
  if (!contexts.size) return;
  try {
    const status = await client.status();
    renderStatus(status);
  } catch (error) {
    if (error instanceof EngineChangedError) return;
    for (const context of contexts.keys()) render(context, { title: 'OFF', active: false });
  }
}
function renderStatus(status) {
  if (!statusCursor.accept(status, client.epoch)) return;
  for (const [context, entry] of contexts) {
    try { render(context, buttonState(status, entry.settings)); } catch { render(context, { title: 'ERR', active: false }); }
  }
}
async function keyDown(message) {
  const entry = contexts.get(message.context);
  if (!entry || entry.busy) return;
  entry.busy = true;
  try {
    const command = commandFor(entry.action, entry.settings, randomUUID());
    if (command) {
      const result = await client.command(command);
      renderStatus(result.status);
      if (!result.success) alert(message.context);
    } else await refresh();
  } catch {
    render(message.context, { title: 'OFF', active: false });
    alert(message.context);
  } finally { entry.busy = false; }
}
socket.addEventListener('open', () => send({ event: registerEvent, uuid }));
socket.addEventListener('message', event => {
  const message = parseStreamDeckEvent(event.data);
  if (!message) return;
  if (message.event === 'willAppear') {
    contexts.set(message.context, { settings: message.payload?.settings ?? {}, action: message.action, busy: false });
    render(message.context, { title: 'OFF', active: false });
  } else if (message.event === 'willDisappear') contexts.delete(message.context);
  else if (message.event === 'didReceiveSettings') {
    const entry = contexts.get(message.context);
    if (entry) { entry.settings = message.payload?.settings ?? {}; entry.title = undefined; }
  } else if (message.event === 'keyDown') void keyDown(message);
});
async function poll() {
  if (stopped) return;
  await refresh();
  if (!stopped) setTimeout(poll, contexts.size ? 500 : 1500).unref();
}
socket.addEventListener('close', () => { stopped = true; });
socket.addEventListener('error', () => { stopped = true; process.exitCode = 1; });
void poll();
