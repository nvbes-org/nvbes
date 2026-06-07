import {
  PersonalInfoForm,
  PersonalInfoSkeleton,
  PersonalInfoSummary,
} from './AccountPersonalInfoPage.shared';
import { useAccountPersonalInfoPage } from './useAccountPersonalInfoPage';

export default function AccountPersonalInfoPage() {
  const {
    birthdate,
    editError,
    editSuccess,
    firstname,
    fullName,
    handleSubmit,
    lastname,
    loading,
    memberSince,
    region,
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
      <div>
        <h1 className="text-xl font-heading font-semibold">Informations personnelles</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Consultez et gerez vos informations de profil.
        </p>
      </div>

      <div className="flex flex-col gap-6">
        <PersonalInfoSummary user={selectedUser} fullName={fullName} memberSince={memberSince} />
        <PersonalInfoForm
          firstname={firstname}
          lastname={lastname}
          username={username}
          birthdate={birthdate}
          region={region}
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
    </div>
  );
}
