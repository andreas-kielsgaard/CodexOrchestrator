import js from '@eslint/js';
import globals from 'globals';
import reactHooks from 'eslint-plugin-react-hooks';
import reactRefresh from 'eslint-plugin-react-refresh';
import tseslint from 'typescript-eslint';
import prettier from 'eslint-config-prettier';

export default tseslint.config(
  { ignores: ['dist', 'coverage', 'storybook-static', '.dev', 'src-tauri/target'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: [
      'scripts/**/*.{js,cjs,mjs}',
      'review-tools/app-inspector/**/*.{js,mjs}',
      'docs/regression-review/**/run-browser.mjs',
      'docs/ux/session-event-model-walkthrough/build/*.mjs',
    ],
    languageOptions: {
      ecmaVersion: 2022,
      globals: globals.node,
    },
  },
  {
    files: ['docs/regression-review/**/run-browser.mjs'],
    languageOptions: { globals: globals.browser },
  },
  {
    files: ['**/*.{ts,tsx}'],
    languageOptions: {
      ecmaVersion: 2022,
      globals: globals.browser,
    },
    plugins: {
      'react-hooks': reactHooks,
      'react-refresh': reactRefresh,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      'react-refresh/only-export-components': ['warn', { allowConstantExport: true }],
    },
  },
  {
    // Provider raw payloads are diagnostic evidence. Shared UI decides on normalized events and
    // Orchid's own control records, and may pass raw evidence along but never inspect it.
    files: ['src/**/*.{ts,tsx}'],
    ignores: [
      'src/**/*.test.{ts,tsx}',
      'src/dev/**',
      'src/**/agentProviders/**',
      'src/application/agentSessions/runtimeControlRecords.ts',
      'src/features/agentSessions/runtimeDiagnostics.ts',
    ],
    rules: {
      'no-restricted-syntax': [
        'error',
        ...[
          "MemberExpression[object.type='MemberExpression'][object.property.name='rawPayload']",
          "TSAsExpression > MemberExpression.expression[property.name='rawPayload']",
          "VariableDeclarator > MemberExpression.init[property.name='rawPayload']",
          "UnaryExpression[operator='typeof'] > MemberExpression[property.name='rawPayload']",
          "BinaryExpression > MemberExpression[property.name='rawPayload']",
        ].map((selector) => ({
          selector,
          message:
            'Read provider facts from normalized events or runtimeControlRecordKind, not rawPayload.',
        })),
      ],
    },
  },
  prettier,
);
