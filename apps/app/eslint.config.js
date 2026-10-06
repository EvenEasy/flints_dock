import tseslint from 'typescript-eslint';
import hooks from 'eslint-plugin-react-hooks';
import refresh from 'eslint-plugin-react-refresh';

// Lint only frontend-owned files; supplied assets and the existing IPC/backend remain intact.
export default tseslint.config(
  {
    ignores: [
      'assets/**',
      'dist/**',
      'node_modules/**',
      '.cache/**',
      'src-tauri/**',
      'frontend-contract/**',
      'test-results/**',
      'playwright-report/**',
    ],
  },
  ...tseslint.configs.recommended,
  {
    files: ['src/**/*.{ts,tsx}'],
    plugins: { 'react-hooks': hooks, 'react-refresh': refresh },
    rules: {
      'react-hooks/rules-of-hooks': 'error',
      'react-hooks/exhaustive-deps': 'error',
      'react-refresh/only-export-components': ['error', { allowConstantExport: true }],
      '@typescript-eslint/no-explicit-any': 'error',
    },
  },
);
