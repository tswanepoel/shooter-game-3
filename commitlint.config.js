module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'type-enum': [2, 'always', ['feat', 'fix', 'docs', 'refactor', 'chore']],
    'scope-empty': [2, 'never'],
    'scope-enum': [
      2,
      'always',
      ['sim', 'client', 'net', 'server', 'web', 'assets', 'ci', 'infra', 'docs', 'repo'],
    ],
    'subject-full-stop': [0, 'never'],
    'header-max-length': [2, 'always', 72],
  },
};
