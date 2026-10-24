//! Stateless MCP Streamable HTTP: JSON responses, no SSE stream or session affinity.
use crate::{AppState, Error, Result, db, model::*};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};

fn schema(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}
fn number() -> Value {
    json!({"type":"number","exclusiveMinimum":0})
}
fn id() -> Value {
    json!({"type":"integer","minimum":1})
}
fn string() -> Value {
    json!({"type":"string"})
}
fn ingredient() -> Value {
    schema(
        json!({"name":string(),"quantity":number(),"unit":string()}),
        &["name", "quantity"],
    )
}
fn recipe_schema() -> Value {
    schema(
        json!({"title":string(),"servings":number(),"instructions":string(),"source_url":string(),"ingredients":{"type":"array","items":ingredient()}}),
        &["title", "servings", "ingredients"],
    )
}
fn cook_schema(update: bool) -> Value {
    let mut properties = json!({"recipe_id":id(),"cooked_on":string(),"rating":{"type":["integer","null"],"minimum":1,"maximum":5},"cook_name":string(),"went_well":string(),"to_improve":string(),"notes":string()});
    if update {
        properties["cook_id"] = id();
    }
    schema(
        properties,
        if update {
            &["recipe_id", "cook_id", "cooked_on"]
        } else {
            &["recipe_id", "cooked_on"]
        },
    )
}
fn tools() -> Value {
    let tool = |name: &str, description: &str, input: Value, read: bool| json!({"name":name,"description":description,"inputSchema":input,"annotations":{"readOnlyHint":read,"destructiveHint":!read,"openWorldHint":false}});
    json!({"tools":[
        tool("list_inventory","Read approximate pantry quantities. Missing entries are unknown. Names and units match case-insensitively; use recipe ingredient names and units.",schema(json!({}),&[]),true),
        tool("set_inventory","Set an absolute quantity (zero means out). Correct counts after cooking or untracked shopping. Does not record a price.",schema(json!({"name":string(),"unit":string(),"quantity":{"type":"number","minimum":0}}),&["name","unit","quantity"]),false),
        tool("import_receipt","Import structured receipt lines into inventory and historical prices atomically. Reuse the same stable receipt_id for retries; changed contents with that ID are rejected. Read recipes/inventory first to map receipt labels to ingredient names. Convert pack contents into recipe units only when known; do not guess weights or prices. total_cost is the line total after discounts, null if unknown. Only include food to track. currency is a three-letter code; purchased_on is YYYY-MM-DD.",schema(json!({"receipt_id":string(),"purchased_on":string(),"currency":string(),"items":{"type":"array","items":schema(json!({"name":string(),"quantity":number(),"unit":string(),"total_cost":{"type":["number","null"],"minimum":0}}),&["name","quantity","unit","total_cost"])}}),&["receipt_id","purchased_on","currency","items"]),false),
        tool("list_purchase_history","Read historical receipt prices, dates and quantities.",schema(json!({}),&[]),true),
        tool("estimate_recipe_cost","Estimate recipe and per-serving costs from latest matching ingredient/unit purchase prices. Missing prices are unknown; totals are partial and separated by currency.",schema(json!({"recipe_id":id()}),&["recipe_id"]),true),
        tool("list_recipes","List saved recipes, including IDs and ingredients.",schema(json!({}),&[]),true),
        tool("get_recipe","Get a saved recipe.",schema(json!({"recipe_id":id()}),&["recipe_id"]),true),
        tool("create_recipe","Save a recipe with structured ingredient quantities and units.",recipe_schema(),false),
        tool("rate_recipe","Set the shared recipe rating to 1–5, or null to clear. This does not create a cook log.",schema(json!({"recipe_id":id(),"rating":{"type":["integer","null"],"minimum":1,"maximum":5}}),&["recipe_id","rating"]),false),
        tool("list_cooks","Read cook history for a recipe, newest date first, including per-cook ratings and notes.",schema(json!({"recipe_id":id()}),&["recipe_id"]),true),
        tool("log_cook","Record one time a recipe was cooked with date, optional 1–5 rating, cook name and notes. Each call creates another cook; inspect history before retrying.",cook_schema(false),false),
        tool("update_cook","Replace a cook log. Supply its date and every notes field you want to keep; omitted optional fields clear.",cook_schema(true),false),
        tool("delete_cook","Delete a cook log for a recipe. Recipe counts and averages recalculate automatically.",schema(json!({"recipe_id":id(),"cook_id":id()}),&["recipe_id","cook_id"]),false),
        tool("list_weekly_wishlist","List recipes wished for a week; any YYYY-MM-DD date identifies its Monday-to-Sunday week.",schema(json!({"week":string()}),&["week"]),true),
        tool("add_to_weekly_wishlist","Add a recipe to a weekly wishlist without choosing a day or meal slot.",schema(json!({"recipe_id":id(),"week":string(),"servings":number()}),&["recipe_id","week","servings"]),false),
        tool("mark_wishlist_cooked","Mark a weekly wishlist entry as cooked or undo that mark.",schema(json!({"wish_id":id(),"cooked":{"type":"boolean"}}),&["wish_id","cooked"]),false),
        tool("add_week_to_shopping_list","Add all uncooked wishlist meals' scaled ingredients for a week, subtracting known stock once per list. Additive: do not retry blindly.",schema(json!({"list_id":id(),"week":string()}),&["list_id","week"]),false),
        tool("list_meals","List dated meal plan entries.",schema(json!({}),&[]),true),
        tool("plan_meal","Schedule a saved recipe for YYYY-MM-DD. Slot: breakfast, lunch, dinner or snack.",schema(json!({"recipe_id":id(),"date":string(),"servings":number(),"slot":string()}),&["recipe_id","date","servings"]),false),
        tool("list_shopping_lists","List shopping lists and their IDs.",schema(json!({}),&[]),true),
        tool("create_shopping_list","Create a named shopping list.",schema(json!({"name":string()}),&["name"]),false),
        tool("get_shopping_list","Read a shopping list and checked state of its items.",schema(json!({"list_id":id()}),&["list_id"]),true),
        tool("add_recipe_to_shopping_list","Add scaled ingredients to a list, subtracting known inventory once per list. Unknown names/units are included in full. Pantry counts are not consumed. Matching names and units merge into unchecked items. Each call adds quantities again; do not retry without checking the list.",schema(json!({"list_id":id(),"recipe_id":id(),"servings":number()}),&["list_id","recipe_id","servings"]),false),
        tool("add_shopping_item","Add an item to a shopping list; merge matching unchecked names and units.",schema(json!({"list_id":id(),"name":string(),"quantity":number(),"unit":string()}),&["list_id","name","quantity"]),false),
        tool("check_shopping_item","Set an item's checked state.",schema(json!({"list_id":id(),"item_id":id(),"checked":{"type":"boolean"}}),&["list_id","item_id","checked"]),false)
    ]})
}
fn int(v: &Value, key: &str) -> Result<i64> {
    v[key]
        .as_i64()
        .filter(|n| *n > 0)
        .ok_or_else(|| Error::bad(format!("{key} must be a positive integer")))
}
fn num(v: &Value, key: &str) -> Result<f64> {
    v[key]
        .as_f64()
        .ok_or_else(|| Error::bad(format!("{key} must be a number")))
}
fn decode<T: serde::de::DeserializeOwned>(v: Value) -> Result<T> {
    serde_json::from_value(v).map_err(|e| Error::bad(e.to_string()))
}
fn call(db: &mut rusqlite::Connection, name: &str, args: Value) -> Result<Value> {
    Ok(match name {
        "list_inventory" => json!(crate::inventory::items(db)?),
        "set_inventory" => json!(crate::inventory::set(db, decode(args)?)?),
        "import_receipt" => crate::inventory::import(db, decode(args)?)?,
        "list_purchase_history" => json!(crate::inventory::history(db)?),
        "estimate_recipe_cost" => crate::inventory::cost(db, int(&args, "recipe_id")?)?,
        "list_recipes" => json!(db::recipes(db)?),
        "get_recipe" => json!(db::recipe(db, int(&args, "recipe_id")?)?),
        "create_recipe" => json!(db::save_recipe(db, None, decode(args)?)?),
        "rate_recipe" => json!(db::rate_recipe(
            db,
            int(&args, "recipe_id")?,
            decode::<RatingInput>(args)?.rating
        )?),
        "list_cooks" => json!(db::cooks(db, int(&args, "recipe_id")?)?),
        "log_cook" => json!(db::save_cook(
            db,
            int(&args, "recipe_id")?,
            None,
            decode(args)?
        )?),
        "update_cook" => json!(db::save_cook(
            db,
            int(&args, "recipe_id")?,
            Some(int(&args, "cook_id")?),
            decode(args)?
        )?),
        "delete_cook" => {
            db::delete_cook(db, int(&args, "recipe_id")?, int(&args, "cook_id")?)?;
            json!({"ok":true})
        }
        "list_weekly_wishlist" => json!(db::wishes(
            db,
            args["week"]
                .as_str()
                .ok_or_else(|| Error::bad("week is required"))?
        )?),
        "add_to_weekly_wishlist" => json!(db::add_wish(db, decode(args)?)?),
        "mark_wishlist_cooked" => {
            db::cook_wish(
                db,
                int(&args, "wish_id")?,
                args["cooked"]
                    .as_bool()
                    .ok_or_else(|| Error::bad("cooked must be boolean"))?,
            )?;
            json!({"ok":true})
        }
        "add_week_to_shopping_list" => json!(db::add_week(
            db,
            int(&args, "list_id")?,
            args["week"]
                .as_str()
                .ok_or_else(|| Error::bad("week is required"))?
        )?),
        "list_meals" => json!(db::meals(db)?),
        "plan_meal" => json!(db::add_meal(db, decode::<MealInput>(args)?)?),
        "list_shopping_lists" => json!(db::lists(db)?),
        "create_shopping_list" => json!(db::create_list(db, decode::<ListInput>(args)?.name)?),
        "get_shopping_list" => json!(db::list(db, int(&args, "list_id")?)?),
        "add_recipe_to_shopping_list" => json!(db::add_recipe(
            db,
            int(&args, "list_id")?,
            int(&args, "recipe_id")?,
            num(&args, "servings")?
        )?),
        "add_shopping_item" => json!(db::add_item(db, int(&args, "list_id")?, decode(args)?)?),
        "check_shopping_item" => {
            let checked = args["checked"]
                .as_bool()
                .ok_or_else(|| Error::bad("checked must be boolean"))?;
            db::check_item(db, int(&args, "list_id")?, int(&args, "item_id")?, checked)?;
            json!({"ok":true})
        }
        _ => return Err(Error::bad("Unknown tool")),
    })
}
fn error(id: Value, code: i64, message: &str) -> Response {
    Json(json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})).into_response()
}
pub async fn handle(
    State(s): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    let request: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => return error(Value::Null, -32700, "Parse error"),
    };
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    if !request.is_object()
        || request["jsonrpc"] != "2.0"
        || !request["method"].is_string()
        || (request.get("id").is_some() && !(id.is_null() || id.is_string() || id.is_number()))
    {
        return error(Value::Null, -32600, "Invalid request");
    }
    if let Some(version) = headers.get("mcp-protocol-version")
        && !matches!(
            version.to_str().unwrap_or(""),
            "2025-03-26" | "2025-06-18" | "2025-11-25"
        )
    {
        return (StatusCode::BAD_REQUEST, "Unsupported MCP protocol version").into_response();
    }
    if request.get("id").is_none() {
        return StatusCode::ACCEPTED.into_response();
    }
    let result = match request["method"].as_str().unwrap_or("") {
        "initialize" => {
            let requested = request["params"]["protocolVersion"].as_str().unwrap_or("");
            let version = if matches!(requested, "2025-03-26" | "2025-06-18" | "2025-11-25") {
                requested
            } else {
                "2025-03-26"
            };
            json!({"protocolVersion":version,"capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":"recetaco","version":env!("CARGO_PKG_VERSION")},"instructions":"Use recipe and list IDs from read tools. Adding ingredients is additive, so inspect state before retrying a write."})
        }
        "ping" => json!({}),
        "tools/list" => tools(),
        "tools/call" => {
            let Some(name) = request["params"]["name"].as_str() else {
                return error(id, -32602, "Tool name is required");
            };
            if !tools()["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t["name"] == name)
            {
                return error(id, -32602, "Unknown tool");
            }
            let args = request["params"]
                .get("arguments")
                .cloned()
                .unwrap_or(json!({}));
            if !args.is_object() {
                return error(id, -32602, "Arguments must be an object");
            }
            let name = name.to_owned();
            match s.store.run(move |db| call(db, &name, args)).await {
                Ok(v) => {
                    json!({"content":[{"type":"text","text":v.to_string()}],"structuredContent":{"result":v},"isError":false})
                }
                Err(e) => json!({"content":[{"type":"text","text":e.message}],"isError":true}),
            }
        }
        _ => return error(id, -32601, "Method not found"),
    };
    Json(json!({"jsonrpc":"2.0","id":id,"result":result})).into_response()
}
