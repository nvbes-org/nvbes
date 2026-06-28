import { AccountEmailAddresses } from './AccountEmailAddresses';
import { PersonalInfoCard, PersonalInfoSkeleton } from './AccountPersonalInfoPage.shared';
import { useAccountEmailAddresses } from './useAccountEmailAddresses';
import { useAccountPersonalInfoPage } from './useAccountPersonalInfoPage';

export default function AccountPersonalInfoPage() {
  const accountEmails = useAccountEmailAddresses();
  const {
    birthdate,
    editError,
    editSuccess,
    firstname,
    handleSubmit,
    lastname,
    loading,
    memberSince,
    region,
    regionLoading,
    regions,
    selectedUser,
    setBirthdate,
    setFirstname,
    setLastname,
    setRegion,
    setUsername,
    username,
  } = useAccountPersonalInfoPage();

  if (!selectedUser) {
    return <PersonalInfoSkeleton />;
  }

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div className="max-w-2xl">
        <h1 className="text-xl font-heading font-semibold">Profil</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Mettez a jour les informations associees a votre compte Identity.
        </p>
      </div>

      <div className="max-w-3xl">
        <PersonalInfoCard
          user={selectedUser}
          memberSince={memberSince}
          firstname={firstname}
          lastname={lastname}
          username={username}
          birthdate={birthdate}
          region={region}
          regionLoading={regionLoading}
          regions={regions}
          editError={editError}
          editSuccess={editSuccess}
          loading={loading}
          onFirstnameChange={setFirstname}
          onLastnameChange={setLastname}
          onUsernameChange={setUsername}
          onBirthdateChange={setBirthdate}
          onRegionChange={setRegion}
          onSubmit={handleSubmit}
        />
      </div>

      <div className="max-w-3xl">
        <AccountEmailAddresses
          emails={accountEmails.emails}
          emailDraft={accountEmails.emailDraft}
          error={accountEmails.error}
          loading={accountEmails.loading}
          primaryMinAgeHours={accountEmails.primaryMinAgeHours}
          success={accountEmails.success}
          onAdd={accountEmails.handleAdd}
          onDelete={accountEmails.handleDelete}
          onDraftChange={accountEmails.setEmailDraft}
          onPromote={accountEmails.handlePromote}
          onResendVerification={accountEmails.handleResendVerification}
        />
      </div>
    </div>
  );
}
