import { useEffect, useState } from 'react';
import { useAccountContext } from '@/hooks/useAccountContext';
import { type SecurityEvent, listSecurityEvents } from './AccountAuditsPage.api';
import { AuditsSkeleton, GdprExportsCard, SecurityJournalCard } from './AccountAuditsPage.sections';

export default function AccountAuditsPage() {
  const [events, setEvents] = useState<SecurityEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const { me } = useAccountContext();

  useEffect(() => {
    const fetchEvents = async () => {
      if (me?.current_workspace_id) {
        try {
          const result = await listSecurityEvents(me.current_workspace_id, 50);
          setEvents(result.events);
        } catch {
          /* user may not have permission */
        }
      }
      setLoading(false);
    };
    void fetchEvents();
  }, [me?.current_workspace_id]);

  if (loading) return <AuditsSkeleton />;

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Audits & RGPD</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Journaux d&apos;audit, rapports de conformite et exports RGPD.
        </p>
      </div>

      <SecurityJournalCard me={me} events={events} />
      <GdprExportsCard />
    </div>
  );
}
