use recetaco::{
    db::{self, Store},
    inventory as inv,
    model::*,
};
use serde_json::json;
fn receipt(id: &str) -> inv::Receipt {
    serde_json::from_value(json!({"receipt_id":id,"purchased_on":"2026-10-24","currency":"eur","items":[{"name":"Rice","quantity":150,"unit":"g","total_cost":3}]})).unwrap()
}
fn recipe() -> RecipeInput {
    serde_json::from_value(json!({"title":"Rice","servings":2,"ingredients":[{"name":"Rice","quantity":100,"unit":"g"},{"name":"Salt","quantity":1,"unit":"g"}]})).unwrap()
}
#[tokio::test]
async fn receipt_retries_cost_coverage_and_allocations() {
    let store = Store::open(":memory:").unwrap();
    store
        .run(|db| {
            assert_eq!(inv::import(db, receipt("one"))?["imported"], true);
            assert_eq!(inv::import(db, receipt("one"))?["imported"], false);
            assert_eq!(inv::items(db)?[0].quantity, 150.0);
            let mut changed = receipt("one");
            changed.items[0].ingredient.quantity = 200.0;
            assert!(inv::import(db, changed).is_err());
            let r = db::save_recipe(db, None, recipe())?;
            let estimate = inv::cost(db, r.id)?;
            assert_eq!(estimate["totals"]["EUR"], 2.0);
            assert_eq!(estimate["per_serving"]["EUR"], 1.0);
            assert_eq!(estimate["complete"], false);
            let l = db::create_list(db, "Shop".into())?;
            let first = db::add_recipe(db, l.id, r.id, 2.0)?;
            assert_eq!(first.items.len(), 1);
            assert_eq!(first.items[0].name, "Salt");
            let second = db::add_recipe(db, l.id, r.id, 2.0)?;
            assert_eq!(
                second
                    .items
                    .iter()
                    .find(|i| i.name == "Rice")
                    .unwrap()
                    .quantity,
                50.0
            );
            assert_eq!(inv::items(db)?[0].quantity, 150.0);
            let manual = db::add_item(
                db,
                l.id,
                Ingredient {
                    name: "Rice".into(),
                    quantity: 10.0,
                    unit: "g".into(),
                },
            )?;
            assert_eq!(
                manual
                    .items
                    .iter()
                    .find(|i| i.name == "Rice")
                    .unwrap()
                    .quantity,
                60.0
            );
            let mut bad = receipt("bad");
            bad.items.push(inv::Purchase {
                ingredient: Ingredient {
                    name: "Rice".into(),
                    quantity: 1_000_000.0,
                    unit: "g".into(),
                },
                total_cost: None,
            });
            assert!(inv::import(db, bad).is_err());
            assert_eq!(inv::history(db)?.len(), 1);
            assert_eq!(inv::items(db)?[0].quantity, 150.0);
            inv::set(
                db,
                inv::Stock {
                    name: " RICE ".into(),
                    unit: "g".into(),
                    quantity: 0.0,
                    updated_at: String::new(),
                },
            )?;
            assert_eq!(inv::items(db)?[0].quantity, 0.0);
            assert_eq!(inv::history(db)?.len(), 1);
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn mcp_import_and_auth_routes() {
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let app = recetaco::app(
        recetaco::AppState::new(Store::open(":memory:").unwrap(), None),
        "/missing",
    );
    let response=app.clone().oneshot(Request::builder().method("POST").uri("/mcp").header("host","localhost").header("content-type","application/json").body(Body::from(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"import_receipt","arguments":receipt("mcp")}}).to_string())).unwrap()).await.unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(value["result"]["isError"], false);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/inventory")
                .header("host", "localhost")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(value[0]["quantity"], 150.0);
}
