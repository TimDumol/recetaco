use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use recetaco::{AppState, app, db::Store};
use serde_json::{Value, json};
use tower::ServiceExt;
fn test_app(token: Option<&str>) -> Router {
    app(
        AppState::new(Store::open(":memory:").unwrap(), token.map(str::to_owned)),
        "/nonexistent",
    )
}
async fn send(app: &Router, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}
fn recipe() -> Value {
    json!({"title":"Rice bowl","servings":2,"ingredients":[{"name":"Rice","quantity":100,"unit":"g"},{"name":"Lemon","quantity":1,"unit":""}],"instructions":"Cook rice."})
}
#[tokio::test]
async fn recipe_plan_shop_and_scale() {
    let app = test_app(None);
    let (s, r) = send(&app, "POST", "/api/recipes", recipe()).await;
    assert_eq!(s, StatusCode::CREATED);
    let rid = r["id"].as_i64().unwrap();
    let (s, m) = send(
        &app,
        "POST",
        "/api/meals",
        json!({"recipe_id":rid,"date":"2026-10-25","servings":4,"slot":"dinner"}),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(m["title"], "Rice bowl");
    let (_, l) = send(&app, "POST", "/api/lists", json!({"name":"Weekend"})).await;
    let lid = l["id"].as_i64().unwrap();
    let path = format!("/api/lists/{lid}/recipes/{rid}");
    let (s, l) = send(&app, "POST", &path, json!({"servings":4})).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(l["items"][0]["quantity"], 200.0);
    let (_, l) = send(
        &app,
        "POST",
        &format!("/api/lists/{lid}/items"),
        json!({"name":" rice ","quantity":50,"unit":"G"}),
    )
    .await;
    assert_eq!(l["items"].as_array().unwrap().len(), 2);
    assert_eq!(l["items"][0]["quantity"], 250.0);
    let item = l["items"][0]["id"].as_i64().unwrap();
    assert_eq!(
        send(
            &app,
            "PATCH",
            &format!("/api/lists/{lid}/items/{item}"),
            json!({"checked":true})
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    let (_, l) = send(&app, "POST", &path, json!({"servings":2})).await;
    assert_eq!(l["items"].as_array().unwrap().len(), 3);
    assert_eq!(
        l["items"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["checked"] == true)
            .count(),
        1
    );
    // Recipe deletion cascades meal plan, but retains the shopping list.
    assert_eq!(
        send(&app, "DELETE", &format!("/api/recipes/{rid}"), json!(null))
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        send(&app, "GET", "/api/meals", json!(null)).await.1,
        json!([])
    );
    assert_eq!(
        send(&app, "GET", &format!("/api/lists/{lid}"), json!(null))
            .await
            .1["items"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}
#[tokio::test]
async fn bad_input_and_missing_records() {
    let app = test_app(None);
    for servings in [0, -1, 1_000_001] {
        let mut r = recipe();
        r["servings"] = json!(servings);
        assert_eq!(
            send(&app, "POST", "/api/recipes", r).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    let mut r = recipe();
    r["ingredients"] = json!([]);
    assert_eq!(
        send(&app, "POST", "/api/recipes", r).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        send(&app, "GET", "/api/recipes/99", json!(null)).await.0,
        StatusCode::NOT_FOUND
    );
    let (_, r) = send(&app, "POST", "/api/recipes", recipe()).await;
    for date in ["2026-02-30", "2026-1-2", "nonsense"] {
        assert_eq!(
            send(
                &app,
                "POST",
                "/api/meals",
                json!({"recipe_id":r["id"],"servings":2,"date":date})
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    let (_, l) = send(&app, "POST", "/api/lists", json!({"name":"One"})).await;
    let (s, _) = send(
        &app,
        "POST",
        &format!("/api/lists/{}/items", l["id"]),
        json!({"name":"Rice","quantity":0}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}
#[tokio::test]
async fn atomic_recipe_addition_rolls_back_on_overflow() {
    let app = test_app(None);
    let (_,r)=send(&app,"POST","/api/recipes",json!({"title":"Big","servings":1,"ingredients":[{"name":"First","quantity":1},{"name":"Too much","quantity":1000000}]})).await;
    let (_, l) = send(&app, "POST", "/api/lists", json!({"name":"Atomic"})).await;
    let (s, _) = send(
        &app,
        "POST",
        &format!("/api/lists/{}/recipes/{}", l["id"], r["id"]),
        json!({"servings":2}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (_, l) = send(&app, "GET", &format!("/api/lists/{}", l["id"]), json!(null)).await;
    assert_eq!(l["items"], json!([]));
}
#[tokio::test]
async fn auth_and_browser_origin_are_enforced() {
    let app = test_app(Some("secret-token"));
    for path in ["/api/recipes", "/api/export", "/mcp"] {
        let method = if path == "/mcp" { "POST" } else { "GET" };
        assert_eq!(
            send(&app, method, path, json!({})).await.0,
            StatusCode::UNAUTHORIZED
        );
    }
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/recipes")
                .header("authorization", "Bearer secret-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/recipes")
                .header("host", "localhost:8080")
                .header("origin", "https://evil.example")
                .header("authorization", "Bearer secret-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        send(&app, "GET", "/health", Value::Null).await.0,
        StatusCode::OK
    );
}
#[tokio::test]
async fn mcp_initialize_discover_and_write() {
    let app = test_app(None);
    let (_,init)=send(&app,"POST","/mcp",json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","clientInfo":{"name":"test","version":"1"},"capabilities":{}}})).await;
    assert_eq!(init["result"]["serverInfo"]["name"], "recetaco");
    let (_, tools) = send(
        &app,
        "POST",
        "/mcp",
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
    )
    .await;
    assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 20);
    let (_,r)=send(&app,"POST","/mcp",json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"create_recipe","arguments":recipe()}})).await;
    assert_eq!(r["result"]["isError"], false);
    let (_, recipes) = send(&app, "GET", "/api/recipes", Value::Null).await;
    assert_eq!(recipes[0]["title"], "Rice bowl");
    let (_,r)=send(&app,"POST","/mcp",json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"create_shopping_list","arguments":{"name":""}}})).await;
    assert_eq!(r["result"]["isError"], true);
    let (_, r) = send(
        &app,
        "POST",
        "/mcp",
        json!({"jsonrpc":"2.0","id":5,"method":"unknown"}),
    )
    .await;
    assert_eq!(r["error"]["code"], -32601);
    assert_eq!(
        send(
            &app,
            "POST",
            "/mcp",
            json!({"jsonrpc":"2.0","method":"notifications/initialized"})
        )
        .await
        .0,
        StatusCode::ACCEPTED
    );
}
#[tokio::test]
async fn checked_items_and_other_lists_cannot_be_modified_accidentally() {
    let app = test_app(None);
    let (_, a) = send(&app, "POST", "/api/lists", json!({"name":"A"})).await;
    let (_, b) = send(&app, "POST", "/api/lists", json!({"name":"B"})).await;
    let (_, a) = send(
        &app,
        "POST",
        &format!("/api/lists/{}/items", a["id"]),
        json!({"name":"Milk","quantity":1,"unit":"L"}),
    )
    .await;
    let wrong = format!("/api/lists/{}/items/{}", b["id"], a["items"][0]["id"]);
    assert_eq!(
        send(&app, "PATCH", &wrong, json!({"checked":true})).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&app, "DELETE", &wrong, Value::Null).await.0,
        StatusCode::NOT_FOUND
    );
}
#[tokio::test]
async fn concurrent_additions_preserve_totals() {
    let app = test_app(None);
    let (_, l) = send(&app, "POST", "/api/lists", json!({"name":"Concurrent"})).await;
    let path = format!("/api/lists/{}/items", l["id"]);
    let mut handles = Vec::new();
    for _ in 0..12 {
        let app = app.clone();
        let path = path.clone();
        handles.push(tokio::spawn(async move {
            send(
                &app,
                "POST",
                &path,
                json!({"name":"Rice","quantity":1,"unit":"g"}),
            )
            .await
            .0
        }));
    }
    for h in handles {
        assert_eq!(h.await.unwrap(), StatusCode::OK);
    }
    let (_, l) = send(&app, "GET", &format!("/api/lists/{}", l["id"]), Value::Null).await;
    assert_eq!(l["items"].as_array().unwrap().len(), 1);
    assert_eq!(l["items"][0]["quantity"], 12.0);
}
