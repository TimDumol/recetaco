use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ingredient {
    pub name: String,
    pub quantity: f64,
    #[serde(default)]
    pub unit: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecipeInput {
    pub title: String,
    pub servings: f64,
    #[serde(default)]
    pub instructions: String,
    #[serde(default)]
    pub source_url: String,
    pub ingredients: Vec<Ingredient>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recipe {
    pub id: i64,
    #[serde(default)]
    pub photo_version: i64,
    #[serde(flatten)]
    pub stats: RecipeStats,
    #[serde(flatten)]
    pub data: RecipeInput,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MealInput {
    pub recipe_id: i64,
    pub date: String,
    pub servings: f64,
    #[serde(default = "dinner")]
    pub slot: String,
}
fn dinner() -> String {
    "dinner".into()
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Meal {
    pub id: i64,
    pub title: String,
    #[serde(flatten)]
    pub data: MealInput,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShoppingList {
    pub id: i64,
    pub name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShoppingItem {
    pub id: i64,
    pub list_id: i64,
    pub name: String,
    pub quantity: f64,
    pub unit: String,
    pub checked: bool,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ListDetail {
    #[serde(flatten)]
    pub list: ShoppingList,
    pub items: Vec<ShoppingItem>,
}
#[derive(Deserialize)]
pub struct ListInput {
    pub name: String,
}
#[derive(Deserialize)]
pub struct CheckInput {
    pub checked: bool,
}
#[derive(Deserialize)]
pub struct AddRecipeInput {
    pub servings: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WishInput {
    pub recipe_id: i64,
    pub week: String,
    pub servings: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Wish {
    pub id: i64,
    pub title: String,
    pub cooked: bool,
    #[serde(flatten)]
    pub data: WishInput,
}
#[derive(Deserialize)]
pub struct WishQuery {
    pub week: String,
}
#[derive(Deserialize)]
pub struct CookedInput {
    pub cooked: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RecipeStats {
    pub rating: Option<i64>,
    pub cook_count: i64,
    pub average_cook_rating: Option<f64>,
    pub last_cooked: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CookInput {
    pub cooked_on: String,
    #[serde(default)]
    pub rating: Option<i64>,
    #[serde(default)]
    pub cook_name: String,
    #[serde(default)]
    pub went_well: String,
    #[serde(default)]
    pub to_improve: String,
    #[serde(default)]
    pub notes: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cook {
    pub id: i64,
    pub recipe_id: i64,
    #[serde(flatten)]
    pub data: CookInput,
}
#[derive(Deserialize)]
pub struct RatingInput {
    pub rating: Option<i64>,
}
