import type { DeviceGroup } from './AccountSessionsPage.device';
import { DeviceCard } from './AccountSessionsPage.deviceCard';

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
    <div className="flex flex-col gap-3">
      <div>
        <h2 className="text-lg font-semibold tracking-tight">Appareils reconnus & fiables</h2>
        <p className="text-xs text-muted-foreground">
          Appareils authentifiés avec un haut niveau de confiance ou enregistrés comme fiables.
        </p>
      </div>
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
    </div>
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
    <div className="flex flex-col gap-3">
      <div>
        <h2 className="text-lg font-semibold tracking-tight">Autres appareils (non reconnus)</h2>
        <p className="text-xs text-muted-foreground">
          Appareils récents ou non vérifiés ayant des sessions actives.
        </p>
      </div>
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
    </div>
  );
}
