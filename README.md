# Recetaco

A small, self-hosted recipe planner with shopping lists. Rust + SQLite server, React browser client, native Kotlin/Jetpack Compose Android client, and built-in MCP. MIT licensed.

## Renaming an existing installation

The project was previously called Hearth. Configuration variables now use `RECETACO_` and the executable is `recetaco`. To keep an existing database, set `RECETACO_DB` to its existing path (for example `hearth.db`) or stop the server and rename the database together with its SQLite sidecar files. The schema and stored recipes are unchanged. Android now uses application ID `dev.recetaco.app`; install it as a new app, reconnect, and register `dev.recetaco.app:/oauth2redirect` in the Pocket ID Android client.

## What works

- Create, read, edit, search, and delete recipes with structured ingredients and serving counts.
- Give a recipe a shared 1–5 rating and keep a cook history: date, optional rating, cook name, went well, to improve, and general notes.
- See cook counts, average cook ratings, and last-cooked dates; edit or delete past cooks.
- Keep a flexible weekly wishlist with servings and cooked checkboxes; add all uncooked meals’ ingredients to a shopping list.
- Optional plans for exact dates and meal slots.
- Upload, replace, and remove a recipe photo in the browser or Android app. Photos appear on recipe cards and details.
- Create shared shopping lists. Add individual items or scaled recipe ingredients, check items off, and remove items.
- Matching unchecked ingredient names and units merge; checked items remain separate. No pantry inventory is required. Repeated additions add quantities again.
- Browser JSON export; server API and MCP share the same validation and database operations.
- Direct Pocket ID OIDC sign-in for browser and Android, using authorization code flow with PKCE.
- Access tokens validated by the Rust API and MCP endpoint; Android authorization state encrypted with Android Keystore.

This first version is an online household app with Pocket ID authentication and one shared kitchen with equal full permissions for both partners. Clients refresh on their own writes; use Refresh/reload to see changes from another client. There is no offline synchronization, separate kitchens or roles, automatic website scraping, ingredient language parsing, nutrition database, or unit conversion. The optional source URL is a reference, not an importer. MCP can create a recipe from already structured content.

## Run locally

Requirements: Rust stable (edition 2024), Node.js 22.12+ and npm. No external database or Node runtime is needed after building.

```sh
cd web
npm ci
npm run build
cd ..
cargo build --release --locked
./target/release/recetaco
```

Open **http://127.0.0.1:8080**. The server serves `web/dist` and creates `recetaco.db` in its working directory. The recipe book starts empty; the optional sample button saves a real recipe.

For development, `scripts/dev.sh` starts the Rust API and Vite at http://127.0.0.1:5173. Vite proxies `/api` to port 8080. Build the web client once before using the Rust-served UI on port 8080.

### Pocket ID authentication

See **[Pocket ID setup](docs/pocket-id.md)** for provider requirements, exact callback URLs, API audience, browser/Android clients, and MCP OAuth integration.

```sh
export RECETACO_OIDC_ISSUER=https://id.example.com
export RECETACO_OIDC_CLIENT_ID=YOUR_BROWSER_CLIENT_ID
export RECETACO_OIDC_ANDROID_CLIENT_ID=YOUR_ANDROID_CLIENT_ID
export RECETACO_OIDC_AUDIENCE=https://recetaco.example.com
./target/release/recetaco
```

Use Recetaco's public HTTPS address on your phone. The default listener is loopback; a reverse proxy can provide HTTPS and preserve the public Host header. No authentication gateway is required. A non-loopback listener requires OIDC or a development token. Without OIDC, local development remains available; Android exposes development connection options.

| Environment variable | Default |
| --- | --- |
| `RECETACO_BIND` | `127.0.0.1:8080` |
| `RECETACO_DB` | `recetaco.db` |
| `RECETACO_WEB_DIR` | `web/dist` |
| `RECETACO_OIDC_ISSUER` | unset; enables Pocket ID when provided |
| `RECETACO_OIDC_CLIENT_ID` | required with OIDC |
| `RECETACO_OIDC_ANDROID_CLIENT_ID` | browser client ID; separate client recommended |
| `RECETACO_OIDC_AUDIENCE` | required with OIDC; Recetaco public HTTPS origin |
| `RECETACO_OIDC_ALLOWED_SUBJECTS` | optional comma-separated Pocket ID subject IDs; set to your two accounts |
| `RECETACO_TOKEN` | legacy development mode only; cannot be combined with OIDC |

Keep the database on local storage. SQLite uses WAL, a 2 MiB page-cache target, and foreign keys. Database operations run on blocking workers, serialized through one connection. The server uses two Tokio runtime workers.

## Native Android

Open `android/` in Android Studio. Install Android SDK 35 and use JDK 17, then build:

```sh
cd android
./gradlew :app:assembleDebug :app:testDebugUnitTest
```

APK: `android/app/build/outputs/apk/debug/app-debug.apk`. Minimum Android version: 8.0/API 26. This is a native Compose application, not a WebView. Set the HTTPS server URL in Connection settings and sign in with Pocket ID. Debug APKs are for local installation; configure your own signing for distribution.

## MCP

Endpoint: **`POST /mcp`** on the same server. Stateless Streamable HTTP, JSON responses, no SSE stream or session affinity. GET/SSE subscriptions are not offered. Supports protocol negotiation for `2025-03-26`, `2025-06-18`, and `2025-11-25`.

The MCP endpoint advertises OAuth protected-resource metadata for Pocket ID. Register an OAuth client as described in the [Pocket ID setup](docs/pocket-id.md#mcp-and-other-api-clients). Header-only clients can use a separately obtained resource access token (renewal is the client’s responsibility):

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

Tools: `rate_recipe`, `list_cooks`, `log_cook`, `update_cook`, `delete_cook`, `list_weekly_wishlist`, `add_to_weekly_wishlist`, `mark_wishlist_cooked`, `add_week_to_shopping_list`, `list_recipes`, `get_recipe`, `create_recipe`, `list_meals`, `plan_meal`, `list_shopping_lists`, `create_shopping_list`, `get_shopping_list`, `add_recipe_to_shopping_list`, `add_shopping_item`, `check_shopping_item`.

Example: “Add the rice bowl for four to next week’s wishlist, and add the uncooked meals’ ingredients to This week.” The client first discovers recipe/list IDs, then calls the write tools. Recipe additions are additive and must not be blindly retried after a timeout.

## HTTP API

All write payloads use JSON. Bearer authentication applies to `/api/*` and `/mcp`; `/health`, `/auth/config`, OAuth resource metadata, and the static UI are public. Maximum request body: 1 MiB for JSON, 5 MiB for photo uploads.

| Method | Route | Purpose |
| --- | --- | --- |
| GET / POST | `/api/recipes` | List / create recipes |
| GET / PUT / DELETE | `/api/recipes/{id}` | Read / replace / delete recipe |
| PUT | `/api/recipes/{id}/rating` | Set shared `{rating: 1..5}` or `{rating: null}` |
| GET / POST | `/api/recipes/{id}/cooks` | Read history / log a cooking occasion |
| PUT / DELETE | `/api/recipes/{id}/cooks/{cook_id}` | Replace / remove a cook entry |
| GET / PUT / DELETE | `/api/recipes/{id}/photo` | Read / replace / remove the protected JPEG |
| GET / POST | `/api/wishes?week=YYYY-MM-DD` | List a week / add a wish (POST uses `/api/wishes`) |
| PATCH / DELETE | `/api/wishes/{id}` | Set `{cooked}` / remove a wish |
| POST | `/api/lists/{id}/weeks/{week}` | Add all uncooked wishlist ingredients |
| GET / POST | `/api/meals` | List / schedule meals |
| DELETE | `/api/meals/{id}` | Remove a plan entry |
| GET / POST | `/api/lists` | List / create shopping lists |
| GET / DELETE | `/api/lists/{id}` | Read with items / delete list |
| POST | `/api/lists/{id}/items` | Add `{name, quantity, unit}` |
| PATCH / DELETE | `/api/lists/{id}/items/{item_id}` | Set `{checked}` / remove item |
| POST | `/api/lists/{id}/recipes/{recipe_id}` | Add ingredients with `{servings}` |
| GET | `/api/export` | Consistent JSON snapshot |

Recipes use `{title, servings, ingredients: [{name, quantity, unit}], instructions, source_url}`. Quantities and servings must be finite, positive, and at most 1,000,000. Units may be blank. Meal input: `{recipe_id, date: "YYYY-MM-DD", servings, slot}`; slots: breakfast/lunch/dinner/snack. List input: `{name}`. Deleting a recipe removes its rating, cook history, photo, weekly wishes, and dated meal entries. Existing shopping items remain.

Weekly wish input: `{recipe_id, week: "YYYY-MM-DD", servings}`. Any day in a week identifies that Monday-to-Sunday week; storage and responses use the Monday date. Weeks can be browsed into the future or past. Marking a meal cooked does not modify existing shopping items. Shopping for a week is additive, so repeating it adds ingredients again.

## Ratings and cook history

Open a recipe to set its shared **Recipe rating**, or choose **Log a cook** to record a cooking occasion. Both partners have equal permission to edit and delete entries. Each cook has a YYYY-MM-DD date, an optional integer rating from 1 to 5, an optional cooked-by name, and three reflection fields: **Went well**, **To improve**, and **Notes**. Multiple cooks on the same date are allowed. Entries sort by cook date, newest first.

The shared recipe rating is independent of individual cook ratings. Recipe summaries include `rating`, `cook_count`, `average_cook_rating`, and `last_cooked`. Unrated cooks still count but are excluded from the average. With no rated cooks the average is null; with no cooks the last date is null. Counts and averages recalculate after edits/deletions. Changing recipe metadata or photos preserves history and ratings.

Cook payload example:

```json
{
  "cooked_on": "2026-10-24",
  "rating": 4,
  "cook_name": "Both",
  "went_well": "Rice texture was right",
  "to_improve": "Use less salt next time",
  "notes": "Used vegetable stock instead of water"
}
```

Only `cooked_on` is required. Omit `rating` or use null for an unrated cook. A PUT replaces the entire entry: send all notes you want to keep. The cook name is a display label, not a separate account or owner. Each notes field is limited to 10,000 bytes; the name to 100 bytes. Wishlist checkmarks do not create or delete cook logs. Logging a cook does not automatically mark a wishlist item cooked or alter shopping items.

Logging is additive: each POST/MCP `log_cook` creates a new occasion. Check history before retrying a write whose outcome is unknown. JSON exports and full SQLite backups include all ratings, dates, and notes. Existing databases upgrade automatically to schema version 3.

## Photos and memory

Each recipe can have one uploaded photo. The clients resize photos before uploading. The server accepts JPEG/PNG/WebP, decodes with dimension/allocation limits, fits within 1200 × 1200, and re-encodes JPEG without the source metadata. Photo processing is serialized to limit concurrent memory spikes. Uploads are limited to 5 MiB and stored JPEGs to 2 MiB each; the decoder has a 32 MiB allocation limit. Processing can temporarily use more memory than idle recipe browsing.

Photos are stored in SQLite separately from recipe JSON and served through the authenticated API. Recipe list responses contain only a photo revision, not image bytes. Complete SQLite backups include photos; JSON exports contain recipe metadata and wishlist entries but omit image binaries. Existing databases gain the new tables automatically without dropping recipes.

## Backups

Use **Export JSON** in the browser for a readable snapshot. Automatic JSON restore is not implemented. For a restorable backup, stop the server, copy `recetaco.db`, and restart. Restore by stopping the server and restoring that file, ensuring stale `-wal`/`-shm` files from a different database are absent. For live backups use SQLite’s backup API or `.backup`, rather than copying only the main database file while it is open.

## Verification

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cd web
npm ci
npm run build
npx playwright install chromium
npm run test:e2e
```

Browser tests also verify a full OIDC code + PKCE sign-in, authenticated API access, persistence, and sign-out against a test provider. They start isolated real Rust servers (build `cargo build` first) and run the recipe → plan → scaled shopping list → check-off → reload workflow at desktop and phone viewport sizes. Android unit tests cover wire compatibility, refreshed access-token headers, auth configuration, serving payloads, URL validation, and error decoding.

For official MCP SDK interoperability, start a disposable server on port 18081, then run `node tests/mcp-smoke.mjs` from `web/`. The smoke test creates a recipe and shopping list. Override its endpoint with `RECETACO_MCP_URL` and optionally `RECETACO_TOKEN`.

On Linux, after a release build, run `python3 scripts/measure-memory.py` to measure server RSS against an isolated temporary database. It reports initial RSS and resident/high-water memory after 100 recipes, 7 meals, 100 cook logs, ingredient additions, and 200 reads. This excludes browser/Android memory and is a small-workload measurement, not a large-library guarantee.

### Initial memory measurement

The included [smoke measurement](artifacts/memory.json) records RSS for the Linux x86_64 release server in local development mode (OIDC disabled). It is not a capacity test and excludes client applications.
