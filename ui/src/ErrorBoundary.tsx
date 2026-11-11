import { Component } from 'react';
import type { ReactNode } from 'react';

export class ErrorBoundary extends Component<{ children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  render() {
    if (this.state.failed) {
      // Never leave an outdated protection badge visible if the OBS source fails.
      if (location.pathname === '/overlay') return null;
      return <main className="fatal-error" role="alert"><h1>Open-Delay</h1><p>L’interface a rencontré une erreur. Le moteur continue de fonctionner. / The interface encountered an error. The engine keeps running.</p><button type="button" onClick={() => location.reload()}>Recharger l’interface / Reload interface</button></main>;
    }
    return this.props.children;
  }
}
