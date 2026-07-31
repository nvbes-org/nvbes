import type { DeviceGroup } from './IdentitySessionsPage.device';
import { DeviceCard } from './IdentitySessionsPage.deviceCard';
import { DeviceSection } from './IdentitySessionsPage.section';

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
