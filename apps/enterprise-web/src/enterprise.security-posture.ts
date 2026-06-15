export type SecurityPostureStatus = 'complete' | 'attention' | 'critical';

export type SecurityPostureControl = {
  id: string;
  label: string;
  description: string;
  status: SecurityPostureStatus;
  weight: number;
  completedWeight: number;
  recommendation: string;
  owner: string;
};

export type SecurityPostureScore = {
  score: number;
  maxScore: number;
  completedWeight: number;
  status: SecurityPostureStatus;
  completedControls: number;
  totalControls: number;
  openRecommendations: SecurityPostureControl[];
};

export const tenantSecurityControls: SecurityPostureControl[] = [
  {
    id: 'admin-mfa',
    label: 'Admin MFA',
    description: 'Owner and admin accounts must use phishing-resistant MFA or TOTP.',
    status: 'critical',
    weight: 25,
    completedWeight: 0,
    recommendation: 'Activer MFA obligatoire pour tous les admins et bloquer les exceptions.',
    owner: 'Identity',
  },
  {
    id: 'session-ttl',
    label: 'Session TTL',
    description: 'Tenant sessions should expire quickly enough for privileged contexts.',
    status: 'attention',
    weight: 15,
    completedWeight: 5,
    recommendation: 'Reduire le TTL des sessions admin a 8 heures avec re-auth step-up.',
    owner: 'Identity',
  },
  {
    id: 'verified-domain',
    label: 'Verified domain',
    description: 'The tenant primary email domain is DNS verified and protected from spoofing.',
    status: 'critical',
    weight: 20,
    completedWeight: 0,
    recommendation: 'Verifier le domaine principal par DNS avant tout auto-provisioning.',
    owner: 'Tenant admin',
  },
  {
    id: 'sso',
    label: 'SSO',
    description: 'Enterprise sign-in should be centralized through a managed IdP.',
    status: 'attention',
    weight: 20,
    completedWeight: 8,
    recommendation: 'Configurer SSO SAML/OIDC puis forcer les admins sur le fournisseur verifie.',
    owner: 'Identity',
  },
  {
    id: 'old-secrets',
    label: 'Old secrets',
    description: 'OAuth clients and service accounts should not keep stale secret versions active.',
    status: 'attention',
    weight: 20,
    completedWeight: 10,
    recommendation: 'Revoquer les anciens secrets apres la fenetre de rotation approuvee.',
    owner: 'Developers',
  },
];

export function calculateSecurityPostureScore(
  controls: SecurityPostureControl[],
): SecurityPostureScore {
  const maxScore = controls.reduce((sum, control) => sum + control.weight, 0);
  const completedWeight = controls.reduce((sum, control) => sum + control.completedWeight, 0);
  const score = maxScore === 0 ? 0 : Math.round((completedWeight / maxScore) * 100);
  const completedControls = controls.filter((control) => control.status === 'complete').length;
  const openRecommendations = controls.filter((control) => control.status !== 'complete');

  return {
    score,
    maxScore,
    completedWeight,
    status: score >= 80 ? 'complete' : score >= 50 ? 'attention' : 'critical',
    completedControls,
    totalControls: controls.length,
    openRecommendations,
  };
}
