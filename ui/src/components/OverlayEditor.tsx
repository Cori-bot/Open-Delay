import { useEffect, useRef, useState } from 'react';
import { Palette, SlidersHorizontal, Radio, Check, Copy, ArrowUpRight, Play, Square, Eye } from 'lucide-react';
import { api } from '../api';
import { colors, defaultOverlay, positions, themes, overlayUrl, overlayPresentation } from '../overlay';
import type { OverlayConfig } from '../overlay';
import type { OverlayControl, OverlayState } from '../overlayControl';
import type { Language, RouteConfig, Status } from '../types';
import { OverlayVisual } from '../OverlayRenderer';
import { SelectField } from './SelectField';
import { BrandMark } from './Brand';
import './OverlayEditor.css';

type Translate = (fr: string, en: string) => string;
type Props = { t: Translate; lang: Language; port: number; status: Status | null; routes: RouteConfig[]; onDirtyChange?: (dirty: boolean) => void };
type Section = 'appearance' | 'content' | 'connection';

export function OverlayEditor({ t, lang, port, status, routes, onDirtyChange }: Props) {
  const [hasEdits, setHasEdits] = useState(false);
  const [section, setSection] = useState<Section>('appearance');
  const [config, setConfig] = useState<OverlayConfig>({ ...defaultOverlay, lang });
  const [control, setControl] = useState<OverlayControl | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [connectionError, setConnectionError] = useState('');
  const mutation = useRef(0);
  const mutationPending = useRef(false);
  const [notice, setNotice] = useState('');
  const [preview, setPreview] = useState<OverlayState | 'actual'>('delayed');
  const active = { ...config, lang, text: config.text.trim() };
  const dirty = JSON.stringify(active) !== JSON.stringify(control?.config);
  useEffect(() => { onDirtyChange?.(hasEdits && dirty); }, [hasEdits, dirty, onDirtyChange]);
  const sourceUrl = port ? `http://127.0.0.1:${port}/overlay?managed=true` : '';
  const linked = !connectionError && !!control?.source_recent;
  const grouped = status?.routes.filter(route => route.is_connected && routes.some(config => config.id === route.id && config.include_in_group)) ?? [];
  const actual = overlayPresentation(status ? { ...status, buffer: { fill_ms: grouped.length ? Math.min(...grouped.map(route => route.buffer.fill_ms)) : 0, target_ms: Math.max(0, ...grouped.map(route => route.buffer.target_ms)) } } : null, active);
  const previewState = preview === 'actual' ? actual.state : preview;
  const previewUnavailable = preview === 'actual' && !actual.visible;

  useEffect(() => {
    const abort = new AbortController(); let cancelled = false; let timer: ReturnType<typeof setTimeout>;
    let first = true;
    async function refresh() {
      if (mutationPending.current) { timer = setTimeout(() => void refresh(), 1500); return; }
      const generation = mutation.current;
      try {
        const next = await api.overlayControl(abort.signal);
        if (!cancelled && generation === mutation.current) {
          setControl(next); setConnectionError('');
          if (first) { if (next.config) setConfig(next.config); setLoading(false); first = false; }
        }
      } catch (cause) { if (!cancelled && generation === mutation.current) setConnectionError(String(cause)); }
      if (!cancelled) timer = setTimeout(() => void refresh(), 1500);
    }
    void refresh();
    return () => { cancelled = true; abort.abort(); clearTimeout(timer); };
  }, []);

  const update = (patch: Partial<OverlayConfig>) => {
    setHasEdits(true);
    setNotice('');
    setConfig(old => {
      const next = { ...old, ...patch };
      if (!next.icon) { next.circleIcon = false; next.iconMatchText = false; }
      if (!next.text.trim()) next.overrideBadgeText = false;
      return next;
    });
  };
  const save = async () => {
    mutation.current++; mutationPending.current = true;
    setBusy(true); setError(''); setNotice('');
    try { const next = await api.saveOverlay(active); setControl(next); setNotice(t('Réglages appliqués à la source liée.', 'Settings applied to the linked source.')); }
    catch (cause) { setError(String(cause)); }
    finally { mutationPending.current = false; setBusy(false); }
  };
  const test = async (state: OverlayState | 'stop') => {
    mutation.current++; mutationPending.current = true;
    setBusy(true); setError(''); setNotice('');
    try { setControl(await api.testOverlay(state)); setNotice(state === 'stop' ? t('Test arrêté. La source reprend l’état réel.', 'Test stopped. The source resumes its actual state.') : t('Test envoyé à la source pour 5 secondes.', 'Test sent to the source for 5 seconds.')); }
    catch (cause) { setError(String(cause)); }
    finally { mutationPending.current = false; setBusy(false); }
  };
  const sections = [
    { id: 'appearance' as const, label: t('Apparence', 'Appearance'), detail: t('Thème, couleurs, position', 'Theme, colors, position'), Icon: Palette },
    { id: 'content' as const, label: t('Contenu et comportement', 'Content & behavior'), detail: t('Badge, texte, animations', 'Badge, text, motion'), Icon: SlidersHorizontal },
    { id: 'connection' as const, label: t('Connexion à OBS', 'Connect to OBS'), detail: t('Source et tests en direct', 'Source and live tests'), Icon: Radio },
  ];
  const toggle = (key: keyof Pick<OverlayConfig, 'badge' | 'popup' | 'icon' | 'circleIcon' | 'iconMatchText' | 'hideLive' | 'instantLive' | 'liveGreen' | 'motion' | 'overrideBadgeText'>, label: string, detail?: string, disabled = false) => <label className="toggle-row" key={key}><span><strong>{label}</strong>{detail && <small>{detail}</small>}</span><input type="checkbox" checked={config[key]} disabled={disabled || loading} onChange={event => update({ [key]: event.target.checked })}/><span className="toggle-track" aria-hidden="true"/></label>;

  return <div className="overlay-workspace">
    <div className="overlay-workspace-heading"><div><p className="eyebrow">{t('VOTRE IDENTITÉ À L’ÉCRAN', 'YOUR ON-SCREEN IDENTITY')}</p><h1>Overlay</h1><p>{t('Créez votre style. Vérifiez-le dans OBS. Gardez le contrôle.', 'Make it yours. Check it in OBS. Stay in control.')}</p></div><span className={`overlay-source-state ${linked ? 'is-active' : ''}`}><span className="status-dot"/>{linked ? t('Source active récemment', 'Source recently active') : t('Source non détectée', 'Source not detected')}</span></div>
    <div className="overlay-workspace-grid">
      <div className="overlay-settings-shell">
        <nav className="overlay-settings-nav" aria-label={t('Configuration de l’overlay', 'Overlay settings')}>{sections.map(item => <button key={item.id} aria-current={section === item.id ? 'true' : undefined} onClick={() => setSection(item.id)}><item.Icon size={18}/><span><strong>{item.label}</strong><small>{item.detail}</small></span><ArrowUpRight size={14}/></button>)}</nav>
        <section className="overlay-settings-pane" aria-label={sections.find(item => item.id === section)?.label}>
          {section === 'appearance' && <><div className="overlay-section-title"><span>01</span><div><h2>{t('Un style qui vous ressemble', 'A style of your own')}</h2><p>{t('Les changements sont visibles immédiatement dans l’aperçu.', 'Changes appear immediately in the preview.')}</p></div></div>
            <fieldset className="overlay-theme-fieldset" disabled={loading}><legend>{t('Thème', 'Theme')}</legend><div className="theme-picker">{themes.map((theme, index) => <button key={theme} aria-pressed={config.theme === theme} className={config.theme === theme ? 'selected' : ''} onClick={() => update({ theme })}><span className={`theme-swatch swatch-${theme}`}><BrandMark size={18}/></span>{['Minimal', 'Studio', 'SaaS', 'Signal', 'Dark Glass', 'Light Glass'][index]}</button>)}</div></fieldset>
            <fieldset className="color-picker" disabled={loading}><legend>{t('Couleur d’accent', 'Accent color')}</legend>{Object.entries(colors).map(([key, value]) => <button key={key} aria-label={key} title={key} aria-pressed={config.color === key} className={config.color === key ? 'selected' : ''} style={{ background: value }} onClick={() => update({ color: key as OverlayConfig['color'] })}>{config.color === key && <Check size={14}/>}</button>)}</fieldset>
            <label>{t('Position à l’écran', 'Screen position')}<SelectField disabled={loading} label={t('Position à l’écran', 'Screen position')} value={config.position} onChange={position => update({ position: position as OverlayConfig['position'] })} options={positions.map((value, index) => ({ value, label: t(['Haut gauche', 'Haut centre', 'Haut droite', 'Bas gauche', 'Bas centre', 'Bas droite'][index], ['Top left', 'Top center', 'Top right', 'Bottom left', 'Bottom center', 'Bottom right'][index]) }))}/></label>
            <div className="form-columns">{(['badgeSize', 'popupSize'] as const).map(key => <label key={key}>{key === 'badgeSize' ? t('Taille du badge', 'Badge size') : t('Taille de la popup', 'Popup size')}<SelectField disabled={loading} label={key === 'badgeSize' ? t('Taille du badge', 'Badge size') : t('Taille de la popup', 'Popup size')} value={config[key]} options={(key === 'badgeSize' ? ['xs', 's', 'm', 'l'] : ['s', 'm', 'l', 'xl']).map(value => ({ value, label: value.toUpperCase() }))} onChange={value => update({ [key]: value })}/></label>)}</div>
            <label>{t('Coins arrondis', 'Corner radius')}<span className="range-label">{config.radius}%</span><input disabled={loading} type="range" min="0" max="100" value={config.radius} onChange={event => update({ radius: Number(event.target.value) })}/></label>
          </>}
          {section === 'content' && <><div className="overlay-section-title"><span>02</span><div><h2>{t('Le bon message, au bon moment', 'The right message, at the right time')}</h2><p>{t('Choisissez ce que votre public voit.', 'Choose what your audience sees.')}</p></div></div><label>{t('Texte personnalisé', 'Custom text')}<input disabled={loading} maxLength={48} value={config.text} placeholder={t('Votre message, en 48 caractères', 'Your message, in 48 characters')} onChange={event => update({ text: event.target.value })}/></label>
            {toggle('badge', t('Afficher le badge', 'Show badge'))}{toggle('popup', t('Popups de transition', 'Transition popups'), t('Un message à chaque changement en mode Overlay.', 'A message at each change in Overlay mode.'))}{toggle('overrideBadgeText', t('Utiliser mon texte dans le badge', 'Use my text in the badge'), undefined, !config.text.trim())}{toggle('icon', t('Afficher l’icône', 'Show icon'))}{toggle('circleIcon', t('Icône dans un cercle', 'Circle icon'), undefined, !config.icon)}{toggle('iconMatchText', t('Icône de la couleur du texte', 'Match icon to text'), undefined, !config.icon)}{toggle('hideLive', t('Masquer le badge en direct', 'Hide live badge'))}{toggle('instantLive', t('Badge live en mode Instant', 'Live badge in Instant mode'))}{toggle('liveGreen', t('Live en vert', 'Green live state'))}{toggle('motion', t('Animations', 'Animations'))}
          </>}
          {section === 'connection' && <><div className="overlay-section-title"><span>03</span><div><h2>{t('Une seule URL. Toujours à jour.', 'One URL. Always up to date.')}</h2><p>{t('Ajoutez cette source une fois dans votre scène.', 'Add this source to your scene once.')}</p></div></div><ol className="overlay-connect-steps"><li>{t('Dans OBS : Sources → + → Navigateur.', 'In OBS: Sources → + → Browser.')}</li><li>{t('Collez l’URL ci-dessous. Dimensions : 1920 × 1080.', 'Paste the URL below. Size: 1920 × 1080.')}</li><li>{t('Appliquez vos réglages ici, puis lancez un test.', 'Apply your settings here, then run a test.')}</li></ol><div className="overlay-source-url"><code>{sourceUrl || t('En attente du moteur…', 'Waiting for the engine…')}</code><button className="button secondary" disabled={!sourceUrl} onClick={() => { void navigator.clipboard.writeText(sourceUrl).then(() => setNotice(t('URL copiée.', 'URL copied.'))).catch(cause => setError(String(cause))); }}><Copy size={15}/>{t('Copier l’URL', 'Copy URL')}</button></div><p className="muted">{t('Remplacez une ancienne URL personnalisée par celle-ci pour recevoir les réglages et les tests en direct.', 'Replace an old customized URL with this one to receive live settings and tests.')}</p><p className="muted">{t('L’activité indique qu’une source Navigateur lit le moteur. Elle ne confirme pas qu’OBS diffuse cette scène.', 'Activity means a Browser source is reading the engine. It does not confirm that OBS is broadcasting this scene.')}</p><details className="overlay-legacy"><summary>{t('URL autonome avec réglages intégrés', 'Standalone URL with embedded settings')}</summary><code>{port ? overlayUrl(active, `http://127.0.0.1:${port}`) : '—'}</code><p>{t('Cette URL conserve son propre style et ne reçoit pas les tests.', 'This URL keeps its own style and does not receive tests.')}</p></details></>}
        </section>
        <div className="overlay-apply-bar"><span>{loading ? t('Chargement…', 'Loading…') : dirty ? t('Modifications non appliquées', 'Unapplied changes') : t('Réglages appliqués', 'Settings applied')}</span><button className="button primary" disabled={loading || busy || !dirty || !control} onClick={() => void save()}><Check size={16}/>{busy ? t('Envoi…', 'Sending…') : t('Appliquer à la source', 'Apply to source')}</button></div>
      </div>
      <div className="overlay-preview-studio"><div className="overlay-preview-topline"><span><Eye size={15}/>{t('APERÇU LOCAL', 'LOCAL PREVIEW')}</span><span>1920 × 1080</span></div><div className="overlay-preview"><div className="overlay-preview-grid" aria-hidden="true"><BrandMark size={50}/><span>{t('Votre scène', 'Your scene')}</span></div><div className="scaled-overlay">{!previewUnavailable && <OverlayVisual config={active} state={previewState} delay={preview === 'actual' ? actual.delay : 15000} popup={preview !== 'actual'}/>}</div></div>
        <div className="overlay-preview-modes" role="group" aria-label={t('État de l’aperçu', 'Preview state')}>{(['live', 'building', 'delayed', 'actual'] as const).map(value => <button key={value} aria-pressed={preview === value} onClick={() => setPreview(value)}>{value === 'live' ? 'Live' : value === 'building' ? t('Préparation', 'Building') : value === 'delayed' ? t('Retardé', 'Delayed') : t('État réel', 'Actual state')}</button>)}</div><p className="preview-caption">{preview === 'actual' ? previewUnavailable ? t('Aucun overlay visible dans cet état réel.', 'No overlay is visible in the current actual state.') : t('État du moteur avec vos réglages de travail.', 'Engine state with your draft settings.') : t('Simulation locale avec vos réglages de travail.', 'Local simulation with your draft settings.')}</p>
        <section className="overlay-live-tests" aria-labelledby="overlay-tests-title"><div><Radio size={18}/><h2 id="overlay-tests-title">{t('Tester dans OBS', 'Test in OBS')}</h2><span>5 s</span></div><p>{t('Ces tests sont visibles dans la source liée, y compris à l’antenne. Ils ne changent pas le délai du stream.', 'These tests are visible in the linked source, including on air. They do not change the stream delay.')}</p>{dirty && <p className="overlay-test-hint">{t('Appliquez d’abord vos réglages pour tester ce style.', 'Apply your settings first to test this style.')}</p>}<div className="overlay-test-buttons">{(['live', 'building', 'delayed'] as const).map(value => <button className="button secondary" key={value} disabled={busy || dirty || !control} onClick={() => void test(value)}><Play size={13}/>{value === 'live' ? t('Test direct', 'Test live') : value === 'building' ? t('Test préparation', 'Test building') : t('Test délai', 'Test delay')}</button>)}</div><button className="text-button" disabled={busy || !control?.test} onClick={() => void test('stop')}><Square size={12}/>{t('Arrêter le test', 'Stop test')}</button>{!linked && <button className="text-button" onClick={() => setSection('connection')}>{t('Relier ma source OBS', 'Link my OBS source')}<ArrowUpRight size={14}/></button>}</section>
      </div>
    </div>
    {(error || connectionError) && <p className="field-error overlay-feedback" role="alert">{error ? t('Action non effectuée. Réessayez.', 'Action not completed. Try again.') : t('Liaison au moteur indisponible. Nouvelle tentative automatique…', 'Engine connection unavailable. Retrying automatically…')} <small>{error || connectionError}</small></p>}{notice && <p className="overlay-feedback" role="status">{notice}</p>}
  </div>;
}
