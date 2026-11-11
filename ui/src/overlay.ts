export const colors = { red: '#ef6671', orange: '#f6a15a', yellow: '#f0cf65', lime: '#c4e877', green: '#75d9a0', teal: '#69d3c5', cyan: '#73d7ed', blue: '#80a9f8', indigo: '#a4a0ff', purple: '#b29aff', pink: '#f29acb', white: '#f1efff' };
export const themes = ['minimal', 'studio', 'saas', 'signal', 'glass', 'light'] as const;
export const positions = ['top-left', 'top-center', 'top-right', 'bottom-left', 'bottom-center', 'bottom-right'] as const;
export type OverlayConfig = { theme: typeof themes[number]; color: keyof typeof colors; position: typeof positions[number]; badgeSize: 'xs' | 's' | 'm' | 'l'; popupSize: 's' | 'm' | 'l' | 'xl'; radius: number; text: string; badge: boolean; popup: boolean; icon: boolean; circleIcon: boolean; iconMatchText: boolean; hideLive: boolean; instantLive: boolean; liveGreen: boolean; motion: boolean; overrideBadgeText: boolean; lang: 'fr' | 'en' };
export const defaultOverlay: OverlayConfig = { theme: 'minimal', color: 'purple', position: 'top-right', badgeSize: 's', popupSize: 'm', radius: 64, text: '', badge: true, popup: true, icon: true, circleIcon: false, iconMatchText: false, hideLive: false, instantLive: false, liveGreen: false, motion: true, overrideBadgeText: false, lang: 'fr' };
export function overlayPresentation(status: { effective_mode: string; effective_delay_ms: number; desired_delay_ms: number; mode: string; overlay_hidden: boolean; is_connected: boolean; buffer?: { fill_ms: number; target_ms: number } } | null, config: OverlayConfig) {
  const state = status?.effective_mode === 'building_delay' || status?.effective_mode === 'preparing' ? 'building' as const : (status?.effective_delay_ms ?? 0) > 0 ? 'delayed' as const : 'live' as const;
  const delay = state === 'building' ? Math.max(0, (status?.buffer?.target_ms ?? status?.desired_delay_ms ?? 0) - (status?.buffer?.fill_ms ?? 0)) : status?.effective_delay_ms ?? 0;
  const visible = !!status && !status.overlay_hidden && status.is_connected && !['recovering', 'degraded', 'error'].includes(status.effective_mode) && !(status.mode === 'instant' && state === 'live' && !config.instantLive);
  return { state, delay, visible };
}
const bools = ['badge', 'popup', 'icon', 'circleIcon', 'iconMatchText', 'hideLive', 'instantLive', 'liveGreen', 'motion', 'overrideBadgeText'] as const;
export function overlayUrl(config: OverlayConfig, origin: string): string {
  const url = new URL('/overlay', origin);
  for (const [key, value] of Object.entries(config)) url.searchParams.set(key, String(value));
  return url.toString();
}
export function parseOverlay(params: URLSearchParams): OverlayConfig {
  const result = { ...defaultOverlay };
  const theme = params.get('theme'); if (themes.some(value => value === theme)) result.theme = theme as OverlayConfig['theme'];
  const color = params.get('color'); if (color && Object.hasOwn(colors, color)) result.color = color as OverlayConfig['color'];
  const position = params.get('position'); if (positions.some(value => value === position)) result.position = position as OverlayConfig['position'];
  const badgeSize = params.get('badgeSize'); if (['xs', 's', 'm', 'l'].includes(badgeSize ?? '')) result.badgeSize = badgeSize as OverlayConfig['badgeSize'];
  const popupSize = params.get('popupSize'); if (['s', 'm', 'l', 'xl'].includes(popupSize ?? '')) result.popupSize = popupSize as OverlayConfig['popupSize'];
  const radius = Number(params.get('radius') ?? 64); result.radius = Number.isFinite(radius) ? Math.min(100, Math.max(0, Math.round(radius))) : 64;
  result.text = (params.get('text') ?? '').trim().slice(0, 48); result.lang = params.get('lang') === 'en' ? 'en' : 'fr';
  for (const key of bools) { if (params.has(key)) result[key] = params.get(key) === 'true'; }
  if (!result.icon) { result.circleIcon = false; result.iconMatchText = false; }
  if (!result.text) result.overrideBadgeText = false;
  return result;
}
