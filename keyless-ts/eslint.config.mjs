// @ts-check
import js from "@eslint/js";
import prettier from "eslint-config-prettier";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist/", "coverage/", "node_modules/"] },
  js.configs.recommended,
  ...tseslint.configs.recommendedTypeChecked,
  {
    languageOptions: {
      parserOptions: {
        // tests/ carries its own tsconfig.json (extending the root one, which
        // builds src/ only), so the project service can type-lint the test files.
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
  // Plain-JS files (this config, jest.config.js, scripts/*.mjs) belong to no
  // TypeScript project: lint them without type information, with the Node globals
  // they actually use declared explicitly. Two blocks so the globals merge with —
  // rather than clobber — disableTypeChecked's languageOptions.
  {
    files: ["**/*.{js,mjs,cjs}"],
    ...tseslint.configs.disableTypeChecked,
  },
  {
    files: ["**/*.{js,mjs,cjs}"],
    languageOptions: {
      globals: {
        console: "readonly",
        process: "readonly",
        module: "writable",
        require: "readonly",
        __dirname: "readonly",
        URL: "readonly",
      },
    },
  },
  // Must stay last: disables stylistic rules that would fight Prettier.
  prettier,
);
