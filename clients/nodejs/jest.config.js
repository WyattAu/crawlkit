module.exports = {
  testEnvironment: "node",
  roots: ["<rootDir>/src"],
  transform: {
    "^.+\\.tsx?$": "<rootDir>/jest.ts-transform.js",
  },
  moduleFileExtensions: ["ts", "tsx", "js", "mjs", "json"],
};
