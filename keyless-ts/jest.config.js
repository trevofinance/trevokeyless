module.exports = {
  preset: "ts-jest",
  testEnvironment: "node",
  testMatch: ["**/tests/**/*.test.ts"],
  // @noble/hashes v2 is published as ESM-only, but the suite runs as CommonJS.
  // Transpile the package to CommonJS (it is kept out of the default
  // transformIgnorePatterns below) with a transform scoped to its files, so
  // isolatedModules/allowJs do not affect type-only export erasure in the
  // project's own TypeScript.
  transform: {
    "/@noble/hashes/.+\\.js$": [
      "ts-jest",
      { isolatedModules: true, tsconfig: { allowJs: true } },
    ],
    "^.+\\.tsx?$": ["ts-jest", {}],
  },
  transformIgnorePatterns: ["/node_modules/(?!@noble/hashes/)"],
};
