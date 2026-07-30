import type { DeviceGroup } from './AccountSessionsPage.device';
import { DeviceCard } from './AccountSessionsPage.deviceCard';
import { DeviceSection } from './AccountSessionsPage.section';

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
    <DeviceSection
      title="Appareil actuel"
      description="L'appareil et navigateur que vous utilisez actuellement."
      className="gap-2"
    >
      <DeviceCard device={device} revoking={revoking} onRevoke={onRevoke} />
    </DeviceSection>
  );
}
