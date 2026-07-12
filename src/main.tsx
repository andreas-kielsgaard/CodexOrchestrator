import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './app/App';
import { tauriAgentSessionClient } from './infrastructure/agentSessions/tauriAgentSessionClient';
import './styles.css';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App agentSessionClient={tauriAgentSessionClient} />
  </StrictMode>,
);
