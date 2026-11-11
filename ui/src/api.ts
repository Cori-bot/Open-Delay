import type { Command, Preferences, RouteConfig, Status } from './types';
import { invoke } from '@tauri-apps/api/core';
import { parseStatus, parseRoutes, parsePreferences, parseCommandResult, parseSavedPreferences, parseDiagnostics } from './validation';
import { parseOverlayControl } from './overlayControl';
import type { OverlayState } from './overlayControl';
import type { OverlayConfig } from './overlay';
export class ApiError extends Error { constructor(message: string, public readonly status: number, public readonly reason?: string) { super(message); } }
let session: Promise<string> | undefined;
const base = '/api/v1';
let origin: Promise<string> | undefined;
export const isDesktop = () => '__TAURI_INTERNALS__' in window;
export function getApiBase(): Promise<string> {
  origin ??= (async () => {
    const service = isDesktop() ? await invoke<{ api_base: string; error?: string }>('service_info') : null;
    if (service?.error) throw new ApiError(service.error, 503);
    const candidate = service?.api_base || import.meta.env.VITE_OPEN_DELAY_API || location.origin;
    const parsed = new URL(candidate);
    if (parsed.protocol !== 'http:' || !['127.0.0.1', 'localhost', '[::1]'].includes(parsed.hostname) || parsed.username || parsed.password) throw new ApiError('The engine must use a local HTTP address.', 400);
    return parsed.origin;
  })().catch(error => { origin = undefined; throw error; });
  return origin;
}
export async function retryService(apiPort: number, rtmpPort: number) {
  if (!isDesktop()) throw new ApiError('Restart the engine with the desired ports.', 409);
  await invoke('retry_service', { apiPort, rtmpPort }); origin = undefined; session = undefined;
}
export async function applyDesktopPreferences(): Promise<string[]> {
  if (!isDesktop()) return [];
  const result = await invoke<{ errors: string[] }>('apply_desktop_preferences');
  return result.errors;
}
async function token(): Promise<string> {
  session ??= getApiBase().then(apiBase => fetch(`${apiBase}${base}/session`, { cache: 'no-store', signal: AbortSignal.timeout(10000) })).then(async response => {
    if (!response.ok) throw new ApiError('Session indisponible / session unavailable', response.status);
    const value: unknown = await response.json();
    if (!value || typeof value !== 'object' || !('token' in value) || typeof value.token !== 'string') throw new ApiError('Invalid session response', 502);
    return value.token;
  }).catch(error => { session = undefined; throw error; });
  return session;
}
async function request<T>(path: string, parse: (value: unknown) => T, body?: unknown, signal?: AbortSignal, method?: string): Promise<T> {
  const response = await fetch(`${await getApiBase()}${base}${path}`, { method: method ?? (body === undefined ? 'GET' : 'POST'), cache: 'no-store', signal: signal ? AbortSignal.any([signal, AbortSignal.timeout(10000)]) : AbortSignal.timeout(10000), headers: { 'Content-Type': 'application/json', 'X-OpenDelay-Token': await token() }, ...(body === undefined ? {} : { body: JSON.stringify(body) }) });
  let value: unknown;
  try { value = await response.json(); } catch { throw new ApiError('Invalid JSON response from the local engine', 502); }
  if (response.status === 401) session = undefined;
  if (!response.ok) {
    const error = value && typeof value === 'object' ? value as Record<string, unknown> : {};
    throw new ApiError(typeof error.message === 'string' ? error.message : `HTTP ${response.status}`, response.status, typeof error.reason === 'string' ? error.reason : undefined);
  }
  try { return parse(value); } catch (error) { throw new ApiError(error instanceof Error ? error.message : 'Invalid engine response', 502); }
}
export const api = {
  overlayControl: (signal?: AbortSignal) => request('/overlay-control', parseOverlayControl, undefined, signal),
  saveOverlay: (config: OverlayConfig) => request('/overlay-control', parseOverlayControl, config),
  testOverlay: (state: OverlayState | 'stop', delay_ms = 15000) => request('/overlay-test', parseOverlayControl, { state, delay_ms }),
  status: (signal?: AbortSignal) => request('/status', parseStatus, undefined, signal),
  routes: (signal?: AbortSignal) => request('/routes', parseRoutes, undefined, signal),
  saveRoute: (route: RouteConfig) => request('/routes', parseRoutes, route),
  removeRoute: (id: string) => request(`/routes/${encodeURIComponent(id)}`, parseRoutes, undefined, undefined, 'DELETE'),
  preferences: (signal?: AbortSignal) => request('/preferences', parsePreferences, undefined, signal),
  savePreferences: (preferences: Preferences) => request('/preferences', parseSavedPreferences, preferences),
  command: (value: Omit<Command, 'id'>, id: string = crypto.randomUUID()) => request('/commands', parseCommandResult, { id, ...value }),
  diagnostics: () => request('/diagnostics', parseDiagnostics),
};
export async function statusEvents(signal: AbortSignal, receive: (status: Status) => void): Promise<void> {
  const response = await fetch(`${await getApiBase()}${base}/events`, { signal, cache: 'no-store', headers: { 'X-OpenDelay-Token': await token(), Accept: 'text/event-stream' } });
  if (!response.ok || !response.body) throw new ApiError('Status event stream unavailable', response.status);
  const reader = response.body.getReader(); const decoder = new TextDecoder(); let pending = '';
  try {
    while (!signal.aborted) {
      const { value, done } = await new Promise<ReadableStreamReadResult<Uint8Array>>((resolve,reject) => {
        const deadline=setTimeout(()=>reject(new ApiError('Status event stream is stale',503)),20000);
        reader.read().then(value=>{clearTimeout(deadline);resolve(value);},error=>{clearTimeout(deadline);reject(error);});
      }); if (done) break;
      pending = (pending + decoder.decode(value, { stream: true })).replace(/\r\n/g, '\n');
      if (pending.length > 1048576) throw new ApiError('Status event exceeds size limit', 502);
      let boundary: number;
      while ((boundary = pending.indexOf('\n\n')) !== -1) {
        const block = pending.slice(0, boundary); pending = pending.slice(boundary + 2);
        const data = block.split('\n').filter(line => line.startsWith('data:')).map(line => line.slice(5).trimStart()).join('\n');
        if (data) {
          let value: unknown;
          try { value = JSON.parse(data); } catch { throw new ApiError('Invalid status event JSON', 502); }
          receive(parseStatus(value));
        }
      }
    }
  } finally { await reader.cancel().catch(() => undefined); reader.releaseLock(); }
  if (!signal.aborted) throw new ApiError('Status event stream closed', 503);
}
