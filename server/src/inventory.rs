//! Inventory is approximate; unknown ingredients never imply absence.
use crate::{AppState, Error, Result, db, model::Ingredient};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Serialize, Deserialize)]
pub struct Stock {
    pub name: String,
    pub unit: String,
    pub quantity: f64,
    #[serde(default)]
    pub updated_at: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Purchase {
    #[serde(flatten)]
    pub ingredient: Ingredient,
    pub total_cost: Option<f64>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub receipt_id: String,
    pub purchased_on: String,
    pub currency: String,
    pub items: Vec<Purchase>,
}
fn key(s: &str) -> String {
    s.trim().to_lowercase()
}
pub fn items(db: &Connection) -> Result<Vec<Stock>> {
    Ok(db
        .prepare("SELECT name,unit,quantity,updated_at FROM inventory ORDER BY name,unit")?
        .query_map([], |r| {
            Ok(Stock {
                name: r.get(0)?,
                unit: r.get(1)?,
                quantity: r.get(2)?,
                updated_at: r.get(3)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?)
}
pub fn set(db: &Connection, mut stock: Stock) -> Result<Vec<Stock>> {
    stock.name = key(&stock.name);
    stock.unit = key(&stock.unit);
    if stock.name.is_empty()
        || stock.name.len() > 200
        || stock.unit.len() > 80
        || !stock.quantity.is_finite()
        || !(0.0..=1_000_000.0).contains(&stock.quantity)
    {
        return Err(Error::bad(
            "Supply a name, unit and quantity between 0 and 1,000,000",
        ));
    }
    db.execute("INSERT INTO inventory(name,unit,quantity) VALUES(?,?,?) ON CONFLICT(name,unit) DO UPDATE SET quantity=excluded.quantity,updated_at=CURRENT_TIMESTAMP",params![stock.name,stock.unit,stock.quantity])?;
    items(db)
}
pub fn remove(db: &Connection, stock: Stock) -> Result<Vec<Stock>> {
    db.execute(
        "DELETE FROM inventory WHERE name=? AND unit=?",
        params![key(&stock.name), key(&stock.unit)],
    )?;
    items(db)
}
pub fn receipts(db: &Connection) -> Result<Vec<Value>> {
    let rows = db
        .prepare("SELECT data FROM receipts ORDER BY id")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|s| Ok(serde_json::from_str(&s)?))
        .collect()
}
pub fn history(db: &Connection) -> Result<Vec<Value>> {
    Ok(db.prepare("SELECT id,receipt_id,name,unit,quantity,total_cost,currency,purchased_on FROM purchases ORDER BY purchased_on DESC,id DESC")?.query_map([],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"receipt_id":r.get::<_,String>(1)?,"name":r.get::<_,String>(2)?,"unit":r.get::<_,String>(3)?,"quantity":r.get::<_,f64>(4)?,"total_cost":r.get::<_,Option<f64>>(5)?,"currency":r.get::<_,String>(6)?,"purchased_on":r.get::<_,String>(7)?})))?.collect::<std::result::Result<_,_>>()?)
}
pub fn import(db: &mut Connection, mut receipt: Receipt) -> Result<Value> {
    if receipt.receipt_id.trim().is_empty()
        || receipt.receipt_id.len() > 200
        || receipt.items.is_empty()
        || receipt.items.len() > 500
    {
        return Err(Error::bad("Receipt needs a stable ID and 1–500 items"));
    }
    let date = chrono::NaiveDate::parse_from_str(&receipt.purchased_on, "%Y-%m-%d")
        .map_err(|_| Error::bad("Date must be YYYY-MM-DD"))?;
    if date.to_string() != receipt.purchased_on {
        return Err(Error::bad("Date must be YYYY-MM-DD"));
    }
    receipt.currency = receipt.currency.trim().to_uppercase();
    if receipt.currency.len() != 3 || !receipt.currency.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(Error::bad("Use a three-letter currency code"));
    }
    for p in &mut receipt.items {
        db::validate_ingredient(&mut p.ingredient)?;
        p.ingredient.name = key(&p.ingredient.name);
        p.ingredient.unit = key(&p.ingredient.unit);
        if p.total_cost
            .is_some_and(|c| !c.is_finite() || !(0.0..=1_000_000.0).contains(&c))
        {
            return Err(Error::bad("Invalid line total"));
        }
    }
    let data = serde_json::to_string(&receipt)?;
    let tx = db.transaction()?;
    if let Some(existing) = tx
        .query_row(
            "SELECT data FROM receipts WHERE id=?",
            [&receipt.receipt_id],
            |r| r.get::<_, String>(0),
        )
        .optional()?
    {
        if existing != data {
            return Err(Error::bad(
                "Receipt ID already imported with different contents",
            ));
        }
        return Ok(json!({"imported":false,"receipt_id":receipt.receipt_id}));
    }
    tx.execute(
        "INSERT INTO receipts(id,data) VALUES(?,?)",
        params![receipt.receipt_id, data],
    )?;
    for p in &receipt.items {
        let i = &p.ingredient;
        let qty = tx
            .query_row(
                "SELECT quantity FROM inventory WHERE name=? AND unit=?",
                params![i.name, i.unit],
                |r| r.get::<_, f64>(0),
            )
            .optional()?
            .unwrap_or(0.0);
        set(
            &tx,
            Stock {
                name: i.name.clone(),
                unit: i.unit.clone(),
                quantity: qty + i.quantity,
                updated_at: String::new(),
            },
        )?;
        tx.execute("INSERT INTO purchases(receipt_id,name,unit,quantity,total_cost,currency,purchased_on) VALUES(?,?,?,?,?,?,?)",params![receipt.receipt_id,i.name,i.unit,i.quantity,p.total_cost,receipt.currency,receipt.purchased_on])?;
    }
    tx.commit()?;
    Ok(json!({"imported":true,"receipt_id":receipt.receipt_id}))
}
// Allocate stock at most once per list. Shopping does not change pantry counts.
pub fn shopping_item(db: &Connection, list_id: i64, i: &Ingredient) -> Result<()> {
    let name = key(&i.name);
    let unit = key(&i.unit);
    let stock = db
        .query_row(
            "SELECT quantity FROM inventory WHERE name=? AND unit=?",
            params![name, unit],
            |r| r.get::<_, f64>(0),
        )
        .optional()?
        .unwrap_or(0.0);
    let used = db
        .query_row(
            "SELECT quantity FROM stock_used WHERE list_id=? AND name=? AND unit=?",
            params![list_id, name, unit],
            |r| r.get::<_, f64>(0),
        )
        .optional()?
        .unwrap_or(0.0);
    let covered = i.quantity.min((stock - used).max(0.0));
    db.execute("INSERT INTO stock_used(list_id,name,unit,quantity) VALUES(?,?,?,?) ON CONFLICT(list_id,name,unit) DO UPDATE SET quantity=quantity+excluded.quantity",params![list_id,name,unit,covered])?;
    if i.quantity - covered > 0.000001 {
        db::merge_item(
            db,
            list_id,
            &Ingredient {
                name: i.name.clone(),
                unit: i.unit.clone(),
                quantity: i.quantity - covered,
            },
        )?;
    }
    Ok(())
}
pub fn cost(db: &Connection, id: i64) -> Result<Value> {
    let recipe = db::recipe(db, id)?;
    let mut lines = Vec::new();
    let mut totals = std::collections::BTreeMap::<String, f64>::new();
    let mut priced = 0;
    for i in &recipe.data.ingredients {
        let price=db.query_row("SELECT total_cost/quantity,currency,purchased_on FROM purchases WHERE name=? AND unit=? AND total_cost IS NOT NULL ORDER BY purchased_on DESC,id DESC LIMIT 1",params![key(&i.name),key(&i.unit)],|r|Ok((r.get::<_,f64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).optional()?;
        if let Some((price, currency, date)) = price {
            let amount = price * i.quantity;
            *totals.entry(currency.clone()).or_default() += amount;
            priced += 1;
            lines.push(json!({"name":i.name,"cost":amount,"currency":currency,"price_date":date}));
        } else {
            lines.push(json!({"name":i.name,"cost":null}));
        }
    }
    let per_serving: std::collections::BTreeMap<_, _> = totals
        .iter()
        .map(|(k, v)| (k.clone(), v / recipe.data.servings))
        .collect();
    Ok(
        json!({"totals":totals,"per_serving":per_serving,"priced_ingredients":priced,"total_ingredients":lines.len(),"complete":priced==lines.len(),"ingredients":lines}),
    )
}
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/inventory", get(read).put(write).delete(delete))
        .route("/api/receipts", post(receipt))
        .route("/api/purchases", get(purchases))
        .route("/api/recipes/{id}/cost", get(estimate))
}
async fn read(State(s): State<AppState>) -> Result<Json<Vec<Stock>>> {
    Ok(Json(s.store.run(|db| items(db)).await?))
}
async fn write(State(s): State<AppState>, Json(v): Json<Stock>) -> Result<Json<Vec<Stock>>> {
    Ok(Json(s.store.run(move |db| set(db, v)).await?))
}
async fn delete(State(s): State<AppState>, Json(v): Json<Stock>) -> Result<Json<Vec<Stock>>> {
    Ok(Json(s.store.run(move |db| remove(db, v)).await?))
}
async fn receipt(State(s): State<AppState>, Json(v): Json<Receipt>) -> Result<Json<Value>> {
    Ok(Json(s.store.run(move |db| import(db, v)).await?))
}
async fn purchases(State(s): State<AppState>) -> Result<Json<Vec<Value>>> {
    Ok(Json(s.store.run(|db| history(db)).await?))
}
async fn estimate(State(s): State<AppState>, Path(id): Path<i64>) -> Result<Json<Value>> {
    Ok(Json(s.store.run(move |db| cost(db, id)).await?))
}
