import type { RouteConfig, RouteStatus } from '../types';
import { AppIcon } from './AppIcon';

type Props = {
  routes: RouteConfig[];
  runtime: RouteStatus[];
  available: boolean;
  t: (fr: string, en: string) => string;
  configure: () => void;
  connect: () => void;
};

export function QuickStart({ routes, runtime, available, t, configure, connect }: Props) {
  const enabled = routes.filter(route => route.enabled && route.include_in_group);
  const connected = runtime.some(state => state.is_connected && enabled.some(route => route.id === state.id));
  if (connected) return null;
  const step = enabled.length ? 1 : 0;
  return <section className="quick-start" aria-labelledby="quick-start-title">
    <div className="quick-start-heading"><span className="quick-start-mark"><AppIcon name="setup" size={22}/></span><div><p className="eyebrow">{t('VOTRE PREMIER SIGNAL', 'YOUR FIRST SIGNAL')}</p><h2 id="quick-start-title">{!available ? t('Connexion au moteur…', 'Connecting to the engine…') : step ? t('Reliez maintenant votre logiciel de stream', 'Now connect your streaming software') : t('Votre régie commence ici', 'Your studio starts here')}</h2></div></div>
    <p>{step ? t('Copiez votre adresse locale dans OBS ou Streamlabs, gardez votre clé habituelle, puis démarrez la sortie. Son état apparaîtra ici.', 'Copy your local address into OBS or Streamlabs, keep your usual key, then start the output. Its state will appear here.') : t('Configurez une destination et incluez-la dans les commandes du groupe. Reliez ensuite OBS pour contrôler son délai ici.', 'Configure an output and include it in group controls. Then connect OBS to control its delay here.')}</p>
    <ol className="quick-start-steps" aria-label={t('Parcours de configuration', 'Setup journey')}>{[t('Choisir une destination', 'Choose an output'), t('Relier OBS / Streamlabs', 'Connect OBS / Streamlabs'), t('Recevoir le signal', 'Receive the signal')].map((label, index) => <li key={label} aria-current={index === step ? 'step' : undefined} data-complete={index < step}><span>{index < step ? <AppIcon name="check" size={14}/> : index + 1}</span>{label}</li>)}</ol>
    <button className="button primary" disabled={!available} onClick={step ? connect : configure}>{step ? t('Voir mon adresse OBS', 'View my OBS address') : t('Configurer ma destination', 'Set up my output')}<AppIcon name="arrow" size={16}/></button>
  </section>;
}
