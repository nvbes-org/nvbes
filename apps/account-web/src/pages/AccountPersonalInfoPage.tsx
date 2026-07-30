import { profileAvatarUrl } from '@/account.avatar';
import { AccountPage, AccountPageHeader } from '@/components/AccountPage';
import { AccountEmailAddresses } from './AccountEmailAddresses';
import { PersonalInfoCard, PersonalInfoSkeleton } from './AccountPersonalInfoPage.shared';
import { useAccountEmailAddresses } from './useAccountEmailAddresses';
import { useAccountPersonalInfoPage } from './useAccountPersonalInfoPage';

export default function AccountPersonalInfoPage() {
  const accountEmails = useAccountEmailAddresses();
  const {
    authuser,
    avatarVersion,
    birthdate,
    avatarError,
    avatarLoading,
    editError,
    editSuccess,
    firstname,
    handleSubmit,
    handleAvatarChange,
    handleAvatarDelete,
    lastname,
    loading,
    isPersonalInfoInvalid,
    isPersonalInfoUnchanged,
    memberSince,
    region,
    regionLoading,
    regions,
    personalInfoErrors,
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
    <AccountPage>
      <AccountPageHeader title="Informations personnelles" contentClassName="max-w-2xl" />

      <div className="max-w-4xl">
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
          isPersonalInfoInvalid={isPersonalInfoInvalid}
          isPersonalInfoUnchanged={isPersonalInfoUnchanged}
          errors={personalInfoErrors}
          onFirstnameChange={setFirstname}
          onLastnameChange={setLastname}
          onUsernameChange={setUsername}
          onBirthdateChange={setBirthdate}
          onRegionChange={setRegion}
          onSubmit={handleSubmit}
          avatarUrl={profileAvatarUrl(authuser, avatarVersion)}
          avatarError={avatarError}
          avatarLoading={avatarLoading}
          onAvatarChange={handleAvatarChange}
          onAvatarDelete={handleAvatarDelete}
        />
      </div>

      <AccountPageHeader
        as="h2"
        size="section"
        title="Addresses emails"
        description="L'email principal reste celui utilise pour la connexion et les documents du compte."
        contentClassName="max-w-2xl"
      />

      <div className="max-w-4xl">
        <AccountEmailAddresses
          adding={accountEmails.adding}
          deletingEmailId={accountEmails.deletingEmailId}
          emails={accountEmails.emails}
          emailDraft={accountEmails.emailDraft}
          error={accountEmails.error}
          promotingEmailId={accountEmails.promotingEmailId}
          primaryMinAgeHours={accountEmails.primaryMinAgeHours}
          resendingEmailId={accountEmails.resendingEmailId}
          onAdd={accountEmails.handleAdd}
          onDelete={accountEmails.handleDelete}
          onDraftChange={accountEmails.setEmailDraft}
          onPromote={accountEmails.handlePromote}
          onResendVerification={accountEmails.handleResendVerification}
        />
      </div>
    </AccountPage>
  );
}
