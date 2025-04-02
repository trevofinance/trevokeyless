import dts from "rollup-plugin-dts";

export default [
  {
    input: "dist/keyless.js",
    output: {
      file: "bundle/index.js",
    },
  },
  {
    input: "dist/keyless.d.ts",
    output: {
      file: "bundle/index.d.ts",
    },
    plugins: [dts()],
  },
];
