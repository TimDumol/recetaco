# Pocket ID sign-in

Recetaco integrates directly with Pocket ID using OIDC authorization code flow with **S256 PKCE**. Pocket ID owns accounts, passkeys, user-group restrictions, consent, and token issuance. There is no password database in Recetaco.

Recetaco is one shared kitchen. Both partners have equal full permissions to create, edit, and delete recipes/photos, ratings and cook logs, weekly wishes, meal plans, and shopping lists. There is no owner/member hierarchy. Restrict the application/API to your household in Pocket ID. You can additionally restrict Recetaco itself to your two stable Pocket ID subject IDs.

## Limit access to your two accounts

Set this alongside the OIDC issuer/client settings:

```sh
export RECETACO_OIDC_ALLOWED_SUBJECTS=YOUR_POCKET_ID_USER_ID,YOUR_WIFES_POCKET_ID_USER_ID
```

Use each account’s stable OIDC `sub` value (the Pocket ID user ID), rather than an email or display name. Both listed accounts get identical full Recetaco permissions. A correctly signed token from another account receives HTTP 403. The allowlist applies equally to API, photos, and MCP, and is not exposed by the public auth-config endpoint. An explicitly empty allowlist is rejected at startup. When unset, access is determined by Pocket ID’s client/API restrictions.

## Provider requirements

Use a Pocket ID version supporting **RFC 9068 JWT access tokens** and **RFC 8707 resource indicators / custom API audiences**. Older deployments issuing opaque access tokens without an API audience need an upgrade; this integration deliberately does not accept an ID token as an API bearer token. Recetaco supports RS256/384/512, ES256/384, and EdDSA signing keys.

The API validates the `at+jwt` token type, signature against discovered JWKS, exact issuer and resource audience, subject, expiration, and optional not-before time. Signing keys are cached for five minutes, with refresh attempts for an unknown key limited to once per 30 seconds. No provider call is made for every API request.

## Register Recetaco in Pocket ID

Replace the example hostnames with your own public HTTPS addresses.

1. Create/register a custom API resource with audience **`https://recetaco.example.com`**, without a trailing slash. Grant your Recetaco clients access to it. No additional application-specific permission scopes are required in this first version.
2. Create a **public browser client** (no client secret). Enable authorization code flow, S256 PKCE, and refresh tokens / `offline_access` if available. Register the exact browser callback **`https://recetaco.example.com/auth/callback`** and post-logout callback **`https://recetaco.example.com`**. Allow this origin to call the token endpoint if your deployment applies an origin restriction.
3. Create a **public native Android client** (no client secret) with the exact callback **`dev.recetaco.app:/oauth2redirect`**. Enable code flow with PKCE and refresh tokens. It must have access to the same API resource. Pocket ID must permit this registered native application URI.
4. Apply Pocket ID's client/API user-group restrictions for the people who should share this kitchen.
5. Copy the issuer and client IDs to the Recetaco server configuration below. Client IDs and the issuer are public; do not configure a client secret in either app.

```sh
export RECETACO_OIDC_ISSUER=https://id.example.com
export RECETACO_OIDC_CLIENT_ID=YOUR_BROWSER_CLIENT_ID
export RECETACO_OIDC_ANDROID_CLIENT_ID=YOUR_ANDROID_CLIENT_ID
export RECETACO_OIDC_AUDIENCE=https://recetaco.example.com
export RECETACO_BIND=127.0.0.1:8080
unset RECETACO_TOKEN
./target/release/recetaco
```

Terminate HTTPS at your reverse proxy, forward Recetaco's public hostname in `Host`, and proxy the UI, `/auth/*`, `/api/*`, `/mcp`, and `/.well-known/*` to Recetaco. This reverse proxy supplies TLS, not authentication. Provider discovery must return an issuer exactly matching `RECETACO_OIDC_ISSUER`; avoid accidentally adding or removing its trailing slash. The resource audience must exactly match Pocket ID's issued token audience.

`RECETACO_OIDC_ANDROID_CLIENT_ID` can default to the browser client ID only if you intentionally register both exact callback URLs on one public client. Separate clients are recommended.

Recetaco fails startup if discovery or JWKS cannot be loaded, OIDC settings are incomplete, or `RECETACO_TOKEN` is also set. OIDC settings must use HTTPS; loopback HTTP is accepted only by the Rust service for local tests. The native Pocket ID sign-in UI requires HTTPS for both Recetaco and the issuer.

## Browser

Open Recetaco and choose **Sign in with Pocket ID**. The OIDC client library maintains state, nonce, and PKCE. Tokens and pending authorization state stay in that browser tab's session storage. Access-token renewal uses the provider's refresh token. Browser JavaScript receives no client secret.

**Connection settings → Sign out** removes the local session and redirects to Pocket ID's discovered logout endpoint. API failures never redirect an API request to an HTML login page.

## Android

Set the HTTPS Recetaco URL in **Connection settings**, then choose **Sign in with Pocket ID**. AppAuth discovers the provider and opens the system browser/Custom Tab so Pocket ID can use passkeys. AppAuth checks the authorization response and exchanges the code with its PKCE verifier. The app sends an access token to Recetaco, never an ID token.

The full authorization state, including any refresh token, is encrypted at rest with Android Keystore and excluded from backup/device transfer. AppAuth refreshes tokens as needed and preserves the API resource parameter. **Sign out of this app** clears local authorization; the independent Pocket ID browser session remains available for a future sign-in.

Custom URI callbacks use PKCE to protect intercepted authorization codes. A future distribution can use verified HTTPS App Links with your domain; that requires hosting its association file and changing both registered and Android callback URLs.

## MCP and other API clients

`POST /mcp` requires a **Pocket ID access token for the same Recetaco API audience**. The old shared server token does not bypass OIDC.

An unauthenticated request returns HTTP 401 with a `WWW-Authenticate` challenge pointing at `/.well-known/oauth-protected-resource`. That document advertises the Recetaco resource and Pocket ID authorization server. Register an OAuth client in Pocket ID for your MCP client and grant it access to the Recetaco API. Configure that client ID/callback with the MCP client if it supports pre-registered OAuth clients.

Recetaco does not implement dynamic client registration or act as an authorization server. Automatic sign-in depends on the MCP client's OAuth and pre-registered-client support. A client that only supports fixed headers can use a separately obtained, short-lived resource access token, but must arrange its own renewal. Do not supply Pocket ID's administrative API key or an ID token.

```json
{
  "mcpServers": {
    "recetaco": {
      "url": "https://recetaco.example.com/mcp",
      "headers": { "Authorization": "Bearer POCKET_ID_RESOURCE_ACCESS_TOKEN" }
    }
  }
}
```

JWT verification is local: revoking access or signing out does not retroactively invalidate an already issued access token before its expiration. Provider-side refresh restrictions prevent new tokens after access is removed. Back-channel logout and online token introspection are not implemented.

## Development mode

Without `RECETACO_OIDC_ISSUER`, loopback development remains unauthenticated, or `RECETACO_TOKEN` can protect a development server. The browser detects that mode; Android exposes **Development connection options**. Those paths cannot override OIDC when it is enabled.

## Verification

Rust integration tests use a local issuer and test RSA keys to verify valid access, invalid signatures, wrong issuer/audience, expired/not-yet-valid credentials, rejected ID tokens, metadata, and protection of MCP. Browser tests run an isolated OIDC issuer that verifies S256 PKCE and complete sign-in, a real authenticated API call, reload, and logout. Android API tests check that refreshed access tokens are used.

These tests do not authenticate against your Pocket ID installation or exercise a passkey on a physical Android device. Deployment still requires the Pocket ID registrations and environment configuration above.
