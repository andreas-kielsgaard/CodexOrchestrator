import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { ApplicationRoot } from './app/ApplicationRoot';
import './styles.css';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ApplicationRoot />
  </StrictMode>,
);
