import { useEffect, useRef, useState } from 'react';
import type { CSSProperties } from 'react';
import { colors, parseOverlay, overlayPresentation } from './overlay';
import type { OverlayConfig } from './overlay';
import { parseOverlayStatus } from './validation';
import type { OverlayStatus } from './validation';
import { BrandMark } from './components/Brand';
import { parseOverlayControl } from './overlayControl';
import type { OverlayControl } from './overlayControl';
export function OverlayVisual({ config, state, delay = 15000, popup = false }: { config: OverlayConfig; state: 'live' | 'building' | 'delayed'; delay?: number; popup?: boolean }) {
  const t = (fr: string, en: string) => config.lang === 'fr' ? fr : en;
  const label = state === 'live' ? 'LIVE' : state === 'building' ? t('PRÉPARATION', 'PREPARING') : t('DÉLAI ACTIF', 'DELAY ACTIVE');
  const title = state === 'live' ? t('De retour en direct', 'Back to live') : state === 'building' ? t('Un instant de recul', 'A moment to rewind') : t('Le temps est de votre côté', 'Time is on your side');
  const style = { '--overlay-accent': state === 'live' && config.liveGreen ? colors.green : colors[config.color], '--badge-radius': config.radius >= 94 ? '999px' : `${config.radius * .19}px`, '--popup-radius': `${config.radius * .34}px` } as CSSProperties;
  return <div style={style} className={`overlay-canvas theme-${config.theme} position-${config.position} badge-${config.badgeSize} popup-${config.popupSize} ${config.motion ? '' : 'no-motion'}`}>
    {config.badge && !(state === 'live' && config.hideLive) && <div className="overlay-badge">{config.icon && <span aria-hidden="true" className={`overlay-symbol ${config.circleIcon ? 'circle' : ''} ${config.iconMatchText ? 'match' : ''}`}><BrandMark size={22}/></span>}<span>{config.overrideBadgeText && config.text ? config.text : label}</span>{state !== 'live' && <strong>{Math.round(delay / 1000)}<small>s</small></strong>}</div>}
    {popup && config.popup && <div className="overlay-popup"><div className="overlay-popup-mark" aria-hidden="true"><BrandMark size={48}/></div><p>{label}</p><h2>{title}</h2><div>{config.text || (state === 'building' ? t('Le direct continue pendant la préparation.', 'The stream stays live while preparing.') : state === 'delayed' ? t('Le stream est maintenant retardé.', 'The stream is now delayed.') : t('Vous voyez à nouveau le direct.', 'You are watching live again.'))}</div>{state !== 'live' && <strong>{Math.round(delay / 1000)}<small> s</small></strong>}</div>}
  </div>;
}
export function OverlayPage() {
  const [urlConfig] = useState(() => parseOverlay(new URLSearchParams(location.search)));
  const [managed] = useState(() => new URLSearchParams(location.search).get('managed') === 'true');
  const [viewer] = useState(() => crypto.randomUUID());
  const [control, setControl] = useState<OverlayControl | null>(null);
  const config = managed && control?.config ? control.config : urlConfig;
  const [status, setStatus] = useState<OverlayStatus | null>(null);
  const [popup, setPopup] = useState(false);
  const previous = useRef<string | null>(null);
  useEffect(() => {
    if (!control?.test) return;
    const id = control.test.id;
    const timer = setTimeout(() => setControl(old => old?.test?.id === id ? { ...old, test: null } : old), Math.max(0, control.test.expires_at_ms - Date.now()));
    return () => clearTimeout(timer);
  }, [control?.test]);
  useEffect(() => {
    document.documentElement.lang = config.lang; document.body.classList.add('transparent');
    let cancelled = false; let timer: ReturnType<typeof setTimeout>; let fade: ReturnType<typeof setTimeout>;
    const controller = new AbortController();
    async function tick() {
      try {
        const response = await fetch(`/api/v1/overlay-status${managed ? `?viewer=${viewer}` : ''}`, { signal: AbortSignal.any([controller.signal, AbortSignal.timeout(1500)]), cache: 'no-store' });
        if (!response.ok) throw new Error('Overlay unavailable');
        const payload: unknown = await response.json();
        const next = parseOverlayStatus(payload);
        const nextControl = managed ? parseOverlayControl(payload) : null;
        if (!cancelled) {
          setStatus(next);
          setControl(nextControl);
          if (next.overlay_hidden) { previous.current = null; setPopup(false); }
          else if (next.mode !== 'instant' && previous.current !== null && previous.current !== next.effective_mode) { setPopup(true); clearTimeout(fade); fade = setTimeout(() => setPopup(false), 3500); }
          previous.current = next.overlay_hidden ? null : next.effective_mode;
        }
      } catch { if (!cancelled) { setStatus(null); setControl(null); previous.current = null; setPopup(false); } }
      if (!cancelled) timer = setTimeout(tick, 500);
    }
    void tick();
    return () => { cancelled = true; controller.abort(); clearTimeout(timer); clearTimeout(fade); document.body.classList.remove('transparent'); };
  }, [config.lang, managed, viewer]);
  if (managed && control?.config && control.test) return <OverlayVisual key={control.test.id} config={config} state={control.test.state} delay={control.test.delay_ms} popup={true}/>;
  const presentation = overlayPresentation(status, config);
  if (!presentation.visible) return null;
  return <OverlayVisual config={config} state={presentation.state} delay={presentation.delay} popup={popup}/>;
}
