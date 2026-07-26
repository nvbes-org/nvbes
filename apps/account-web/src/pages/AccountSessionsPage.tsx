import { Button } from '@/components/ui/button';
import {
  CurrentSessionCard,
  EmptySessionsCard,
  OtherDevicesSection,
  RecognizedDevicesSection,
  SessionsSkeleton,
} from './AccountSessionsPage.shared';
import { useAccountSessionsPage } from './useAccountSessionsPage';

export default function AccountSessionsPage() {
  const {
    currentDevice,
    isPending,
    otherDevices,
    recognizedDevices,
    revoking,
    sessions,
    onRevoke,
    onRevokeDevice,
    onRevokeOthers,
  } = useAccountSessionsPage();

  if (isPending) {
    return <SessionsSkeleton />;
  }

  const hasOtherDevices = recognizedDevices.length > 0 || otherDevices.length > 0;

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-3xl font-heading font-semibold">Appareils & sessions</h1>
          <p className="text-sm text-muted-foreground mt-1">
            Gérez et révoquez vos sessions actives groupées par appareil.
          </p>
        </div>

        {hasOtherDevices && (
          <Button variant="outline" size="sm" onClick={onRevokeOthers} className="shrink-0">
            Déconnecter les autres
          </Button>
        )}
      </div>

      {currentDevice && (
        <CurrentSessionCard device={currentDevice} revoking={revoking} onRevoke={onRevoke} />
      )}

      {recognizedDevices.length > 0 && (
        <RecognizedDevicesSection
          devices={recognizedDevices}
          revoking={revoking}
          onRevoke={onRevoke}
          onRevokeDevice={onRevokeDevice}
        />
      )}

      {otherDevices.length > 0 && (
        <OtherDevicesSection
          devices={otherDevices}
          revoking={revoking}
          onRevoke={onRevoke}
          onRevokeDevice={onRevokeDevice}
        />
      )}

      {sessions.length === 0 && <EmptySessionsCard />}
    </div>
  );
}
