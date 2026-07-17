module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'type-enum': [
      2,
      'always',
      ['feat', 'fix', 'docs', 'refactor', 'chore']
    ],
    'subject-full-stop': [0, 'never'],
    'header-max-length': [2, 'always', 72],
    'scope-empty': [0, 'never']
  }
};