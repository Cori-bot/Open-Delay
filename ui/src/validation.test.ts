import { describe, expect, it } from 'vitest';
import { parseOverlayStatus, parsePreferences, parseStatus } from './validation';
import { defaultPreferences } from './types';

describe('local protocol validation', () => {
  it('accepts a redacted overlay snapshot without private preferences or routes', () => {
    const value = { mode: 'instant', effective_mode: 'instant_delayed', effective_delay_ms: 15000, desired_delay_ms: 15000, overlay_hidden: false, is_connected: true };
    expect(parseOverlayStatus(value)).toEqual(value);
  });
  it('rejects invalid overlay booleans and non-finite timing', () => {
    expect(() => parseOverlayStatus({ mode: 'instant', effective_mode: 'instant_delayed', effective_delay_ms: Infinity, desired_delay_ms: 15000, overlay_hidden: false, is_connected: 'true' })).toThrow();
  });
  it('rejects malformed preference and endpoint boundaries', () => {
    expect(() => parsePreferences({ ...defaultPreferences, hotkeys: null })).toThrow();
    const snapshot = { sequence: 1, mode: 'instant', effective_mode: 'live', is_connected: false, desired_delay_ms: 0, effective_delay_ms: 0, transition_in_progress: false, overlay_hidden: false, routes: [], preferences: defaultPreferences };
    expect(() => parseStatus({ ...snapshot, endpoints: { api_base: 'https://remote.example', rtmp_port: 49155 } })).toThrow();
    expect(parseStatus({ ...snapshot, endpoints: { api_base: 'http://127.0.0.1:49154', rtmp_port: 49155 } }).sequence).toBe(1);
  });
});
