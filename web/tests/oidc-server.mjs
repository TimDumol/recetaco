// Test-only OIDC issuer: no real accounts or passkeys. Checks code + S256 PKCE.
import { createServer } from "node:http";
import {
  generateKeyPairSync,
  randomBytes,
  createHash,
  sign,
} from "node:crypto";
import { spawn } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
const issuer = "http://127.0.0.1:18084",
  origin = "http://127.0.0.1:18083";
const { publicKey, privateKey } = generateKeyPairSync("rsa", {
  modulusLength: 2048,
});
const jwk = {
  ...publicKey.export({ format: "jwk" }),
  kid: "browser-test",
  use: "sig",
  alg: "RS256",
};
const codes = new Map(),
  refreshes = new Map();
function jwt(claims, typ) {
  const encoded = [
    Buffer.from(JSON.stringify({ alg: "RS256", typ, kid: jwk.kid })).toString(
      "base64url",
    ),
    Buffer.from(JSON.stringify(claims)).toString("base64url"),
  ].join(".");
  return (
    encoded +
    "." +
    sign("RSA-SHA256", Buffer.from(encoded), privateKey).toString("base64url")
  );
}
const provider = createServer(async (req, res) => {
  res.setHeader("Access-Control-Allow-Origin", origin);
  res.setHeader("Access-Control-Allow-Headers", "content-type");
  const url = new URL(req.url, issuer);
  const json = (code, data) => {
    res.writeHead(code, {
      "Content-Type": "application/json",
      "Cache-Control": "no-store",
    });
    res.end(JSON.stringify(data));
  };
  if (req.method === "OPTIONS") {
    res.writeHead(204);
    return res.end();
  }
  if (url.pathname === "/.well-known/openid-configuration")
    return json(200, {
      issuer,
      authorization_endpoint: issuer + "/authorize",
      token_endpoint: issuer + "/token",
      jwks_uri: issuer + "/keys",
      end_session_endpoint: issuer + "/logout",
      response_types_supported: ["code"],
      subject_types_supported: ["public"],
      id_token_signing_alg_values_supported: ["RS256"],
      code_challenge_methods_supported: ["S256"],
    });
  if (url.pathname === "/keys") return json(200, { keys: [jwk] });
  if (url.pathname === "/authorize") {
    const params = url.searchParams;
    if (
      params.get("client_id") !== "browser-client" ||
      params.get("redirect_uri") !== origin + "/auth/callback" ||
      params.get("code_challenge_method") !== "S256" ||
      params.get("resource") !== origin
    )
      return json(400, { error: "invalid_request" });
    const code = randomBytes(24).toString("hex");
    codes.set(code, {
      challenge: params.get("code_challenge"),
      nonce: params.get("nonce"),
      client: params.get("client_id"),
    });
    const callback = new URL(params.get("redirect_uri"));
    callback.searchParams.set("code", code);
    callback.searchParams.set("state", params.get("state"));
    res.writeHead(302, { Location: callback.toString() });
    return res.end();
  }
  if (url.pathname === "/token") {
    const chunks = [];
    for await (const chunk of req) chunks.push(chunk);
    const params = new URLSearchParams(Buffer.concat(chunks).toString());
    let grant;
    if (params.get("grant_type") === "authorization_code") {
      grant = codes.get(params.get("code"));
      codes.delete(params.get("code"));
      if (
        !grant ||
        params.get("client_id") !== grant.client ||
        createHash("sha256")
          .update(params.get("code_verifier") || "")
          .digest("base64url") !== grant.challenge
      )
        return json(400, { error: "invalid_grant" });
    } else {
      grant = refreshes.get(params.get("refresh_token"));
      if (!grant) return json(400, { error: "invalid_grant" });
    }
    const now = Math.floor(Date.now() / 1000),
      refresh = randomBytes(32).toString("hex");
    refreshes.set(refresh, grant);
    return json(200, {
      token_type: "Bearer",
      expires_in: 600,
      scope: "openid profile offline_access",
      access_token: jwt(
        {
          iss: issuer,
          aud: origin,
          sub: "test-household-user",
          iat: now,
          exp: now + 600,
        },
        "at+jwt",
      ),
      id_token: jwt(
        {
          iss: issuer,
          aud: grant.client,
          sub: "test-household-user",
          nonce: grant.nonce,
          iat: now,
          exp: now + 600,
        },
        "JWT",
      ),
      refresh_token: refresh,
    });
  }
  if (url.pathname === "/logout") {
    res.writeHead(302, { Location: origin });
    return res.end();
  }
  json(404, { error: "not_found" });
});
await new Promise((resolve) => provider.listen(18084, "127.0.0.1", resolve));
const dir = mkdtempSync(join(tmpdir(), "recetaco-oidc-"));
const server = spawn("../target/debug/recetaco", [], {
  env: {
    ...process.env,
    RECETACO_BIND: "127.0.0.1:18083",
    RECETACO_DB: join(dir, "db.sqlite"),
    RECETACO_WEB_DIR: "dist",
    RECETACO_TOKEN: "",
    RECETACO_OIDC_ISSUER: issuer,
    RECETACO_OIDC_CLIENT_ID: "browser-client",
    RECETACO_OIDC_ANDROID_CLIENT_ID: "android-client",
    RECETACO_OIDC_AUDIENCE: origin,
  },
  stdio: "inherit",
});
for (const signal of ["SIGTERM", "SIGINT"])
  process.on(signal, () => server.kill("SIGINT"));
server.on("exit", (code) => {
  provider.close();
  rmSync(dir, { recursive: true, force: true });
  process.exit(code ?? 0);
});
