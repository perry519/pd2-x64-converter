import antfu from '@antfu/eslint-config'
import betterTailwindcss from 'eslint-plugin-better-tailwindcss'
import globals from 'globals'

export default antfu(
  {
    ignores: ['dist/**', 'target/**', 'src-tauri/target/**'],
    stylistic: true,
    svelte: true,
  },
  {
    ...betterTailwindcss.configs['recommended-warn'],
    files: ['src/**/*.{ts,svelte}'],
    ignores: ['src/components/ui/**/*'],
    settings: {
      'better-tailwindcss': {
        entryPoint: 'src/app.css',
      },
    },
  },
  {
    files: ['**/*.svelte'],
    rules: {
      'import/no-mutable-exports': 'off',
      'no-redeclare': 'off',
    },
  },
  {
    files: ['wdio.conf.mjs', 'test/e2e/**/*.mjs'],
    languageOptions: {
      globals: globals.mocha,
    },
  },
)
