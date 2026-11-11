import { useState } from 'react';
import { Download, RefreshCw, ShieldCheck } from 'lucide-react';
import { isDesktop } from '../api';
import type { useUpdates } from '../useUpdates';
import { Modal } from './Modal';
import './UpdatePanel.css';

export function UpdatePanel({ updates, active, engineReady, t }: { updates: ReturnType<typeof useUpdates>; active: boolean; engineReady: boolean; t: (fr: string, en: string) => string }) {
  const [confirm, setConfirm] = useState(false);
  const { status, error } = updates;
  const busy = status?.phase === 'checking' || status?.phase === 'downloading' || status?.phase === 'installing';
  const available = status?.phase === 'available';
  const messages = {
    idle: t('Vérifiez si une nouvelle version est disponible.', 'Check whether a new version is available.'),
    checking: t('Recherche de mises à jour…', 'Checking for updates…'),
    available: t(`La version ${status?.version ?? ''} est disponible.`, `Version ${status?.version ?? ''} is available.`),
    up_to_date: t('Vous utilisez la dernière version publiée.', 'You are using the latest published version.'),
    downloading: t('Téléchargement et vérification de la signature…', 'Downloading and verifying the signature…'),
    installing: t('Installation en cours. L’application va redémarrer.', 'Installing. The application will restart.'),
    error: t('La mise à jour n’a pas pu aboutir. Vous pouvez réessayer.', 'The update could not be completed. You can try again.'),
  };
  return <section className="panel update-panel" aria-labelledby="updates-title"><div className="update-panel-heading"><ShieldCheck size={20}/><div><h2 id="updates-title">{t('Mises à jour', 'Updates')}</h2><span>{t('Version installée', 'Installed version')} {status?.current_version ?? '0.2.0'}</span></div></div>
    {!isDesktop() ? <p>{t('Les mises à jour sont disponibles dans l’application Windows.', 'Updates are available in the Windows application.')}</p> : <>
      <p role="status">{!status ? t('Connexion au service de mise à jour…', 'Connecting to the updater…') : !status.configured ? t('Les mises à jour ne sont pas configurées pour cette version.', 'Updates are not configured for this build.') : messages[status.phase]}</p>
      {(error || status?.error) && <p className="field-error" role="alert">{error || status?.error}</p>}
      {status?.phase === 'downloading' && <div className="update-progress"><progress aria-label={t('Téléchargement de la mise à jour', 'Update download')} {...(status.total ? { value: Math.min(status.downloaded, status.total), max: status.total } : {})}/><span>{Math.round(status.downloaded / 1024)} KiB{status.total ? ` / ${Math.round(status.total / 1024)} KiB` : ''}</span></div>}
      {available && status?.notes && <details><summary>{t('Nouveautés de cette version', 'What’s new in this version')}</summary><p className="update-notes">{status.notes}</p></details>}
      {!engineReady && <p className="update-stream-note">{t('Vérification de l’état des sorties…', 'Checking output state…')}</p>}{active && <p className="update-stream-note">{t('Arrêtez toutes vos sorties avant d’installer une mise à jour.', 'Stop all outputs before installing an update.')}</p>}
      <div className="update-actions"><button type="button" className="button secondary" disabled={busy || !status?.configured} onClick={() => void updates.check()}><RefreshCw size={15}/>{t('Vérifier les mises à jour', 'Check for updates')}</button>{available && <button type="button" className="button primary" disabled={active || !engineReady || busy} onClick={() => setConfirm(true)}><Download size={16}/>{t('Télécharger et installer', 'Download and install')}</button>}</div>
      <p className="update-footnote">{t('Recherche au démarrage. Installation à votre demande, avec vérification cryptographique du paquet.', 'Checked at startup. Installed at your request, with cryptographic package verification.')}</p>
    </>}
    {confirm && <Modal title={t('Installer la mise à jour', 'Install update')} close={() => setConfirm(false)}><p>{t(`Installer Open-Delay ${status?.version ?? ''} ? L’application va se fermer puis redémarrer. Vos réglages sont conservés.`, `Install Open-Delay ${status?.version ?? ''}? The app will close and restart. Your settings are preserved.`)}</p>{active && <p className="field-error">{t('Une sortie est active. Arrêtez-la avant de continuer.', 'An output is active. Stop it before continuing.')}</p>}<div className="dialog-actions"><button className="button quiet" onClick={() => setConfirm(false)}>{t('Plus tard', 'Later')}</button><button className="button primary" disabled={active || !engineReady || busy || !available} onClick={() => { setConfirm(false); void updates.install(); }}>{t('Installer et redémarrer', 'Install and restart')}</button></div></Modal>}
  </section>;
}
