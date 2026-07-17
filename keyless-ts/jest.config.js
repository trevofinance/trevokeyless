module.exports = {
  preset: "ts-jest",
  testEnvironment: "node",
  testMatch: ["**/tests/**/*.test.ts"],
  // @noble/hashes v2 and @scure/base v2 are published as ESM-only, but the suite runs
  // as CommonJS. Transpile those packages to CommonJS (they are kept out of the default
  // transformIgnorePatterns below) with transforms scoped to their files, so
  // isolatedModules/allowJs do not affect type-only export erasure in the
  // project's own TypeScript. The [/\\\\] separator classes keep the patterns
  // matching on Windows, where jest tests them against backslash paths — with
  // plain "/" separators neither pattern matched there, so the packages were
  // loaded untransformed and the suite failed on their ESM syntax.
  transform: {
    "[/\\\\]@noble[/\\\\]hashes[/\\\\].+\\.js$": [
      "ts-jest",
      { isolatedModules: true, tsconfig: { allowJs: true } },
    ],
    "[/\\\\]@scure[/\\\\]base[/\\\\].+\\.js$": [
      "ts-jest",
      { isolatedModules: true, tsconfig: { allowJs: true } },
    ],
    "^.+\\.tsx?$": ["ts-jest", {}],
  },
  transformIgnorePatterns: [
    "[/\\\\]node_modules[/\\\\](?!@noble[/\\\\]hashes[/\\\\]|@scure[/\\\\]base[/\\\\])",
  ],
  // Source uses Node-compatible `.js` specifiers so the emitted ESM can run
  // directly. During ts-jest execution those specifiers resolve to `.ts` files.
  moduleNameMapper: {
    "^(\\.{1,2}/.*)\\.js$": "$1",
  },
};
