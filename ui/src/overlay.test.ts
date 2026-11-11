import { describe, expect, it } from 'vitest';
import { defaultOverlay, overlayUrl, parseOverlay, overlayPresentation } from './overlay';
import { parseOverlayControl } from './overlayControl';
describe('overlay URL contract', () => {
  it('roundtrips a configured source without preview or token parameters', () => {
    const config = { ...defaultOverlay, text: 'Pause & café ?', radius: 88, circleIcon: true, overrideBadgeText: true };
    const url = new URL(overlayUrl(config, 'http://127.0.0.1:54321'));
    expect(parseOverlay(url.searchParams)).toEqual(config);
    expect(url.searchParams.has('preview')).toBe(false);
    expect(url.searchParams.has('token')).toBe(false);
  });
  it('bounds untrusted parameters and enforces dependent options', () => {
    const config = parseOverlay(new URLSearchParams({ theme: '<script>', color: 'constructor', radius: '9999', icon: 'false', circleIcon: 'true', iconMatchText: 'true', overrideBadgeText: 'true' }));
    expect(config.theme).toBe('minimal'); expect(config.color).toBe('purple'); expect(config.radius).toBe(100);
    expect(config.circleIcon).toBe(false); expect(config.iconMatchText).toBe(false); expect(config.overrideBadgeText).toBe(false);
  });
  it('rejects non-finite radius and limits text to 48 characters', () => {
    const config = parseOverlay(new URLSearchParams({ radius: 'NaN', text: 'a'.repeat(80) }));
    expect(config.radius).toBe(64); expect(config.text).toHaveLength(48);
  });
});
describe('managed overlay presentation', () => {
  const live = { effective_mode: 'live', effective_delay_ms: 0, desired_delay_ms: 0, mode: 'overlay', overlay_hidden: false, is_connected: true };
  it('uses effective state on return to live and remaining history during preparation', () => {
    expect(overlayPresentation(live, defaultOverlay)).toMatchObject({ state: 'live', delay: 0, visible: true });
    expect(overlayPresentation({ ...live, effective_mode: 'preparing', desired_delay_ms: 30000, buffer: { fill_ms: 12000, target_ms: 30000 } }, defaultOverlay)).toMatchObject({ state: 'building', delay: 18000 });
  });
  it('hides disconnected, degraded and Instant live sources consistently', () => {
    expect(overlayPresentation(null, defaultOverlay).visible).toBe(false);
    expect(overlayPresentation({ ...live, is_connected: false }, defaultOverlay).visible).toBe(false);
    expect(overlayPresentation({ ...live, effective_mode: 'degraded' }, defaultOverlay).visible).toBe(false);
    expect(overlayPresentation({ ...live, mode: 'instant' }, defaultOverlay).visible).toBe(false);
  });
  it('validates shared config and test bounds before rendering the source', () => {
    const value = { config: defaultOverlay, revision: 1, source_recent: true, test: { id: 'test', state: 'delayed', delay_ms: 15000, expires_at_ms: 2000000000000 } };
    expect(parseOverlayControl(value).config).toEqual(defaultOverlay);
    expect(() => parseOverlayControl({ ...value, config: { ...defaultOverlay, theme: '<script>' } })).toThrow();
    expect(() => parseOverlayControl({ ...value, test: { ...value.test, delay_ms: 600001 } })).toThrow();
    expect(() => parseOverlayControl({ ...value, config: { ...defaultOverlay, text: '😀'.repeat(25) } })).toThrow();
  });
});
