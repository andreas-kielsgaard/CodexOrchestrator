import { getCurrent, onOpenUrl } from '@tauri-apps/plugin-deep-link';
import {
  parseSessionDeepLink,
  type SessionDeepLinkSource,
} from '../../application/agentSessions/deepLinks';
export const tauriSessionDeepLinks: SessionDeepLinkSource = {
  async subscribe(listener) {
    const receive = (urls: string[]) =>
      urls.forEach((url) => {
        const id = parseSessionDeepLink(url);
        if (id) listener(id);
      });
    const stop = await onOpenUrl(receive);
    try {
      receive((await getCurrent()) ?? []);
    } catch (error) {
      stop();
      throw error;
    }
    return stop;
  },
};
