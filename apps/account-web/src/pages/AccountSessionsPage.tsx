import { Button } from '@/components/ui/button';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import StepUpModal from '@/components/StepUpModal';
import { ShieldAlert } from 'lucide-react';
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
    currentSession,
    confirmingRisk,
    isPending,
    otherDevices,
    recognizedDevices,
    revoking,
    sessions,
    onRevoke,
    onRevokeDevice,
    onRevokeOthers,
    onRequestRiskConfirmation,
    onRiskStepUpSuccess,
    onCancelRiskConfirmation,
    showRiskStepUp,
    showRevokeOthersStepUp,
    onCancelRevokeOthers,
    onRevokeOthersStepUpSuccess,
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

      {currentSession?.risk_decision === 'step_up' && currentSession.risk_confirmed_at === null && (
        <Alert variant="destructive">
          <ShieldAlert />
          <AlertTitle>Session à risque à confirmer</AlertTitle>
          <AlertDescription className="flex flex-wrap items-center justify-between gap-3">
            <span>
              Une variation inhabituelle a été détectée. Confirmez explicitement cette session avant
              de poursuivre des opérations sensibles.
            </span>
            <Button
              size="sm"
              variant="outline"
              disabled={confirmingRisk}
              onClick={onRequestRiskConfirmation}
            >
              Confirmer cette session
            </Button>
          </AlertDescription>
        </Alert>
      )}

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

      <StepUpModal
        open={showRiskStepUp}
        onOpenChange={(open) => {
          if (!open) onCancelRiskConfirmation();
        }}
        onSuccess={onRiskStepUpSuccess}
        onCancel={onCancelRiskConfirmation}
        description="Confirmez votre identité pour autoriser explicitement cette session à risque."
      />
      <StepUpModal
        open={showRevokeOthersStepUp}
        onOpenChange={(open) => {
          if (!open) onCancelRevokeOthers();
        }}
        onSuccess={() => void onRevokeOthersStepUpSuccess()}
        onCancel={onCancelRevokeOthers}
        description="Confirmez votre identité pour déconnecter toutes les autres sessions."
      />
    </div>
  );
}
