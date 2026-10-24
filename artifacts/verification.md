# Verification — 2026-10-24

- Rust: 19 integration tests passed. Includes independent shared recipe ratings/per-cook ratings, averages excluding unrated entries, last-cooked date ordering, edits/deletions, invalid input, cross-recipe isolation, export, metadata preservation, cascading recipe deletion, authentication, photos, weekly shopping, and existing-database migration. Formatting and Clippy passed with warnings denied.
- Browser: all 7 recipe/photo/OIDC/wishlist/cook-history scenarios passed together after the Recetaco rename. The new scenarios create two cooks with notes, edit a rating, delete an entry, clear the independent recipe rating, and verify persistence after reload.
- Android: updated native debug APK built; 12 JVM API tests passed, including cook-log payloads, edits using PUT, nullable shared ratings, summary decoding, photo requests, weekly wishes, and refreshed access tokens. Lint passed with zero errors; dependency/style warnings remain.
- MCP: official TypeScript SDK discovered 20 tools and verified shopping/wishlists plus shared recipe ratings, cook-log create/read/update/delete, cook count, averages, and independence of shared ratings.
- Memory: release server measured 6.30 MiB initial RSS and 7.33 MiB after 100 recipes, 100 cook logs, 7 planned meals, shopping additions, and 200 reads. OIDC/photo processing and client/provider memory are excluded from this smoke measurement.
- No live Pocket ID passkey login or Android device/emulator UI run was performed. The same two-account allowlist/equal permissions apply to all cook/rating endpoints.

Schema version 3 adds cook history and shared ratings without replacing existing recipe/photo/wishlist tables. JSON exports and SQLite backups include cook ratings, names, dates, and reflection notes. Photos remain excluded from JSON exports.

Recetaco rename verification: Rust tests and Clippy, React production build/formatting, all 7 browser scenarios, Android debug build/JVM tests/lint passed. Android application ID and Pocket ID callback now use `dev.recetaco.app`; server configuration uses `RECETACO_`.
