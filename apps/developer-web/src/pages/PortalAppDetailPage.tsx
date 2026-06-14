import { Link } from '@tanstack/react-router';

import { PageHeader } from './Portal.shared';

export function PortalAppDetailPage() {
  return (
    <section>
      <PageHeader
        title="App detail"
        body="Detailed redirect editing and per-client delivery history will build on the app management API."
      />
      <Link to="/portal/apps" className="text-sm font-medium text-primary">
        Back to apps
      </Link>
    </section>
  );
}
