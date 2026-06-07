import { WorkerQueueStatusPageContent } from './WorkerQueueStatusPage.shared';
import { useWorkerQueueStatusPage } from './useWorkerQueueStatusPage';

export default function WorkerQueueStatusPage() {
  const page = useWorkerQueueStatusPage();
  return <WorkerQueueStatusPageContent {...page} />;
}
