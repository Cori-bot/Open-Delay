import { useId, useState } from 'react';
import { Clock3, Radio, Repeat2, Activity } from 'lucide-react';
import { BrandMark } from './Brand';
import './StreamDeckPreview.css';

type Translate = (fr: string, en: string) => string;

/** Local, illustrative preview. Selecting a key never sends an engine command. */
export function StreamDeckPreview({ t }: { t: Translate }) {
  const [selected, setSelected] = useState(0);
  const [perspective, setPerspective] = useState(true);
  const detailId = useId();
  const actions = [
    { name: 'Set Delay', label: t('Délai', 'Delay'), Icon: Clock3, title: t('Choisir un délai', 'Choose a delay'), detail: t('Dans Stream Deck, réglez la durée en secondes et la destination. La touche demande ce délai au moteur ; vérifiez ensuite le délai effectif dans la régie.', 'In Stream Deck, set the duration in seconds and the output. The key requests this delay from the engine; then check the effective delay in the dashboard.') },
    { name: 'Go Live', label: t('Direct', 'Live'), Icon: Radio, title: t('Revenir au direct', 'Return to live'), detail: t('Demande un délai de 0 seconde et annule une préparation en cours. Vérifiez le retour au direct dans la régie. Cette commande ne remplace pas un arrêt d’urgence.', 'Requests a delay of 0 seconds and cancels a pending preparation. Check the return to live in the dashboard. This command does not replace an emergency stop.') },
    { name: 'Toggle', label: t('Basculer', 'Toggle'), Icon: Repeat2, title: t('Basculer le délai', 'Toggle delay'), detail: t('Configurez le délai de bascule dans les propriétés de la touche. Alternez entre ce délai et le direct pour la destination choisie.', 'Configure the toggle delay in the key properties. Switch between that delay and live for the selected output.') },
    { name: 'Status', label: t('État', 'Status'), Icon: Activity, title: t('Lire l’état de la sortie', 'Read output status'), detail: t('Affiche l’état renvoyé par le moteur sur une touche. Consultez le délai effectif et les éventuelles erreurs ; cette action ne modifie pas le stream.', 'Displays the engine state on a key. Check the effective delay and any errors; this action does not change the stream.') },
  ];
  const action = actions[selected];

  return <section className="deck-preview" aria-label={t('Découvrir les actions Stream Deck', 'Explore Stream Deck actions')}>
    <div className="deck-preview-toolbar"><span className="deck-demo-badge">{t('APERÇU INTERACTIF', 'INTERACTIVE PREVIEW')}</span><button type="button" className="deck-view-toggle" aria-pressed={perspective} onClick={() => setPerspective(value => !value)}>{perspective ? t('Vue de face', 'Front view') : t('Vue en relief', 'Perspective view')}</button></div>
    <div className={`deck-preview-scene ${perspective ? 'is-perspective' : ''}`}>
      <div className="deck-preview-device">
        <div className="deck-preview-brand"><BrandMark size={15}/><span>open-delay</span></div>
        <div className="deck-preview-keys" role="group" aria-label={t('Touches de démonstration', 'Demo keys')}>
          {Array.from({ length: 15 }, (_, index) => {
            const item = actions[index];
            return <span className="deck-preview-slot" key={index}>{item ? <button type="button" className="deck-preview-key" aria-pressed={selected === index} aria-label={`${item.name} — ${item.title}`} aria-controls={detailId} onClick={() => setSelected(index)}><item.Icon size={23} strokeWidth={1.6} aria-hidden="true"/><span>{item.label}</span></button> : <span className="deck-preview-key is-empty" aria-hidden="true"/>}</span>;
          })}
        </div>
      </div>
    </div>
    <p className="deck-preview-hint">{t('Sélectionnez une touche pour comprendre son rôle.', 'Select a key to learn what it does.')}</p>
    <div id={detailId} className="deck-action-detail" role="status" aria-live="polite" aria-atomic="true"><span>{action.name}</span><h2>{action.title}</h2><p>{action.detail}</p></div>
    <p className="deck-preview-disclaimer">{t('Démonstration locale. Aucune commande envoyée, aucun matériel connecté détecté ici.', 'Local demonstration. No commands sent; connected hardware is not detected here.')}</p>
  </section>;
}
