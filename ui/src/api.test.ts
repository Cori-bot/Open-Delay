import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { defaultPreferences } from './types';
function status(sequence: number) { return { sequence, mode: 'instant', effective_mode: 'live', is_connected: false, desired_delay_ms: 0, effective_delay_ms: 0, transition_in_progress: false, overlay_hidden: false, routes: [], preferences: defaultPreferences }; }
beforeEach(() => { vi.resetModules(); vi.stubGlobal('window', {}); vi.stubGlobal('location', { origin: 'http://127.0.0.1:49871' }); });
afterEach(() => vi.unstubAllGlobals());
function response(value: unknown, status = 200) { return new Response(JSON.stringify(value), { status, headers: { 'Content-Type': 'application/json' } }); }
describe('engine API', () => {
  it('obtains a memory-only token and sends it as a header, never in the URL', async () => {
    const fetcher = vi.fn().mockResolvedValueOnce(response({ token: 'test-session' })).mockResolvedValueOnce(response({ routes: [] }));
    vi.stubGlobal('fetch', fetcher);
    const { api } = await import('./api');
    expect(await api.routes()).toEqual([]);
    expect(fetcher.mock.calls[0][0]).toBe('http://127.0.0.1:49871/api/v1/session');
    expect(fetcher.mock.calls[1][1].headers['X-OpenDelay-Token']).toBe('test-session');
    expect(fetcher.mock.calls.every(call => !String(call[0]).includes('test-session'))).toBe(true);
  });
  it('preserves route refusals and returned status for partial commands', async () => {
    const result = { success: false, reason: 'partial_failure', message: 'Some routes refused', status: status(4), results: [{ route_id: 'custom', success: false, reason: 'buffer_not_ready' }] };
    vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(response({ token: 'test' })).mockResolvedValueOnce(response(result)));
    const { api } = await import('./api');
    expect(await api.command({ action: 'go_live' }, 'same-command-id')).toEqual(result);
  });
  it('reacquires the session after an unauthorized response without replaying the failed mutation', async () => {
    const fetcher = vi.fn().mockResolvedValueOnce(response({ token: 'expired' })).mockResolvedValueOnce(response({ message: 'Expired' }, 401)).mockResolvedValueOnce(response({ token: 'fresh' })).mockResolvedValueOnce(response({ routes: [] }));
    vi.stubGlobal('fetch', fetcher);
    const { api } = await import('./api');
    await expect(api.command({ action: 'go_live' })).rejects.toThrow('Expired');
    await api.routes();
    expect(fetcher.mock.calls.filter(call => String(call[0]).endsWith('/commands'))).toHaveLength(1);
    expect(fetcher.mock.calls[3][1].headers['X-OpenDelay-Token']).toBe('fresh');
  });
  it('parses status events split across chunks and CRLF boundaries', async () => {
    const encoder = new TextEncoder();
    const encoded = JSON.stringify(status(7));
    const stream = new ReadableStream({ start(controller) { for (const text of ['event: status\r', `\ndata: ${encoded.slice(0, 12)}`, `${encoded.slice(12)}\r\n\r`, '\n']) controller.enqueue(encoder.encode(text)); controller.close(); } });
    vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(response({ token: 'test' })).mockResolvedValueOnce(new Response(stream, { headers: { 'Content-Type': 'text/event-stream' } })));
    const { statusEvents } = await import('./api'); const receive = vi.fn();
    await expect(statusEvents(new AbortController().signal, receive)).rejects.toThrow('Status event stream closed');
    expect(receive).toHaveBeenCalledExactlyOnceWith(status(7));
  });
  it('rejects malformed successful status responses instead of rendering unchecked data', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(response({ token: 'test' })).mockResolvedValueOnce(response({ sequence: 3, routes: null })));
    const { api } = await import('./api');
    await expect(api.status()).rejects.toThrow('Invalid status response');
  });
  it('does not deliver structurally invalid SSE events to React', async () => {
    const stream = new ReadableStream({ start(controller) { controller.enqueue(new TextEncoder().encode('event: status\ndata: {"sequence":2}\n\n')); controller.close(); } });
    vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(response({ token: 'test' })).mockResolvedValueOnce(new Response(stream)));
    const { statusEvents } = await import('./api'); const receive = vi.fn();
    await expect(statusEvents(new AbortController().signal, receive)).rejects.toThrow('Invalid status response');
    expect(receive).not.toHaveBeenCalled();
  });
});
