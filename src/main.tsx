import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { ApplicationRoot } from './app/ApplicationRoot';
import { OperationalSpineCheckpointDemo } from './dev/demonstration/OperationalSpineCheckpointDemo';
import './styles.css';

const requestedSurface = new URLSearchParams(window.location.search);

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    {requestedSurface.has('demo-operational-spine') ? (
      <OperationalSpineCheckpointDemo />
    ) : (
      <ApplicationRoot />
    )}
  </StrictMode>,
);
