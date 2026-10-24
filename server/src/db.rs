use crate::{Error, Result, model::*};
use rusqlite::{Connection, OptionalExtension, params};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Store(Arc<Mutex<Connection>>);
impl Store {
    pub fn open(path: &str) -> Result<Self> {
        let db = Connection::open(path)?;
        db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA cache_size=-2048;
            CREATE TABLE IF NOT EXISTS recipes(id INTEGER PRIMARY KEY, data TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS meals(id INTEGER PRIMARY KEY, recipe_id INTEGER NOT NULL REFERENCES recipes(id) ON DELETE CASCADE, date TEXT NOT NULL, servings REAL NOT NULL, slot TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS meals_date ON meals(date);
            CREATE TABLE IF NOT EXISTS lists(id INTEGER PRIMARY KEY, name TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS items(id INTEGER PRIMARY KEY, list_id INTEGER NOT NULL REFERENCES lists(id) ON DELETE CASCADE, name TEXT NOT NULL, quantity REAL NOT NULL, unit TEXT NOT NULL, checked INTEGER NOT NULL DEFAULT 0);
            CREATE INDEX IF NOT EXISTS items_list ON items(list_id);
            CREATE TABLE IF NOT EXISTS photos(recipe_id INTEGER PRIMARY KEY REFERENCES recipes(id) ON DELETE CASCADE, jpeg BLOB NOT NULL, version INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS wishes(id INTEGER PRIMARY KEY, recipe_id INTEGER NOT NULL REFERENCES recipes(id) ON DELETE CASCADE, week TEXT NOT NULL, servings REAL NOT NULL, cooked INTEGER NOT NULL DEFAULT 0);
            CREATE INDEX IF NOT EXISTS wishes_week ON wishes(week);
            CREATE TABLE IF NOT EXISTS recipe_ratings(recipe_id INTEGER PRIMARY KEY REFERENCES recipes(id) ON DELETE CASCADE, rating INTEGER NOT NULL CHECK(rating BETWEEN 1 AND 5));
            CREATE TABLE IF NOT EXISTS cooks(id INTEGER PRIMARY KEY, recipe_id INTEGER NOT NULL REFERENCES recipes(id) ON DELETE CASCADE, cooked_on TEXT NOT NULL, rating INTEGER CHECK(rating BETWEEN 1 AND 5), cook_name TEXT NOT NULL, went_well TEXT NOT NULL, to_improve TEXT NOT NULL, notes TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS cooks_recipe_date ON cooks(recipe_id,cooked_on DESC,id DESC);
            PRAGMA user_version=3;")?;
        Ok(Self(Arc::new(Mutex::new(db))))
    }
    pub async fn run<T, F>(&self, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    {
        let db = self.0.clone();
        tokio::task::spawn_blocking(move || f(&mut *db.lock().map_err(|_| Error::internal())?))
            .await
            .map_err(|_| Error::internal())?
    }
}
fn text(value: &str, label: &str, max: usize) -> Result<()> {
    if value.trim().is_empty() || value.len() > max {
        return Err(Error::bad(format!("{label} must contain 1–{max} bytes")));
    }
    Ok(())
}
fn positive(value: f64, label: &str) -> Result<()> {
    if !value.is_finite() || value <= 0.0 || value > 1_000_000.0 {
        return Err(Error::bad(format!(
            "{label} must be greater than zero and at most 1,000,000"
        )));
    }
    Ok(())
}
pub fn validate_ingredient(i: &mut Ingredient) -> Result<()> {
    i.name = i.name.trim().to_string();
    i.unit = i.unit.trim().to_string();
    text(&i.name, "Ingredient name", 200)?;
    positive(i.quantity, "Quantity")?;
    if i.unit.len() > 80 {
        return Err(Error::bad("Unit is too long"));
    }
    Ok(())
}
pub fn recipes(db: &Connection) -> Result<Vec<Recipe>> {
    let mut stmt = db.prepare(
        "SELECT id,data,COALESCE((SELECT version FROM photos WHERE recipe_id=recipes.id),0), (SELECT rating FROM recipe_ratings WHERE recipe_id=recipes.id), (SELECT COUNT(*) FROM cooks WHERE recipe_id=recipes.id), (SELECT AVG(rating) FROM cooks WHERE recipe_id=recipes.id), (SELECT MAX(cooked_on) FROM cooks WHERE recipe_id=recipes.id) FROM recipes ORDER BY json_extract(data,'$.title') COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            RecipeStats {
                rating: row.get(3)?,
                cook_count: row.get(4)?,
                average_cook_rating: row.get(5)?,
                last_cooked: row.get(6)?,
            },
        ))
    })?;
    rows.map(|r| {
        let (id, data, photo_version, stats) = r?;
        Ok(Recipe {
            id,
            photo_version,
            stats,
            data: serde_json::from_str(&data)?,
        })
    })
    .collect()
}
pub fn recipe(db: &Connection, id: i64) -> Result<Recipe> {
    let data: String = db
        .query_row("SELECT data FROM recipes WHERE id=?", [id], |r| r.get(0))
        .optional()?
        .ok_or_else(Error::not_found)?;
    let photo_version = photo_version(db, id)?;
    Ok(Recipe {
        id,
        photo_version,
        stats: recipe_stats(db, id)?,
        data: serde_json::from_str(&data)?,
    })
}
pub fn save_recipe(db: &Connection, id: Option<i64>, mut data: RecipeInput) -> Result<Recipe> {
    data.title = data.title.trim().into();
    text(&data.title, "Recipe title", 200)?;
    positive(data.servings, "Servings")?;
    if data.ingredients.is_empty() || data.ingredients.len() > 200 {
        return Err(Error::bad("A recipe needs 1–200 ingredients"));
    }
    if data.instructions.len() > 100_000 || data.source_url.len() > 2000 {
        return Err(Error::bad("Recipe text is too long"));
    }
    if !data.source_url.is_empty()
        && !(data.source_url.starts_with("https://") || data.source_url.starts_with("http://"))
    {
        return Err(Error::bad("Source URL must use http or https"));
    }
    for i in &mut data.ingredients {
        validate_ingredient(i)?;
    }
    let json = serde_json::to_string(&data)?;
    let id = if let Some(id) = id {
        if db.execute("UPDATE recipes SET data=? WHERE id=?", params![json, id])? == 0 {
            return Err(Error::not_found());
        }
        id
    } else {
        db.execute("INSERT INTO recipes(data) VALUES(?)", [json])?;
        db.last_insert_rowid()
    };
    Ok(Recipe {
        id,
        photo_version: photo_version(db, id)?,
        stats: recipe_stats(db, id)?,
        data,
    })
}
pub fn delete_recipe(db: &Connection, id: i64) -> Result<()> {
    if db.execute("DELETE FROM recipes WHERE id=?", [id])? == 0 {
        return Err(Error::not_found());
    }
    Ok(())
}
pub fn meals(db: &Connection) -> Result<Vec<Meal>> {
    let mut stmt = db.prepare("SELECT m.id,m.recipe_id,m.date,m.servings,m.slot,json_extract(r.data,'$.title') FROM meals m JOIN recipes r ON r.id=m.recipe_id ORDER BY m.date,m.slot,m.id")?;
    Ok(stmt
        .query_map([], |r| {
            Ok(Meal {
                id: r.get(0)?,
                title: r.get(5)?,
                data: MealInput {
                    recipe_id: r.get(1)?,
                    date: r.get(2)?,
                    servings: r.get(3)?,
                    slot: r.get(4)?,
                },
            })
        })?
        .collect::<std::result::Result<_, _>>()?)
}
pub fn add_meal(db: &Connection, data: MealInput) -> Result<Meal> {
    let date = chrono::NaiveDate::parse_from_str(&data.date, "%Y-%m-%d")
        .map_err(|_| Error::bad("Date must be YYYY-MM-DD"))?;
    if date.format("%Y-%m-%d").to_string() != data.date {
        return Err(Error::bad("Date must be YYYY-MM-DD"));
    }
    positive(data.servings, "Servings")?;
    if !["breakfast", "lunch", "dinner", "snack"].contains(&data.slot.as_str()) {
        return Err(Error::bad("Unknown meal slot"));
    }
    let recipe = recipe(db, data.recipe_id)?;
    db.execute(
        "INSERT INTO meals(recipe_id,date,servings,slot) VALUES(?,?,?,?)",
        params![data.recipe_id, data.date, data.servings, data.slot],
    )?;
    Ok(Meal {
        id: db.last_insert_rowid(),
        title: recipe.data.title,
        data,
    })
}
pub fn delete_meal(db: &Connection, id: i64) -> Result<()> {
    if db.execute("DELETE FROM meals WHERE id=?", [id])? == 0 {
        return Err(Error::not_found());
    }
    Ok(())
}
pub fn lists(db: &Connection) -> Result<Vec<ShoppingList>> {
    let mut stmt = db.prepare("SELECT id,name FROM lists ORDER BY id")?;
    Ok(stmt
        .query_map([], |r| {
            Ok(ShoppingList {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?)
}
pub fn create_list(db: &Connection, name: String) -> Result<ShoppingList> {
    let name = name.trim().to_string();
    text(&name, "List name", 200)?;
    db.execute("INSERT INTO lists(name) VALUES(?)", [&name])?;
    Ok(ShoppingList {
        id: db.last_insert_rowid(),
        name,
    })
}
pub fn list(db: &Connection, id: i64) -> Result<ListDetail> {
    let name = db
        .query_row("SELECT name FROM lists WHERE id=?", [id], |r| r.get(0))
        .optional()?
        .ok_or_else(Error::not_found)?;
    let mut stmt = db.prepare("SELECT id,list_id,name,quantity,unit,checked FROM items WHERE list_id=? ORDER BY checked,id")?;
    let items = stmt
        .query_map([id], |r| {
            Ok(ShoppingItem {
                id: r.get(0)?,
                list_id: r.get(1)?,
                name: r.get(2)?,
                quantity: r.get(3)?,
                unit: r.get(4)?,
                checked: r.get(5)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?;
    Ok(ListDetail {
        list: ShoppingList { id, name },
        items,
    })
}
pub fn delete_list(db: &Connection, id: i64) -> Result<()> {
    if db.execute("DELETE FROM lists WHERE id=?", [id])? == 0 {
        return Err(Error::not_found());
    }
    Ok(())
}
fn merge_item(db: &Connection, list_id: i64, i: &Ingredient) -> Result<()> {
    // Deliberately conservative: no guessed unit conversions or pantry deductions.
    let existing: Option<(i64,f64)> = db.query_row("SELECT id,quantity FROM items WHERE list_id=? AND checked=0 AND lower(trim(name))=lower(trim(?)) AND lower(trim(unit))=lower(trim(?)) ORDER BY id LIMIT 1",params![list_id,i.name,i.unit],|r| Ok((r.get(0)?,r.get(1)?))).optional()?;
    if let Some((id, qty)) = existing {
        let total = qty + i.quantity;
        positive(total, "Combined quantity")?;
        db.execute("UPDATE items SET quantity=? WHERE id=?", params![total, id])?;
    } else {
        db.execute(
            "INSERT INTO items(list_id,name,quantity,unit) VALUES(?,?,?,?)",
            params![list_id, i.name, i.quantity, i.unit],
        )?;
    }
    Ok(())
}
pub fn add_item(db: &mut Connection, list_id: i64, mut i: Ingredient) -> Result<ListDetail> {
    validate_ingredient(&mut i)?;
    let tx = db.transaction()?;
    list(&tx, list_id)?;
    merge_item(&tx, list_id, &i)?;
    tx.commit()?;
    list(db, list_id)
}
pub fn add_recipe(
    db: &mut Connection,
    list_id: i64,
    recipe_id: i64,
    servings: f64,
) -> Result<ListDetail> {
    positive(servings, "Servings")?;
    let tx = db.transaction()?;
    list(&tx, list_id)?;
    let r = recipe(&tx, recipe_id)?;
    for mut i in r.data.ingredients {
        i.quantity *= servings / r.data.servings;
        validate_ingredient(&mut i)?;
        merge_item(&tx, list_id, &i)?;
    }
    tx.commit()?;
    list(db, list_id)
}
pub fn check_item(db: &Connection, list_id: i64, item_id: i64, checked: bool) -> Result<()> {
    if db.execute(
        "UPDATE items SET checked=? WHERE id=? AND list_id=?",
        params![checked, item_id, list_id],
    )? == 0
    {
        return Err(Error::not_found());
    }
    Ok(())
}
pub fn delete_item(db: &Connection, list_id: i64, item_id: i64) -> Result<()> {
    if db.execute(
        "DELETE FROM items WHERE id=? AND list_id=?",
        params![item_id, list_id],
    )? == 0
    {
        return Err(Error::not_found());
    }
    Ok(())
}

pub fn photo_version(db: &Connection, id: i64) -> Result<i64> {
    Ok(db
        .query_row("SELECT version FROM photos WHERE recipe_id=?", [id], |r| {
            r.get(0)
        })
        .optional()?
        .unwrap_or(0))
}
pub fn photo(db: &Connection, id: i64) -> Result<Vec<u8>> {
    db.query_row("SELECT jpeg FROM photos WHERE recipe_id=?", [id], |r| {
        r.get(0)
    })
    .optional()?
    .ok_or_else(Error::not_found)
}
pub fn save_photo(db: &Connection, id: i64, jpeg: Vec<u8>) -> Result<Recipe> {
    recipe(db, id)?;
    db.execute("INSERT INTO photos(recipe_id,jpeg,version) VALUES(?,?,1) ON CONFLICT(recipe_id) DO UPDATE SET jpeg=excluded.jpeg,version=photos.version+1",params![id,jpeg])?;
    recipe(db, id)
}
pub fn delete_photo(db: &Connection, id: i64) -> Result<()> {
    recipe(db, id)?;
    db.execute("DELETE FROM photos WHERE recipe_id=?", [id])?;
    Ok(())
}
/// Any day can identify a week; storage always uses the Monday date.
pub fn week_start(raw: &str) -> Result<String> {
    use chrono::Datelike;
    let day = chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .map_err(|_| Error::bad("Week must be a YYYY-MM-DD date"))?;
    if day.format("%Y-%m-%d").to_string() != raw {
        return Err(Error::bad("Week must be YYYY-MM-DD"));
    }
    let monday = day
        .checked_sub_signed(chrono::Duration::days(
            day.weekday().num_days_from_monday().into(),
        ))
        .ok_or_else(|| Error::bad("Week is out of range"))?;
    Ok(monday.format("%Y-%m-%d").to_string())
}
pub fn wishes(db: &Connection, week: &str) -> Result<Vec<Wish>> {
    let week = week_start(week)?;
    let mut stmt=db.prepare("SELECT w.id,w.recipe_id,w.week,w.servings,w.cooked,json_extract(r.data,'$.title') FROM wishes w JOIN recipes r ON r.id=w.recipe_id WHERE w.week=? ORDER BY w.cooked,w.id")?;
    Ok(stmt
        .query_map([week], |r| {
            Ok(Wish {
                id: r.get(0)?,
                title: r.get(5)?,
                cooked: r.get(4)?,
                data: WishInput {
                    recipe_id: r.get(1)?,
                    week: r.get(2)?,
                    servings: r.get(3)?,
                },
            })
        })?
        .collect::<std::result::Result<_, _>>()?)
}
pub fn add_wish(db: &Connection, mut data: WishInput) -> Result<Wish> {
    data.week = week_start(&data.week)?;
    positive(data.servings, "Servings")?;
    let r = recipe(db, data.recipe_id)?;
    db.execute(
        "INSERT INTO wishes(recipe_id,week,servings) VALUES(?,?,?)",
        params![data.recipe_id, data.week, data.servings],
    )?;
    Ok(Wish {
        id: db.last_insert_rowid(),
        title: r.data.title,
        cooked: false,
        data,
    })
}
pub fn cook_wish(db: &Connection, id: i64, cooked: bool) -> Result<()> {
    if db.execute("UPDATE wishes SET cooked=? WHERE id=?", params![cooked, id])? == 0 {
        return Err(Error::not_found());
    }
    Ok(())
}
pub fn delete_wish(db: &Connection, id: i64) -> Result<()> {
    if db.execute("DELETE FROM wishes WHERE id=?", [id])? == 0 {
        return Err(Error::not_found());
    }
    Ok(())
}
pub fn add_week(db: &mut Connection, list_id: i64, week: &str) -> Result<ListDetail> {
    let tx = db.transaction()?;
    list(&tx, list_id)?;
    for wish in wishes(&tx, week)?.into_iter().filter(|w| !w.cooked) {
        let r = recipe(&tx, wish.data.recipe_id)?;
        for mut i in r.data.ingredients {
            i.quantity *= wish.data.servings / r.data.servings;
            validate_ingredient(&mut i)?;
            merge_item(&tx, list_id, &i)?;
        }
    }
    tx.commit()?;
    list(db, list_id)
}
pub fn all_wishes(db: &Connection) -> Result<Vec<Wish>> {
    let weeks = db
        .prepare("SELECT DISTINCT week FROM wishes ORDER BY week")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut out = Vec::new();
    for week in weeks {
        out.extend(wishes(db, &week)?);
    }
    Ok(out)
}

fn validate_rating(rating: Option<i64>) -> Result<()> {
    if rating.is_some_and(|r| !(1..=5).contains(&r)) {
        return Err(Error::bad("Rating must be an integer from 1 to 5, or null"));
    }
    Ok(())
}
pub fn recipe_stats(db: &Connection, id: i64) -> Result<RecipeStats> {
    Ok(db.query_row("SELECT (SELECT rating FROM recipe_ratings WHERE recipe_id=?1),COUNT(*),AVG(rating),MAX(cooked_on) FROM cooks WHERE recipe_id=?1",[id],|r|Ok(RecipeStats {rating:r.get(0)?,cook_count:r.get(1)?,average_cook_rating:r.get(2)?,last_cooked:r.get(3)?}))?)
}
pub fn rate_recipe(db: &Connection, id: i64, rating: Option<i64>) -> Result<Recipe> {
    validate_rating(rating)?;
    recipe(db, id)?;
    match rating {
        Some(rating) => {
            db.execute("INSERT INTO recipe_ratings(recipe_id,rating) VALUES(?,?) ON CONFLICT(recipe_id) DO UPDATE SET rating=excluded.rating",params![id,rating])?;
        }
        None => {
            db.execute("DELETE FROM recipe_ratings WHERE recipe_id=?", [id])?;
        }
    }
    recipe(db, id)
}
fn cook_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Cook> {
    Ok(Cook {
        id: r.get(0)?,
        recipe_id: r.get(1)?,
        data: CookInput {
            cooked_on: r.get(2)?,
            rating: r.get(3)?,
            cook_name: r.get(4)?,
            went_well: r.get(5)?,
            to_improve: r.get(6)?,
            notes: r.get(7)?,
        },
    })
}
pub fn cooks(db: &Connection, recipe_id: i64) -> Result<Vec<Cook>> {
    recipe(db, recipe_id)?;
    let mut stmt=db.prepare("SELECT id,recipe_id,cooked_on,rating,cook_name,went_well,to_improve,notes FROM cooks WHERE recipe_id=? ORDER BY cooked_on DESC,id DESC")?;
    Ok(stmt
        .query_map([recipe_id], cook_row)?
        .collect::<std::result::Result<_, _>>()?)
}
pub fn save_cook(
    db: &Connection,
    recipe_id: i64,
    id: Option<i64>,
    mut data: CookInput,
) -> Result<Cook> {
    recipe(db, recipe_id)?;
    validate_rating(data.rating)?;
    let date = chrono::NaiveDate::parse_from_str(&data.cooked_on, "%Y-%m-%d")
        .map_err(|_| Error::bad("Cook date must be YYYY-MM-DD"))?;
    if date.format("%Y-%m-%d").to_string() != data.cooked_on {
        return Err(Error::bad("Cook date must be YYYY-MM-DD"));
    }
    data.cook_name = data.cook_name.trim().into();
    data.went_well = data.went_well.trim().into();
    data.to_improve = data.to_improve.trim().into();
    data.notes = data.notes.trim().into();
    if data.cook_name.len() > 100
        || data.went_well.len() > 10_000
        || data.to_improve.len() > 10_000
        || data.notes.len() > 10_000
    {
        return Err(Error::bad(
            "Cook name is limited to 100 bytes and each notes field to 10,000 bytes",
        ));
    }
    let id = if let Some(id) = id {
        if db.execute("UPDATE cooks SET cooked_on=?,rating=?,cook_name=?,went_well=?,to_improve=?,notes=? WHERE id=? AND recipe_id=?",params![data.cooked_on,data.rating,data.cook_name,data.went_well,data.to_improve,data.notes,id,recipe_id])?==0 {return Err(Error::not_found());}
        id
    } else {
        db.execute("INSERT INTO cooks(recipe_id,cooked_on,rating,cook_name,went_well,to_improve,notes) VALUES(?,?,?,?,?,?,?)",params![recipe_id,data.cooked_on,data.rating,data.cook_name,data.went_well,data.to_improve,data.notes])?;
        db.last_insert_rowid()
    };
    Ok(Cook {
        id,
        recipe_id,
        data,
    })
}
pub fn delete_cook(db: &Connection, recipe_id: i64, id: i64) -> Result<()> {
    if db.execute(
        "DELETE FROM cooks WHERE id=? AND recipe_id=?",
        params![id, recipe_id],
    )? == 0
    {
        return Err(Error::not_found());
    }
    Ok(())
}
pub fn all_cooks(db: &Connection) -> Result<Vec<Cook>> {
    let mut stmt=db.prepare("SELECT id,recipe_id,cooked_on,rating,cook_name,went_well,to_improve,notes FROM cooks ORDER BY cooked_on DESC,id DESC")?;
    Ok(stmt
        .query_map([], cook_row)?
        .collect::<std::result::Result<_, _>>()?)
}
