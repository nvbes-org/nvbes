import type { DeviceGroup } from './AccountSessionsPage.device';
import { DeviceCard } from './AccountSessionsPage.deviceCard';
import { DeviceSection } from './AccountSessionsPage.section';

function DeviceList({
  devices,
  revoking,
  onRevoke,
  onRevokeDevice,
}: {
  devices: DeviceGroup[];
  revoking: string | null;
  onRevoke: (sessionId: string) => void;
  onRevokeDevice: (device: DeviceGroup) => void;
}) {
  return (
    <div className="flex flex-col gap-3">
      {devices.map((device) => (
        <DeviceCard
          key={device.id}
          device={device}
          revoking={revoking}
          onRevoke={onRevoke}
          onRevokeDevice={onRevokeDevice}
        />
      ))}
    </div>
  );
}

export function RecognizedDevicesSection({
  devices,
  revoking,
  onRevoke,
  onRevokeDevice,
}: {
  devices: DeviceGroup[];
  revoking: string | null;
  onRevoke: (sessionId: string) => void;
  onRevokeDevice: (device: DeviceGroup) => void;
}) {
  if (devices.length === 0) return null;

  return (
    <DeviceSection
      title="Appareils reconnus & fiables"
      description="Appareils authentifiés avec un haut niveau de confiance ou enregistrés comme fiables."
    >
      <DeviceList
        devices={devices}
        revoking={revoking}
        onRevoke={onRevoke}
        onRevokeDevice={onRevokeDevice}
      />
    </DeviceSection>
  );
}

export function OtherDevicesSection({
  devices,
  revoking,
  onRevoke,
  onRevokeDevice,
}: {
  devices: DeviceGroup[];
  revoking: string | null;
  onRevoke: (sessionId: string) => void;
  onRevokeDevice: (device: DeviceGroup) => void;
}) {
  if (devices.length === 0) return null;

  return (
    <DeviceSection
      title="Autres appareils (non reconnus)"
      description="Appareils récents ou non vérifiés ayant des sessions actives."
    >
      <DeviceList
        devices={devices}
        revoking={revoking}
        onRevoke={onRevoke}
        onRevokeDevice={onRevokeDevice}
      />
    </DeviceSection>
  );
}
