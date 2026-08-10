import { invoke } from '@tauri-apps/api/core';
import type {
  EpicOriginProject,
  EpicOriginProjectClient,
} from '../../application/epicOriginProject';

export const tauriEpicOriginProjectClient: EpicOriginProjectClient = {
  chooseProject: () => invoke<EpicOriginProject | null>('choose_epic_origin_project'),
  inspectProject: (path) =>
    invoke<EpicOriginProject>('inspect_epic_origin_project', { input: { path } }),
};
