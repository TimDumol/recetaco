use recetaco::{
    db::{self, Store},
    model::RecipeInput,
};
use serde_json::json;

#[tokio::test]
async fn cooking_steps_roundtrip_and_validation() {
    let store = Store::open(":memory:").unwrap();
    store.run(|db| {
        let input = json!({"title":"Pasta", "servings":2,"ingredients":[{"name":"Oil","quantity":20,"unit":"ml"}],"steps":[{"text":"Heat half the oil","ingredients":[{"ingredient":0,"quantity":10}],"after":[]},{"text":"Mix","ingredients":[{"ingredient":0,"quantity":10}],"after":[0]}]});
        let saved = db::save_recipe(db,None,serde_json::from_value(input.clone()).unwrap())?;
        let loaded = db::recipe(db,saved.id)?;
        assert_eq!(loaded.data.steps[1].after, vec![0]);
        assert_eq!(loaded.data.steps[0].ingredients[0].quantity, Some(10.0));
        for broken in [
            json!({"text":"Cycle","after":[0]}),
            json!({"text":"Invalid ingredient","ingredients":[{"ingredient":1}]}),
            json!({"text":"Too much","ingredients":[{"ingredient":0,"quantity":21}]}),
            json!({"text":"Zero","ingredients":[{"ingredient":0,"quantity":0}]}),
        ] {
            let mut bad = input.clone(); bad["steps"] = json!([broken]);
            assert!(db::save_recipe(db,Some(saved.id),serde_json::from_value(bad).unwrap()).is_err());
        }
        assert_eq!(db::recipe(db,saved.id)?.data.steps.len(),2);
        let mut legacy = input; legacy.as_object_mut().unwrap().remove("steps");
        let legacy: RecipeInput = serde_json::from_value(legacy).unwrap();
        assert!(db::save_recipe(db,None,legacy)?.data.steps.is_empty());
        Ok(())
    }).await.unwrap();
}
