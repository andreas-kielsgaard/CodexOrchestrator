import { createRoot } from 'react-dom/client';
import { AgentSessionHarness } from './AgentSessionHarness';
import './harness.css';

// StrictMode is intentionally omitted here so one-step controls remain deterministic while
// manually inspecting subscription, reload, and remount behavior in the development harness.
createRoot(document.getElementById('root')!).render(<AgentSessionHarness />);
