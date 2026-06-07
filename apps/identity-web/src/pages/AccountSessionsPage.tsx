import {
  CurrentSessionCard,
  EmptySessionsCard,
  OtherSessionsCard,
  SessionsSkeleton,
} from './AccountSessionsPage.shared';
import { useAccountSessionsPage } from './useAccountSessionsPage';

export default function AccountSessionsPage() {
  const {
    currentSession,
    isPending,
    listRef,
    otherSessions,
    revoking,
    sessions,
    totalSize,
    virtualItems,
    virtualizer,
    onRevoke,
    onRevokeOthers,
  } = useAccountSessionsPage();

  if (isPending) {
    return <SessionsSkeleton />;
  }

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <h1 className="text-xl font-heading font-semibold">Appareils & sessions</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Gerer vos sessions actives sur tous vos appareils.
        </p>
      </div>

      {currentSession && <CurrentSessionCard session={currentSession} />}

      {otherSessions.length > 0 && (
        <OtherSessionsCard
          sessions={otherSessions}
          listRef={listRef}
          totalSize={totalSize}
          virtualItems={virtualItems}
          measureElement={virtualizer.measureElement}
          revoking={revoking}
          onRevoke={onRevoke}
          onRevokeOthers={onRevokeOthers}
        />
      )}

      {sessions.length === 0 && <EmptySessionsCard />}
    </div>
  );
}
