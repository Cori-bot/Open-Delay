import React from 'react';
import ReactDOM from 'react-dom/client';
import { App } from './App';
import { OverlayPage } from './OverlayRenderer';
import { ErrorBoundary } from './ErrorBoundary';
import './studio.css';
import './support.css';
import './overlay.css';
import '@fontsource-variable/geist';
const root = document.getElementById('root');
// Keep the OBS document transparent even if its renderer unmounts on an error.
if (location.pathname === '/overlay') document.documentElement.classList.add('overlay-document');
if (!root) throw new Error('Open-Delay root element is missing');
ReactDOM.createRoot(root).render(<React.StrictMode><ErrorBoundary>{location.pathname === '/overlay' ? <OverlayPage/> : <App/>}</ErrorBoundary></React.StrictMode>);
