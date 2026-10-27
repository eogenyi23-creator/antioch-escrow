/** @type {import('jest').Config} */
module.exports = {
  // Run tests in ESM mode to support @stellar/stellar-sdk's pure-ESM
  // transitive dependencies (uint8array-extras, @noble/hashes, @exodus/bytes).
  // Requires: node --experimental-vm-modules in the test script.
  extensionsToTreatAsEsm: [".ts"],
  transform: {
    "^.+\\.tsx?$": [
      "ts-jest",
      {
        useESM: true,
        tsconfig: {
          module: "ESNext",
          moduleResolution: "bundler",
        },
      },
    ],
  },
  testEnvironment: "node",
  roots: ["<rootDir>/src"],
  testMatch: ["**/__tests__/**/*.test.ts"],
  collectCoverageFrom: ["src/**/*.ts", "!src/**/__tests__/**"],
};
