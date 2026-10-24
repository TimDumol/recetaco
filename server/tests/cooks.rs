use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use recetaco::{AppState, app, db::Store};
use serde_json::{Value, json};
use tower::ServiceExt;
async fn send(app: &Router, method: &str, path: &str, value: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .header("host", "localhost")
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(value.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}
async fn setup() -> (Router, i64) {
    let app = app(
        AppState::new(Store::open(":memory:").unwrap(), None),
        "/missing",
    );
    let (_,r)=send(&app,"POST","/api/recipes",json!({"title":"Rice","servings":2,"ingredients":[{"name":"Rice","quantity":100,"unit":"g"}]})).await;
    (app, r["id"].as_i64().unwrap())
}
#[tokio::test]
async fn ratings_and_cook_stats_are_independent_and_recalculate_after_edits() {
    let (app, id) = setup().await;
    let path = format!("/api/recipes/{id}");
    let (_, r) = send(&app, "GET", &path, Value::Null).await;
    assert_eq!(r["cook_count"], 0);
    assert!(r["rating"].is_null());
    assert!(r["average_cook_rating"].is_null());
    assert!(r["last_cooked"].is_null());
    let (_, r) = send(&app, "PUT", &format!("{path}/rating"), json!({"rating":4})).await;
    assert_eq!(r["rating"], 4);
    let (s,a)=send(&app,"POST",&format!("{path}/cooks"),json!({"cooked_on":"2026-10-20","rating":5,"cook_name":" Both ","went_well":" Texture ","to_improve":"Less salt","notes":"Used stock"})).await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(a["cook_name"], "Both");
    let (_, b) = send(
        &app,
        "POST",
        &format!("{path}/cooks"),
        json!({"cooked_on":"2026-10-24","rating":3}),
    )
    .await;
    let (_, c) = send(
        &app,
        "POST",
        &format!("{path}/cooks"),
        json!({"cooked_on":"2026-10-21"}),
    )
    .await;
    let (_, r) = send(&app, "GET", &path, Value::Null).await;
    assert_eq!(r["rating"], 4);
    assert_eq!(r["cook_count"], 3);
    assert_eq!(r["average_cook_rating"], 4.0);
    assert_eq!(r["last_cooked"], "2026-10-24");
    let (_, history) = send(&app, "GET", &format!("{path}/cooks"), Value::Null).await;
    assert_eq!(history[0]["id"], b["id"]);
    assert_eq!(history[2]["went_well"], "Texture");
    let (_,edited)=send(&app,"PUT",&format!("{path}/cooks/{}",a["id"]),json!({"cooked_on":"2026-10-19","rating":1,"went_well":"Soft rice","to_improve":"More water","notes":"New notes"})).await;
    assert_eq!(edited["notes"], "New notes");
    let (_, r) = send(&app, "GET", &path, Value::Null).await;
    assert_eq!(r["cook_count"], 3);
    assert_eq!(r["average_cook_rating"], 2.0);
    send(
        &app,
        "DELETE",
        &format!("{path}/cooks/{}", b["id"]),
        Value::Null,
    )
    .await;
    let (_, r) = send(&app, "GET", &path, Value::Null).await;
    assert_eq!(r["cook_count"], 2);
    assert_eq!(r["average_cook_rating"], 1.0);
    assert_eq!(r["last_cooked"], "2026-10-21");
    send(
        &app,
        "DELETE",
        &format!("{path}/cooks/{}", a["id"]),
        Value::Null,
    )
    .await;
    let (_, r) = send(&app, "GET", &path, Value::Null).await;
    assert_eq!(r["cook_count"], 1);
    assert!(r["average_cook_rating"].is_null());
    send(
        &app,
        "DELETE",
        &format!("{path}/cooks/{}", c["id"]),
        Value::Null,
    )
    .await;
    let (_, r) = send(
        &app,
        "PUT",
        &format!("{path}/rating"),
        json!({"rating":null}),
    )
    .await;
    assert_eq!(r["cook_count"], 0);
    assert!(r["rating"].is_null());
    assert!(r["last_cooked"].is_null());
}
#[tokio::test]
async fn invalid_logs_are_rejected_and_cannot_modify_another_recipe() {
    let (app, id) = setup().await;
    let path = format!("/api/recipes/{id}");
    for value in [json!(0), json!(6), json!(-1), json!(3.5)] {
        assert!(
            !send(
                &app,
                "PUT",
                &format!("{path}/rating"),
                json!({"rating":value})
            )
            .await
            .0
            .is_success()
        );
        assert!(
            !send(
                &app,
                "POST",
                &format!("{path}/cooks"),
                json!({"cooked_on":"2026-10-24","rating":value})
            )
            .await
            .0
            .is_success()
        );
    }
    for date in ["bad", "2026-02-30", "2026-1-2"] {
        assert_eq!(
            send(
                &app,
                "POST",
                &format!("{path}/cooks"),
                json!({"cooked_on":date})
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        send(
            &app,
            "POST",
            &format!("{path}/cooks"),
            json!({"cooked_on":"2026-10-24","notes":"x".repeat(10001)})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(&app, "GET", "/api/recipes/999/cooks", Value::Null)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let (_, log) = send(
        &app,
        "POST",
        &format!("{path}/cooks"),
        json!({"cooked_on":"2026-10-24","notes":"Keep me"}),
    )
    .await;
    let (_, other) = send(
        &app,
        "POST",
        "/api/recipes",
        json!({"title":"Other","servings":1,"ingredients":[{"name":"Milk","quantity":1}]}),
    )
    .await;
    let wrong = format!("/api/recipes/{}/cooks/{}", other["id"], log["id"]);
    assert_eq!(
        send(
            &app,
            "PUT",
            &wrong,
            json!({"cooked_on":"2026-10-24","notes":"Changed"})
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&app, "DELETE", &wrong, Value::Null).await.0,
        StatusCode::NOT_FOUND
    );
    let (_, history) = send(&app, "GET", &format!("{path}/cooks"), Value::Null).await;
    assert_eq!(history[0]["notes"], "Keep me");
}
#[tokio::test]
async fn logs_and_ratings_export_and_cascade_while_metadata_edits_preserve_them() {
    let (app, id) = setup().await;
    let path = format!("/api/recipes/{id}");
    send(&app, "PUT", &format!("{path}/rating"), json!({"rating":5})).await;
    send(
        &app,
        "POST",
        &format!("{path}/cooks"),
        json!({"cooked_on":"2026-10-24","rating":4,"notes":"Remember this"}),
    )
    .await;
    let (_, r) = send(
        &app,
        "PUT",
        &path,
        json!({"title":"Renamed rice","servings":3,"ingredients":[{"name":"Rice","quantity":120}]}),
    )
    .await;
    assert_eq!(r["rating"], 5);
    assert_eq!(r["cook_count"], 1);
    let (_, export) = send(&app, "GET", "/api/export", Value::Null).await;
    assert_eq!(export["version"], 4);
    assert_eq!(export["cooks"][0]["notes"], "Remember this");
    assert_eq!(export["recipes"][0]["average_cook_rating"], 4.0);
    send(&app, "DELETE", &path, Value::Null).await;
    let (_, export) = send(&app, "GET", "/api/export", Value::Null).await;
    assert_eq!(export["cooks"], json!([]));
}
#[tokio::test]
async fn rating_and_cook_endpoints_require_authentication() {
    let app = app(
        AppState::new(Store::open(":memory:").unwrap(), Some("secret".into())),
        "/missing",
    );
    for (method, path, data) in [
        ("PUT", "/api/recipes/1/rating", json!({"rating":5})),
        ("GET", "/api/recipes/1/cooks", Value::Null),
        (
            "POST",
            "/api/recipes/1/cooks",
            json!({"cooked_on":"2026-10-24"}),
        ),
        ("DELETE", "/api/recipes/1/cooks/1", Value::Null),
    ] {
        assert_eq!(
            send(&app, method, path, data).await.0,
            StatusCode::UNAUTHORIZED
        );
    }
}
