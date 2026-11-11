import type { CommandResult, Preferences, RouteConfig, RouteStatus, Status } from './types';

type RecordValue = Record<string, unknown>;
const object = (value: unknown): value is RecordValue => value !== null && typeof value === 'object' && !Array.isArray(value);
const number = (value: unknown): value is number => typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
const text = (value: unknown): value is string => typeof value === 'string';
const optionalText = (value: unknown) => value === undefined || value === null || text(value);
const mode = (value: unknown) => value === 'instant' || value === 'overlay';
const port = (value: unknown) => number(value) && value <= 65535;

function invalid(label: string): never { throw new Error(`Invalid ${label} response from the local engine`); }

export function isPreferences(value: unknown): value is Preferences {
  if (!object(value) || !object(value.hotkeys)) return false;
  const keys = value.hotkeys;
  return (value.language === 'fr' || value.language === 'en') && mode(value.mode)
    && number(value.buffer_ms) && value.buffer_ms >= 1000 && value.buffer_ms <= 600000
    && number(value.disk_bytes) && number(value.cache_bytes) && port(value.api_port) && port(value.rtmp_port)
    && ['lan_ingest', 'startup', 'quit_on_close', 'onboarding_complete'].every(key => typeof value[key] === 'boolean')
    && ['start_delay', 'go_live', 'toggle_overlay', 'custom_delay'].every(key => keys[key] === null || text(keys[key]) && keys[key].length <= 80)
    && number(keys.custom_delay_ms) && keys.custom_delay_ms >= 1000 && keys.custom_delay_ms <= 600000;
}
export function parsePreferences(value: unknown): Preferences { return isPreferences(value) ? value : invalid('preferences'); }

function isRouteConfig(value: unknown): value is RouteConfig {
  return object(value) && ['id', 'name', 'platform', 'local_app', 'upstream_url'].every(key => text(value[key]))
    && typeof value.enabled === 'boolean' && typeof value.include_in_group === 'boolean';
}
export function parseRoutes(value: unknown): RouteConfig[] {
  const routes = Array.isArray(value) ? value : object(value) ? value.routes : undefined;
  return Array.isArray(routes) && routes.length <= 32 && routes.every(isRouteConfig) ? routes : invalid('routes');
}
function isRouteStatus(value: unknown): value is RouteStatus {
  if (!object(value) || !object(value.buffer)) return false;
  return text(value.id) && typeof value.is_connected === 'boolean' && number(value.generation)
    && text(value.transport_state) && text(value.effective_mode) && number(value.desired_delay_ms)
    && number(value.effective_delay_ms) && number(value.bytes_received) && number(value.bytes_sent)
    && number(value.buffer.fill_ms) && number(value.buffer.target_ms) && number(value.buffer.size_bytes)
    && typeof value.buffer.ready === 'boolean' && typeof value.buffer.degraded === 'boolean' && optionalText(value.error);
}
function isEndpoints(value: unknown): boolean {
  if (!object(value) || !text(value.api_base) || !port(value.rtmp_port) || value.rtmp_port === 0) return false;
  try {
    const url = new URL(value.api_base);
    return url.protocol === 'http:' && ['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname)
      && !url.username && !url.password && !!url.port && url.pathname === '/' && !url.search && !url.hash;
  } catch { return false; }
}
export function parseStatus(value: unknown): Status {
  if (!object(value) || !number(value.sequence) || !mode(value.mode) || !text(value.effective_mode)
    || typeof value.is_connected !== 'boolean' || !number(value.desired_delay_ms) || !number(value.effective_delay_ms)
    || typeof value.transition_in_progress !== 'boolean' || typeof value.overlay_hidden !== 'boolean'
    || !Array.isArray(value.routes) || value.routes.length > 32 || !value.routes.every(isRouteStatus)
    || !isPreferences(value.preferences) || value.endpoints !== undefined && !isEndpoints(value.endpoints)) return invalid('status');
  return value as Status;
}
export function parseCommandResult(value: unknown): CommandResult {
  if (!object(value) || typeof value.success !== 'boolean' || !optionalText(value.message) || !optionalText(value.reason)
    || value.results !== undefined && (!Array.isArray(value.results) || !value.results.every(result => object(result)
      && text(result.route_id) && typeof result.success === 'boolean' && optionalText(result.reason)))) return invalid('command');
  const status = parseStatus(value.status);
  return { ...value, status } as CommandResult;
}
export function parseSavedPreferences(value: unknown): { success: boolean; restart_required: boolean; preferences: Preferences } {
  if (!object(value) || value.success !== true || typeof value.restart_required !== 'boolean') return invalid('saved preferences');
  return { success: true, restart_required: value.restart_required, preferences: parsePreferences(value.preferences) };
}
export function parseDiagnostics(value: unknown): { entries: { at: string; level: string; message: string }[] } {
  if (!object(value)) return invalid('diagnostics');
  const entries = value.entries ?? value.logs;
  if (!Array.isArray(entries) || !entries.every(entry => object(entry) && text(entry.at) && text(entry.level) && text(entry.message))) return invalid('diagnostics');
  return { entries: entries as { at: string; level: string; message: string }[] };
}

export type OverlayStatus = { effective_mode: string; effective_delay_ms: number; desired_delay_ms: number; mode: 'instant' | 'overlay'; overlay_hidden: boolean; is_connected: boolean; buffer?: { fill_ms: number; target_ms: number } };
export function parseOverlayStatus(value: unknown): OverlayStatus {
  if (!object(value) || !text(value.effective_mode) || !number(value.effective_delay_ms) || !number(value.desired_delay_ms)
    || !mode(value.mode) || typeof value.overlay_hidden !== 'boolean' || typeof value.is_connected !== 'boolean'
    || value.buffer !== undefined && (!object(value.buffer) || !number(value.buffer.fill_ms) || !number(value.buffer.target_ms))) return invalid('overlay status');
  return value as OverlayStatus;
}
