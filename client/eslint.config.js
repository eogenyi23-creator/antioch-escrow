// ESLint flat config (ESLint 9/10)
const tseslint = require("@typescript-eslint/eslint-plugin");
const tsParser = require("@typescript-eslint/parser");

/** @type {import('eslint').Linter.Config[]} */
module.exports = [
  {
    // Lint all TypeScript source files.
    files: ["src/**/*.ts"],
    ignores: ["src/**/__tests__/**"],
    languageOptions: {
      parser: tsParser,
    },
    plugins: {
      "@typescript-eslint": tseslint,
    },
    rules: {
      ...tseslint.configs.recommended.rules,
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/explicit-function-return-type": "warn",
      "no-console": "warn",
    },
  },
];
