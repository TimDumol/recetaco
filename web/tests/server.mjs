import { spawn } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
const dir = mkdtempSync(join(tmpdir(), "recetaco-browser-"));
const environment = { ...process.env };
for (const name of [
  "RECETACO_OIDC_ISSUER",
  "RECETACO_OIDC_CLIENT_ID",
  "RECETACO_OIDC_ANDROID_CLIENT_ID",
  "RECETACO_OIDC_AUDIENCE",
])
  delete environment[name];
const server = spawn("../target/debug/recetaco", [], {
  env: {
    ...environment,
    RECETACO_BIND: "127.0.0.1:18080",
    RECETACO_DB: join(dir, "test.db"),
    RECETACO_WEB_DIR: "dist",
    RECETACO_TOKEN: "",
  },
  stdio: "inherit",
});
for (const signal of ["SIGTERM", "SIGINT"])
  process.on(signal, () => {
    server.kill("SIGINT");
  });
server.on("exit", (code) => {
  rmSync(dir, { recursive: true, force: true });
  process.exit(code ?? 0);
});
