import type { DeviceGroup } from './AccountSessionsPage.device';
import { DeviceCard } from './AccountSessionsPage.deviceCard';

export function CurrentSessionCard({
  device,
  revoking,
  onRevoke,
}: {
  device: DeviceGroup;
  revoking: string | null;
  onRevoke: (sessionId: string) => void;
}) {
  return (
    <div className="flex flex-col gap-2">
      <div>
        <h2 className="text-lg font-semibold tracking-tight">Appareil actuel</h2>
        <p className="text-xs text-muted-foreground">
          L&apos;appareil et navigateur que vous utilisez actuellement.
        </p>
      </div>
      <DeviceCard device={device} revoking={revoking} onRevoke={onRevoke} />
    </div>
  );
}
