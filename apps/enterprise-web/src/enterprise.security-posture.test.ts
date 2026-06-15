import {
  calculateSecurityPostureScore,
  type SecurityPostureControl,
} from './enterprise.security-posture';

type TestExpectation = {
  toBe(expected: unknown): void;
  toEqual(expected: unknown): void;
};

type TestApi = {
  describe(name: string, fn: () => void): void;
  expect(actual: unknown): TestExpectation;
  it(name: string, fn: () => void): void;
};

const testModuleName = 'vite-plus/test';
const testApi = (await import(testModuleName)) as TestApi;
const describe: TestApi['describe'] = (name, fn) => testApi.describe(name, fn);
const expect: TestApi['expect'] = (actual) => testApi.expect(actual);
const it: TestApi['it'] = (name, fn) => testApi.it(name, fn);

describe('enterprise security posture', () => {
  it('calculates a weighted posture score', () => {
    const controls: SecurityPostureControl[] = [
      {
        id: 'mfa',
        label: 'MFA',
        description: 'Admins use MFA.',
        status: 'complete',
        weight: 25,
        completedWeight: 25,
        recommendation: 'Keep MFA enforced.',
        owner: 'Identity',
      },
      {
        id: 'sso',
        label: 'SSO',
        description: 'Tenant uses SSO.',
        status: 'attention',
        weight: 75,
        completedWeight: 25,
        recommendation: 'Configure SSO.',
        owner: 'Identity',
      },
    ];

    expect(calculateSecurityPostureScore(controls).score).toBe(50);
  });

  it('returns concrete open recommendations', () => {
    const controls: SecurityPostureControl[] = [
      {
        id: 'domain',
        label: 'Domain',
        description: 'Domain is verified.',
        status: 'critical',
        weight: 20,
        completedWeight: 0,
        recommendation: 'Verify domain.',
        owner: 'Tenant admin',
      },
    ];

    expect(calculateSecurityPostureScore(controls).openRecommendations).toEqual(controls);
  });
});
