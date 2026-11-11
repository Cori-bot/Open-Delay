import { readFile } from 'node:fs/promises';
import path from 'node:path';

export function localBase(value) {
  if (typeof value !== 'string') throw new Error('Invalid discovery address');
  const url = new URL(value);
  if (url.protocol !== 'http:' || !['127.0.0.1', 'localhost'].includes(url.hostname)
      || url.username || url.password || url.pathname !== '/' || url.search || url.hash
      || !url.port || Number(url.port) < 1 || Number(url.port) > 65535) {
    throw new Error('Open-Delay must use a loopback HTTP address');
  }
  return url.origin;
}

export function normalizeSettings(settings = {}) {
  if (!settings || typeof settings !== 'object' || Array.isArray(settings)) throw new Error('Invalid action settings');
  if (settings.delay_seconds !== undefined && typeof settings.delay_seconds !== 'number'
    && (typeof settings.delay_seconds !== 'string' || !/^\d+$/.test(settings.delay_seconds))) throw new Error('Invalid delay value');
  const seconds = settings.delay_seconds === undefined ? 15 : Number(settings.delay_seconds);
  if (!Number.isInteger(seconds) || seconds < 0 || seconds > 600) throw new Error('Delay must be an integer from 0 to 600');
  const route = typeof settings.route_id === 'string' ? settings.route_id.trim() : '';
  if (route && !/^[a-zA-Z0-9_-]{1,64}$/.test(route)) throw new Error('Invalid route identity');
  return { delay_seconds: seconds, route_id: route };
}

export function commandFor(action, settings, id) {
  const normalized = normalizeSettings(settings);
  const selected = normalized.route_id ? { route_ids: [normalized.route_id] } : {};
  if (action === 'dev.opendelay.golive' || action === 'dev.opendelay.setdelay' && normalized.delay_seconds === 0) {
    return { id, action: 'go_live', ...selected };
  }
  if (action === 'dev.opendelay.setdelay') return { id, action: 'prepare_delay', delay_ms: normalized.delay_seconds * 1000, ...selected };
  if (action === 'dev.opendelay.toggle') return { id, action: 'toggle_delay', delay_ms: normalized.delay_seconds * 1000, ...selected };
  return null;
}

const actionIds = new Set(['dev.opendelay.setdelay', 'dev.opendelay.golive', 'dev.opendelay.toggle', 'dev.opendelay.status']);
const eventIds = new Set(['willAppear', 'willDisappear', 'didReceiveSettings', 'keyDown']);
export function parseStreamDeckEvent(raw) {
  if (typeof raw !== 'string' || raw.length > 1024 * 1024) return null;
  let message;
  try { message = JSON.parse(raw); } catch { return null; }
  if (!message || typeof message !== 'object' || Array.isArray(message)
      || !eventIds.has(message.event) || typeof message.context !== 'string'
      || !message.context || message.context.length > 256) return null;
  if ((message.event === 'willAppear' || message.event === 'keyDown') && !actionIds.has(message.action)) return null;
  return message;
}

export function validStatus(status) {
  const delay = value => Number.isSafeInteger(value) && value >= 0;
  return !!status && typeof status === 'object' && typeof status.is_connected === 'boolean'
    && delay(status.sequence)
    && typeof status.effective_mode === 'string' && delay(status.effective_delay_ms)
    && typeof status.transition_in_progress === 'boolean' && Array.isArray(status.routes)
    && status.routes.length <= 32
    && status.routes.every(route => route && typeof route.id === 'string'
      && typeof route.include_in_group === 'boolean'
      && typeof route.is_connected === 'boolean' && typeof route.transport_state === 'string'
      && typeof route.effective_mode === 'string' && delay(route.effective_delay_ms)
      && route.buffer && typeof route.buffer.degraded === 'boolean');
}

export class EngineChangedError extends Error {
  constructor() { super('Engine changed during request'); }
}

export class StatusCursor {
  constructor() { this.epoch = -1; this.sequence = -1; }
  accept(status, epoch) {
    if (!validStatus(status) || epoch < this.epoch) return false;
    if (epoch > this.epoch) { this.epoch = epoch; this.sequence = -1; }
    if (status.sequence < this.sequence) return false;
    this.sequence = status.sequence;
    return true;
  }
}

export function buttonState(status, settings = {}) {
  if (!validStatus(status)) return { title: 'OFF', active: false };
  const { route_id: routeId } = normalizeSettings(settings);
  const selected = routeId ? status.routes.filter(route => route.id === routeId)
    : status.routes.filter(route => route.is_connected && route.include_in_group !== false);
  if (!selected.length || !selected.some(route => route.is_connected)) return { title: 'OFF', active: false };
  if (selected.some(route => route.transport_state === 'recovering' || route.recovery === 'retrying')) return { title: 'REC', active: false };
  if (selected.some(route => route.buffer.degraded || route.effective_mode === 'degraded')) return { title: 'DEG', active: false };
  if (selected.some(route => ['building_delay', 'transitioning', 'preparing'].includes(route.effective_mode))) return { title: 'BUILD', active: false };
  const delay = Math.min(...selected.filter(route => route.is_connected).map(route => route.effective_delay_ms));
  return delay > 0 ? { title: `${Math.floor(delay / 1000)}s`, active: true } : { title: 'LIVE', active: false };
}

export class OpenDelayClient {
  constructor({ discoveryPath, fetchImpl = fetch, readFileImpl = readFile } = {}) {
    this.discoveryPath = discoveryPath ?? path.join(process.env.LOCALAPPDATA ?? '', 'Open-Delay', 'runtime.json');
    this.fetchImpl = fetchImpl;
    this.readFileImpl = readFileImpl;
    this.base = null;
    this.token = null;
    this.sessionPromise = null;
    this.epoch = 0;
    this.pid = null;
  }
  async discover() {
    const text = await this.readFileImpl(this.discoveryPath, 'utf8');
    if (text.length > 4096) throw new Error('Invalid discovery file');
    const runtime = JSON.parse(text);
    if (runtime?.name !== 'Open-Delay' || !Number.isInteger(runtime.pid) || runtime.pid < 1) throw new Error('Invalid discovery file');
    const base = localBase(runtime.api_base);
    if (base !== this.base || runtime.pid !== this.pid) {
      this.epoch += 1; this.base = base; this.pid = runtime.pid; this.token = null; this.sessionPromise = null;
    }
    return base;
  }
  async session(base) {
    const epoch = this.epoch;
    if (this.token) return this.token;
    if (this.sessionPromise?.epoch === epoch && this.sessionPromise.base === base) return this.sessionPromise.promise;
    const promise = (async () => {
      const response = await this.fetchImpl(`${base}/api/v1/session`, { headers: { Origin: base }, signal: AbortSignal.timeout(1500), redirect: 'error' });
      if (!response.ok) throw new Error('Local session unavailable');
      const data = await response.json();
      if (typeof data?.token !== 'string' || data.token.length < 8 || data.token.length > 256) throw new Error('Invalid local session');
      if (this.base !== base || this.epoch !== epoch) throw new EngineChangedError();
      this.token = data.token;
      return data.token;
    })().finally(() => { if (this.sessionPromise?.promise === promise) this.sessionPromise = null; });
    this.sessionPromise = { base, epoch, promise };
    return promise;
  }
  async request(endpoint, body) {
    const base = await this.discover();
    const epoch = this.epoch;
    const token = await this.session(base);
    if (this.epoch !== epoch || this.base !== base) throw new EngineChangedError();
    const response = await this.fetchImpl(`${base}/api/v1/${endpoint}`, {
      method: body === undefined ? 'GET' : 'POST', redirect: 'error', signal: AbortSignal.timeout(2000),
      headers: { Origin: base, 'X-OpenDelay-Token': token, 'Content-Type': 'application/json' },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    });
    if (response.status === 401 && this.epoch === epoch && this.base === base) this.token = null;
    if (!response.ok) throw new Error(`Local engine refused request (${response.status})`);
    const value = await response.json();
    if (this.epoch !== epoch || this.base !== base) throw new EngineChangedError();
    return value;
  }
  async status() {
    const value = await this.request('status');
    if (!validStatus(value)) throw new Error('Invalid local status');
    return value;
  }
  async command(body) {
    // Never automatically repeat a toggle after an ambiguous transport failure.
    // The same caller-supplied id can be reused explicitly for a deduplicated retry.
    const result = await this.request('commands', body);
    if (!result || typeof result.success !== 'boolean' || !validStatus(result.status)) throw new Error('Invalid command response');
    return result;
  }
}
