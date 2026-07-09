import type { OpenTasksFeatureControllerOptions } from './controllers/useOpenTasksFeatureController';
import { useOpenTasksFeatureController } from './controllers/useOpenTasksFeatureController';
import { OpenTasksScreen } from './views/OpenTasksScreen';

export type OpenTasksPageProps = OpenTasksFeatureControllerOptions;

export function OpenTasksPage(props: OpenTasksPageProps) {
  const feature = useOpenTasksFeatureController(props);

  return <OpenTasksScreen view={feature.view} actions={feature.actions} />;
}
