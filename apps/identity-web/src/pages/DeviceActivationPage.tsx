import {
  DeviceActivationApproveStep,
  DeviceActivationInputStep,
  DeviceActivationShell,
  DeviceActivationSuccessStep,
} from './DeviceActivationPage.shared';
import { useDeviceActivationPage } from './useDeviceActivationPage';

export default function DeviceActivationPage() {
  const {
    deviceInfo,
    error,
    handleApprove,
    handleDeny,
    handleVerify,
    loading,
    lockedWorkspace,
    navigate,
    selectedWorkspace,
    setSelectedWorkspace,
    setUserCode,
    step,
    userCode,
    workspaces,
  } = useDeviceActivationPage();

  return (
    <DeviceActivationShell error={error}>
      {step === 'input' && (
        <DeviceActivationInputStep
          userCode={userCode}
          loading={loading}
          onUserCodeChange={setUserCode}
          onSubmit={handleVerify}
        />
      )}

      {step === 'approve' && deviceInfo && (
        <DeviceActivationApproveStep
          deviceInfo={deviceInfo}
          workspaces={workspaces}
          selectedWorkspace={selectedWorkspace}
          lockedWorkspace={lockedWorkspace}
          loading={loading}
          onWorkspaceChange={setSelectedWorkspace}
          onApprove={handleApprove}
          onDeny={handleDeny}
        />
      )}

      {step === 'success' && (
        <DeviceActivationSuccessStep onGoAccount={() => void navigate({ to: '/account' })} />
      )}
    </DeviceActivationShell>
  );
}
