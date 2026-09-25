module.exports = {
  extends: ['@commitlint/config-conventional'],
  defaultIgnores: false,
  plugins: [
    {
      rules: {
        'ai-assisted-format': ({ raw }) => {
          if (!/^AI-Assisted:/imu.test(raw)) {
            return [true];
          }
          const ok = /^AI-Assisted:\s+\S.+$/imu.test(raw);
          return [
            ok,
            'AI-Assisted trailer must be "AI-Assisted: <agent-or-model>" on its own line',
          ];
        },
        'human-review-required-format': ({ raw }) => {
          if (!/^Human-Review-Required:/imu.test(raw)) {
            return [true];
          }
          const ok = /^Human-Review-Required:\s*protected-paths\s*$/imu.test(raw);
          return [
            ok,
            'Human-Review-Required trailer must be exactly "Human-Review-Required: protected-paths"',
          ];
        },
      },
    },
  ],
  rules: {
    'ai-assisted-format': [2, 'always'],
    'human-review-required-format': [2, 'always'],
  },
};
