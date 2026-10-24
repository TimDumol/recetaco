# Security review — 2026-10-24

Reviewed main at 031fc112cd43f4219ba41b54977e3854119a341f.

## Remediation

The three findings below are fixed in this change:

- Android requires HTTPS except for exact loopback development hosts. Network security configuration denies other cleartext traffic, and the default API client refuses HTTPS-to-HTTP redirects.
- Unauthenticated API and MCP requests require a localhost or loopback IP Host, including requests without Origin. Public reverse proxies must use OIDC or token authentication.
- Photo uploads acquire capacity before body collection and return HTTP 429 when busy. Collection has a 5 MiB limit and a 15-second timeout. The blocking image task owns the permit until processing completes, including after request cancellation.

## Original findings

1. **High: Android can expose bearer tokens over HTTP.** `android/app/src/main/java/dev/recetaco/app/Api.kt:105–123` accepts HTTP origins and attaches Authorization headers. `android/app/src/main/AndroidManifest.xml:4` enables cleartext traffic. A user selecting an HTTP server exposes API tokens (including refreshed OIDC access tokens) to network observers; an attacker can also alter unauthenticated auth configuration in transit. Require HTTPS for non-loopback connections and restrict cleartext to explicit development hosts.

2. **Medium: unauthenticated local mode lacks a trusted Host allowlist.** `server/src/main.rs:8–13,54` allows no authentication on loopback. `server/src/lib.rs:83–97` compares Origin to the request-supplied Host rather than a configured trusted origin. An attacker-controlled hostname that rebinds to loopback can satisfy both values and access the API without a token where browser network protections permit it. Validate Host against configured trusted origins, including requests without Origin, and prefer authentication even locally. This is a source-level finding; no live browser exploit was run.

3. **Medium: photo uploads can accumulate unbounded buffered bodies.** `server/src/photos.rs:41–50` extracts up to 5 MiB per request before waiting on the single-worker semaphore. Many concurrent authenticated uploads can retain these bodies while queued and exhaust memory; cancellation also releases the permit while already-started blocking work can continue. Acquire bounded capacity before body collection, reject excess requests, and keep capacity owned by the blocking task until completion. Proxy concurrency limits can provide immediate mitigation.

## Secrets and history

Foxguard secrets scanning found one private key: `server/tests/fixtures/test-only-rsa.pem`. It is explicitly loaded by the test issuer in `server/tests/oidc.rs:77`; no production key-loading reference was found. It is public test material and must never be reused by a real issuer. The new configuration excludes only this exact fixture from secret scanning. No other credential exposure was detected by the scanner and targeted manual searches.

The cloned main history has two commits; the earlier commit contains only LICENSE. Other remote branches, external deployments, and runtime credentials were not audited. PNG screenshots and binary artifacts were not manually inspected for secrets.

## Scanner results and limits

Foxguard 0.14.1 code scan reported 16 findings. Ten are post-quantum migration notices, five are generic SSRF/command-injection warnings whose inspected call sites use fixed local paths/origins or browser-relative API URLs, and one flags an unwrap of the static MCP tools array. These do not establish exploitable vulnerabilities at those call sites. No blanket baseline or code-rule suppression was added; touching flagged files may require reviewing existing findings before committing.

The code scanner scanned 48 files and skipped 26: 25 unsupported extensions and a parse error in Android MainActivity.kt. Manual review supplemented scanner coverage but is not a complete audit.

Foxguard SCA attempted both lockfiles but OSV returned HTTP 400 and the tool exited successfully with an empty report. **Dependency vulnerability status is unknown**, not clean. Rust regression tests cover trusted and untrusted Hosts, matching hostile Origins, photo admission, body limits, and capacity release; the existing API/OIDC/photo suites are also exercised. Android URL-validation tests now reject non-loopback HTTP origins.

Final validation: all 21 Rust tests, rustfmt, and Clippy with warnings denied passed. Android Gradle unit tests were attempted but could not run because this environment has no Android SDK; the new Android test cases remain unexecuted here.

## Installed protection

Foxguard 0.14.1 and pre-commit 4.5.1 are installed in this workspace at `/workspace/security-tools/bin`. The Git pre-commit hook is enabled in this checkout. Both code and secret hooks use staged mode and isolated Python environments with a pinned Foxguard version, so no global Foxguard executable is required.

For another checkout, run:

```sh
./scripts/install-security-hooks.sh
```

Run full scans with the installed CLI:

```sh
.security-venv/bin/foxguard .
.security-venv/bin/foxguard secrets .
.security-venv/bin/foxguard sca .
```

Validation: pre-commit configuration validation, installer shell syntax, clean staged hook pass, staged private-key rejection in an isolated temporary repository, and both hooks on the staged fixes all passed. The configured full secret scan has zero findings after the narrowly scoped test-key exclusion. Hooks can be bypassed locally; they are not a replacement for CI enforcement.
