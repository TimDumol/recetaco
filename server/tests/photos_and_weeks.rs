use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use recetaco::{
    AppState, app,
    db::{self, Store},
};
use serde_json::{Value, json};
use tower::ServiceExt;
async fn send(app: &Router, method: &str, path: &str, data: Value) -> (StatusCode, Value) {
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(data.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = r.status();
    let b = r.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&b).unwrap_or(Value::Null))
}
fn recipe() -> Value {
    json!({"title":"Rice bowl","servings":2,"ingredients":[{"name":"Rice","quantity":100,"unit":"g"}]})
}
#[tokio::test]
async fn weekly_wishes_need_no_day_and_shop_only_uncooked_meals() {
    let store = Store::open(":memory:").unwrap();
    let app = app(AppState::new(store, None), "/missing");
    let (_, r) = send(&app, "POST", "/api/recipes", recipe()).await;
    let (_, wish) = send(
        &app,
        "POST",
        "/api/wishes",
        json!({"recipe_id":r["id"],"week":"2026-10-25","servings":4}),
    )
    .await;
    assert_eq!(wish["week"], "2026-10-19");
    assert_eq!(wish["cooked"], false);
    let (_, cooked) = send(
        &app,
        "POST",
        "/api/wishes",
        json!({"recipe_id":r["id"],"week":"2026-10-19","servings":10}),
    )
    .await;
    assert_eq!(
        send(
            &app,
            "PATCH",
            &format!("/api/wishes/{}", cooked["id"]),
            json!({"cooked":true})
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    let (_, list) = send(&app, "POST", "/api/lists", json!({"name":"This week"})).await;
    let (s, list) = send(
        &app,
        "POST",
        &format!("/api/lists/{}/weeks/2026-10-24", list["id"]),
        json!({}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(list["items"][0]["quantity"], 200.0);
    let (_, other) = send(&app, "GET", "/api/wishes?week=2026-10-26", Value::Null).await;
    assert_eq!(other, json!([]));
    let (_, export) = send(&app, "GET", "/api/export", Value::Null).await;
    assert_eq!(export["wishes"].as_array().unwrap().len(), 2);
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/wishes",
            json!({"recipe_id":r["id"],"week":"2026-02-30","servings":2})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/wishes",
            json!({"recipe_id":r["id"],"week":"2026-10-24","servings":0})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(db::week_start("2027-01-01").unwrap(), "2026-12-28");
    send(
        &app,
        "DELETE",
        &format!("/api/recipes/{}", r["id"]),
        Value::Null,
    )
    .await;
    assert_eq!(
        send(&app, "GET", "/api/wishes?week=2026-10-19", Value::Null)
            .await
            .1,
        json!([])
    );
}
#[tokio::test]
async fn whole_week_addition_is_atomic() {
    let app = app(
        AppState::new(Store::open(":memory:").unwrap(), None),
        "/missing",
    );
    let (_, r) = send(&app, "POST", "/api/recipes", recipe()).await;
    send(
        &app,
        "POST",
        "/api/wishes",
        json!({"recipe_id":r["id"],"week":"2026-10-19","servings":2}),
    )
    .await;
    let (_,big)=send(&app,"POST","/api/recipes",json!({"title":"Too big","servings":1,"ingredients":[{"name":"Overflow","quantity":1000000}]})).await;
    send(
        &app,
        "POST",
        "/api/wishes",
        json!({"recipe_id":big["id"],"week":"2026-10-19","servings":2}),
    )
    .await;
    let (_, list) = send(&app, "POST", "/api/lists", json!({"name":"Atomic"})).await;
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("/api/lists/{}/weeks/2026-10-19", list["id"]),
            json!({})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(
            &app,
            "GET",
            &format!("/api/lists/{}", list["id"]),
            Value::Null
        )
        .await
        .1["items"],
        json!([])
    );
}
fn jpeg() -> Vec<u8> {
    let rgb = image::RgbImage::from_pixel(1400, 700, image::Rgb([130, 100, 40]));
    let mut bytes = vec![];
    image::codecs::jpeg::JpegEncoder::new(&mut bytes)
        .encode_image(&rgb)
        .unwrap();
    bytes
}
#[tokio::test]
async fn photos_resize_replace_remove_and_require_authentication() {
    let app = app(
        AppState::new(Store::open(":memory:").unwrap(), None),
        "/missing",
    );
    let (_, r) = send(&app, "POST", "/api/recipes", recipe()).await;
    let path = format!("/api/recipes/{}/photo", r["id"]);
    for version in [1, 2] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(&path)
                    .header("content-type", "image/jpeg")
                    .body(Body::from(jpeg()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(json["photo_version"], version);
    }
    let response = app
        .clone()
        .oneshot(Request::builder().uri(&path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.headers()["content-type"], "image/jpeg");
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let image = image::load_from_memory(&bytes).unwrap();
    assert_eq!((image.width(), image.height()), (1200, 600));
    let (_, recipes) = send(&app, "GET", "/api/recipes", Value::Null).await;
    assert_eq!(recipes[0]["photo_version"], 2);
    let invalid = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(&path)
                .body(Body::from("<svg><script>alert(1)</script></svg>"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    send(&app, "DELETE", &path, Value::Null).await;
    assert_eq!(
        send(&app, "GET", &path, Value::Null).await.0,
        StatusCode::NOT_FOUND
    );
    let guarded = recetaco::app(
        AppState::new(Store::open(":memory:").unwrap(), Some("secret".into())),
        "/missing",
    );
    assert_eq!(
        send(&guarded, "GET", "/api/recipes/1/photo", Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let response = guarded
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/recipes/1/photo")
                .body(Body::from(jpeg()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
#[tokio::test]
async fn existing_v1_database_is_preserved_and_upgraded() {
    let path = std::env::temp_dir().join(format!(
        "recetaco-migration-{}-{}.db",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    {
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection.execute_batch("CREATE TABLE recipes(id INTEGER PRIMARY KEY,data TEXT NOT NULL);PRAGMA user_version=1;").unwrap();
        connection
            .execute(
                "INSERT INTO recipes(id,data) VALUES(1,?)",
                [recipe().to_string()],
            )
            .unwrap();
    }
    let store = Store::open(path.to_str().unwrap()).unwrap();
    store
        .run(|db| {
            assert_eq!(db::recipe(db, 1)?.photo_version, 0);
            assert!(db::wishes(db, "2026-10-19")?.is_empty());
            assert_eq!(
                db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))?,
                3
            );
            Ok(())
        })
        .await
        .unwrap();
    drop(store);
    let _ = std::fs::remove_file(path);
}
