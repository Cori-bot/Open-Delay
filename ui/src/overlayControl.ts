import { defaultOverlay, parseOverlay } from './overlay';
import type { OverlayConfig } from './overlay';

export type OverlayState = 'live' | 'building' | 'delayed';
export type OverlayControl = {
  config: OverlayConfig | null;
  revision: number;
  source_recent: boolean;
  test: { id: string; state: OverlayState; delay_ms: number; expires_at_ms: number } | null;
};
export function parseOverlayControl(value: unknown): OverlayControl {
  const fail = (): never => { throw new Error('Invalid overlay control response'); };
  if (!value || typeof value !== 'object') return fail();
  const record = value as Record<string, unknown>;
  if (!Number.isSafeInteger(record.revision) || Number(record.revision) < 0 || typeof record.source_recent !== 'boolean') return fail();
  let config: OverlayConfig | null = null;
  if (record.config !== null) {
    if (!record.config || typeof record.config !== 'object') return fail();
    const candidate = record.config as Record<string, unknown>;
    const params = new URLSearchParams();
    for (const [key, fallback] of Object.entries(defaultOverlay)) {
      if (typeof candidate[key] !== typeof fallback) return fail();
      params.set(key, String(candidate[key]));
    }
    config = parseOverlay(params);
    if (Object.entries(config).some(([key, entry]) => entry !== candidate[key])) return fail();
  }
  let test: OverlayControl['test'] = null;
  if (record.test !== null) {
    if (!record.test || typeof record.test !== 'object') return fail();
    const candidate = record.test as Record<string, unknown>;
    if (typeof candidate.id !== 'string' || !['live', 'building', 'delayed'].includes(String(candidate.state)) || !Number.isSafeInteger(candidate.delay_ms) || Number(candidate.delay_ms) < 0 || Number(candidate.delay_ms) > 600000 || !Number.isSafeInteger(candidate.expires_at_ms)) return fail();
    test = candidate as OverlayControl['test'];
  }
  return { config, revision: Number(record.revision), source_recent: record.source_recent, test };
}
