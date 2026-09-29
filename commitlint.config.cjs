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
      },
    },
  ],
  rules: {
    'ai-assisted-format': [2, 'always'],
  },
};
