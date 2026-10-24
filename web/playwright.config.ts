import { defineConfig, devices } from "@playwright/test";
export default defineConfig({
  testDir: "./tests",
  fullyParallel: false,
  workers: 1,
  use: { baseURL: "http://127.0.0.1:18080", trace: "retain-on-failure" },
  webServer: [
    {
      command: "node tests/server.mjs",
      url: "http://127.0.0.1:18080/health",
      reuseExistingServer: false,
    },
    {
      command: "node tests/oidc-server.mjs",
      url: "http://127.0.0.1:18083/health",
      reuseExistingServer: false,
    },
  ],
  projects: [
    {
      name: "oidc",
      testMatch: "auth.spec.ts",
      use: { ...devices["Desktop Chrome"] },
    },
    {
      name: "desktop",
      testMatch: "workflow.spec.ts",
      use: { ...devices["Desktop Chrome"] },
    },
    {
      name: "mobile",
      testMatch: "workflow.spec.ts",
      use: { ...devices["Pixel 7"], defaultBrowserType: "chromium" },
    },
  ],
});
