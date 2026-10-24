pub mod auth;
pub mod db;
mod mcp;
pub mod model;
pub mod photos;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
};
use model::*;
use serde_json::{Value, json};
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub struct Error {
    status: StatusCode,
    message: String,
}
impl Error {
    pub fn bad(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }
    pub fn not_found() -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: "Not found".into(),
        }
    }
    pub fn internal() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Storage operation failed".into(),
        }
    }
}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        eprintln!("SQLite: {e}");
        Self::internal()
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        eprintln!("JSON: {e}");
        Self::internal()
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (self.status, Json(json!({"error":self.message}))).into_response()
    }
}
#[derive(Clone)]
pub struct AppState {
    pub store: db::Store,
    pub token: Option<Arc<str>>,
    pub oidc: Option<Arc<auth::Oidc>>,
}
impl AppState {
    pub fn new(store: db::Store, token: Option<String>) -> Self {
        Self {
            store,
            token: token.map(Arc::from),
            oidc: None,
        }
    }
}
async fn protect(
    State(s): State<AppState>,
    headers: HeaderMap,
    mut request: axum::extract::Request,
    next: Next,
) -> Response {
    // Refuse cross-origin browser requests; native/API clients do not send Origin.
    if let Some(origin) = headers.get("origin") {
        let host = headers
            .get("host")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let origin = origin.to_str().unwrap_or("");
        if origin != format!("http://{host}") && origin != format!("https://{host}") {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"error":"Cross-origin requests are not allowed"})),
            )
                .into_response();
        }
    }
    if let Some(oidc) = &s.oidc {
        let supplied = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "));
        match supplied {
            Some(token) => {
                match oidc.validate(token).await {
                    Ok(principal) => {
                        request.extensions_mut().insert(principal);
                    }
                    Err(auth::AuthError::Forbidden) => {
                        return (
                            StatusCode::FORBIDDEN,
                            Json(json!({"error":"This account is not a member of this kitchen"})),
                        )
                            .into_response();
                    }
                    Err(auth::AuthError::Unavailable) => return (
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(json!({"error":"Pocket ID signing keys are temporarily unavailable"})),
                    )
                        .into_response(),
                    Err(auth::AuthError::Invalid) => {
                        return (
                            StatusCode::UNAUTHORIZED,
                            [(axum::http::header::WWW_AUTHENTICATE, oidc.challenge())],
                            Json(json!({"error":"Sign in with Pocket ID"})),
                        )
                            .into_response();
                    }
                }
            }
            None => {
                return (
                    StatusCode::UNAUTHORIZED,
                    [(axum::http::header::WWW_AUTHENTICATE, oidc.challenge())],
                    Json(json!({"error":"Sign in with Pocket ID"})),
                )
                    .into_response();
            }
        }
    } else if let Some(token) = &s.token {
        let supplied = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .unwrap_or("");
        if !constant_eq(supplied.as_bytes(), token.as_bytes()) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error":"A valid API token is required"})),
            )
                .into_response();
        }
    }
    next.run(request).await
}
fn constant_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |v, (a, b)| v | (a ^ b)) == 0
}
pub fn app(state: AppState, web_dir: &str) -> Router {
    let api = Router::new()
        .route("/api/recipes", get(all_recipes).post(create_recipe))
        .route(
            "/api/recipes/{id}",
            get(one_recipe).put(update_recipe).delete(remove_recipe),
        )
        .route(
            "/api/recipes/{id}/photo",
            get(photos::get)
                .put(photos::upload)
                .delete(photos::delete)
                .layer(DefaultBodyLimit::max(5 * 1024 * 1024)),
        )
        .route("/api/recipes/{id}/rating", axum::routing::put(rate_recipe))
        .route(
            "/api/recipes/{id}/cooks",
            get(recipe_cooks).post(create_cook),
        )
        .route(
            "/api/recipes/{id}/cooks/{cook_id}",
            axum::routing::put(update_cook).delete(remove_cook),
        )
        .route("/api/wishes", get(all_wishes).post(create_wish))
        .route("/api/wishes/{id}", patch(cook_wish).delete(remove_wish))
        .route("/api/lists/{id}/weeks/{week}", post(add_week))
        .route("/api/meals", get(all_meals).post(create_meal))
        .route("/api/meals/{id}", axum::routing::delete(remove_meal))
        .route("/api/lists", get(all_lists).post(new_list))
        .route("/api/lists/{id}", get(one_list).delete(remove_list))
        .route("/api/lists/{id}/items", post(new_item))
        .route(
            "/api/lists/{id}/items/{item_id}",
            patch(check_item).delete(remove_item),
        )
        .route("/api/lists/{id}/recipes/{recipe_id}", post(add_recipe))
        .route("/api/export", get(export))
        .route("/mcp", post(mcp::handle))
        .route_layer(middleware::from_fn_with_state(state.clone(), protect));
    Router::new()
        .merge(api)
        .route("/auth/config", get(auth::config))
        .route(
            "/.well-known/oauth-protected-resource",
            get(auth::resource_metadata),
        )
        .route(
            "/.well-known/oauth-protected-resource/mcp",
            get(auth::resource_metadata),
        )
        .route(
            "/health",
            get(|| async { Json(json!({"status":"ok","version":env!("CARGO_PKG_VERSION")})) }),
        )
        .fallback_service(
            ServeDir::new(web_dir)
                .not_found_service(ServeFile::new(format!("{web_dir}/index.html"))),
        )
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .with_state(state)
}
async fn all_recipes(State(s): State<AppState>) -> Result<Json<Vec<Recipe>>> {
    Ok(Json(s.store.run(|db| db::recipes(db)).await?))
}
async fn one_recipe(State(s): State<AppState>, Path(id): Path<i64>) -> Result<Json<Recipe>> {
    Ok(Json(s.store.run(move |db| db::recipe(db, id)).await?))
}
async fn create_recipe(
    State(s): State<AppState>,
    Json(data): Json<RecipeInput>,
) -> Result<(StatusCode, Json<Recipe>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            s.store
                .run(move |db| db::save_recipe(db, None, data))
                .await?,
        ),
    ))
}
async fn update_recipe(
    State(s): State<AppState>,
    Path(id): Path<i64>,
    Json(data): Json<RecipeInput>,
) -> Result<Json<Recipe>> {
    Ok(Json(
        s.store
            .run(move |db| db::save_recipe(db, Some(id), data))
            .await?,
    ))
}
async fn remove_recipe(State(s): State<AppState>, Path(id): Path<i64>) -> Result<StatusCode> {
    s.store.run(move |db| db::delete_recipe(db, id)).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn all_meals(State(s): State<AppState>) -> Result<Json<Vec<Meal>>> {
    Ok(Json(s.store.run(|db| db::meals(db)).await?))
}
async fn create_meal(
    State(s): State<AppState>,
    Json(data): Json<MealInput>,
) -> Result<(StatusCode, Json<Meal>)> {
    Ok((
        StatusCode::CREATED,
        Json(s.store.run(move |db| db::add_meal(db, data)).await?),
    ))
}
async fn remove_meal(State(s): State<AppState>, Path(id): Path<i64>) -> Result<StatusCode> {
    s.store.run(move |db| db::delete_meal(db, id)).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn all_lists(State(s): State<AppState>) -> Result<Json<Vec<ShoppingList>>> {
    Ok(Json(s.store.run(|db| db::lists(db)).await?))
}
async fn new_list(
    State(s): State<AppState>,
    Json(data): Json<ListInput>,
) -> Result<(StatusCode, Json<ShoppingList>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            s.store
                .run(move |db| db::create_list(db, data.name))
                .await?,
        ),
    ))
}
async fn one_list(State(s): State<AppState>, Path(id): Path<i64>) -> Result<Json<ListDetail>> {
    Ok(Json(s.store.run(move |db| db::list(db, id)).await?))
}
async fn remove_list(State(s): State<AppState>, Path(id): Path<i64>) -> Result<StatusCode> {
    s.store.run(move |db| db::delete_list(db, id)).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn new_item(
    State(s): State<AppState>,
    Path(id): Path<i64>,
    Json(data): Json<Ingredient>,
) -> Result<Json<ListDetail>> {
    Ok(Json(
        s.store.run(move |db| db::add_item(db, id, data)).await?,
    ))
}
async fn check_item(
    State(s): State<AppState>,
    Path((id, item_id)): Path<(i64, i64)>,
    Json(data): Json<CheckInput>,
) -> Result<StatusCode> {
    s.store
        .run(move |db| db::check_item(db, id, item_id, data.checked))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn remove_item(
    State(s): State<AppState>,
    Path((id, item_id)): Path<(i64, i64)>,
) -> Result<StatusCode> {
    s.store
        .run(move |db| db::delete_item(db, id, item_id))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn add_recipe(
    State(s): State<AppState>,
    Path((id, recipe_id)): Path<(i64, i64)>,
    Json(data): Json<AddRecipeInput>,
) -> Result<Json<ListDetail>> {
    Ok(Json(
        s.store
            .run(move |db| db::add_recipe(db, id, recipe_id, data.servings))
            .await?,
    ))
}
async fn export(State(s): State<AppState>) -> Result<Json<Value>> {
    Ok(Json(s.store.run(|db| { let lists=db::lists(db)?.into_iter().map(|l|db::list(db,l.id)).collect::<Result<Vec<_>>>()?; Ok(json!({"version":3,"cooks":db::all_cooks(db)?,"recipes":db::recipes(db)?,"meals":db::meals(db)?,"wishes":db::all_wishes(db)?,"lists":lists})) }).await?))
}

async fn all_wishes(
    State(s): State<AppState>,
    Query(q): Query<WishQuery>,
) -> Result<Json<Vec<Wish>>> {
    Ok(Json(s.store.run(move |db| db::wishes(db, &q.week)).await?))
}
async fn create_wish(
    State(s): State<AppState>,
    Json(data): Json<WishInput>,
) -> Result<(StatusCode, Json<Wish>)> {
    Ok((
        StatusCode::CREATED,
        Json(s.store.run(move |db| db::add_wish(db, data)).await?),
    ))
}
async fn cook_wish(
    State(s): State<AppState>,
    Path(id): Path<i64>,
    Json(data): Json<CookedInput>,
) -> Result<StatusCode> {
    s.store
        .run(move |db| db::cook_wish(db, id, data.cooked))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn remove_wish(State(s): State<AppState>, Path(id): Path<i64>) -> Result<StatusCode> {
    s.store.run(move |db| db::delete_wish(db, id)).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn add_week(
    State(s): State<AppState>,
    Path((id, week)): Path<(i64, String)>,
) -> Result<Json<ListDetail>> {
    Ok(Json(
        s.store.run(move |db| db::add_week(db, id, &week)).await?,
    ))
}

async fn rate_recipe(
    State(s): State<AppState>,
    Path(id): Path<i64>,
    Json(data): Json<RatingInput>,
) -> Result<Json<Recipe>> {
    Ok(Json(
        s.store
            .run(move |db| db::rate_recipe(db, id, data.rating))
            .await?,
    ))
}
async fn recipe_cooks(State(s): State<AppState>, Path(id): Path<i64>) -> Result<Json<Vec<Cook>>> {
    Ok(Json(s.store.run(move |db| db::cooks(db, id)).await?))
}
async fn create_cook(
    State(s): State<AppState>,
    Path(id): Path<i64>,
    Json(data): Json<CookInput>,
) -> Result<(StatusCode, Json<Cook>)> {
    Ok((
        StatusCode::CREATED,
        Json(
            s.store
                .run(move |db| db::save_cook(db, id, None, data))
                .await?,
        ),
    ))
}
async fn update_cook(
    State(s): State<AppState>,
    Path((id, cook_id)): Path<(i64, i64)>,
    Json(data): Json<CookInput>,
) -> Result<Json<Cook>> {
    Ok(Json(
        s.store
            .run(move |db| db::save_cook(db, id, Some(cook_id), data))
            .await?,
    ))
}
async fn remove_cook(
    State(s): State<AppState>,
    Path((id, cook_id)): Path<(i64, i64)>,
) -> Result<StatusCode> {
    s.store
        .run(move |db| db::delete_cook(db, id, cook_id))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
