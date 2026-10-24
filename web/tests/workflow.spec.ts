import { test, expect } from "@playwright/test";
test("create recipe, plan a meal, scale shopping ingredients, check off and persist", async ({
  page,
}, info) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  const name = `Lemon rice ${info.project.name}`;
  await page.goto("/");
  await page.getByRole("button", { name: "New recipe", exact: true }).click();
  await page.getByRole("textbox", { name: "Recipe name" }).fill(name);
  await page
    .getByRole("spinbutton", { name: "Servings", exact: true })
    .fill("2");
  await page.getByRole("textbox", { name: "Ingredient 1 name" }).fill("Rice");
  await page
    .getByRole("spinbutton", { name: "Ingredient 1 quantity" })
    .fill("100");
  await page.getByRole("textbox", { name: "Ingredient 1 unit" }).fill("g");
  await page
    .getByRole("button", { name: "Add ingredient", exact: true })
    .click();
  await page.getByRole("textbox", { name: "Ingredient 2 name" }).fill("Lemon");
  await page
    .getByRole("textbox", { name: "Instructions" })
    .fill("Cook rice. Add lemon.");
  await page.getByRole("button", { name: "Save recipe" }).click();
  await expect(page.getByRole("dialog")).not.toBeVisible();
  await page.getByRole("button", { name: new RegExp(name) }).click();
  await page.getByRole("button", { name: "Plan this meal" }).click();
  await page.getByRole("spinbutton", { name: "Servings" }).fill("4");
  await page.getByRole("button", { name: "Plan meal", exact: true }).click();
  await page
    .getByText("Optional plans for exact dates", { exact: true })
    .click();
  await expect(page.getByRole("heading", { name })).toBeVisible();
  await page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name, exact: true }) })
    .getByRole("button", { name: "Add ingredients" })
    .click();
  await expect(page.getByRole("spinbutton", { name: "Servings" })).toHaveValue(
    "4",
  );
  await page
    .getByRole("combobox", { name: "Shopping list", exact: true })
    .selectOption("0");
  await page
    .getByRole("textbox", { name: "New list name" })
    .fill(`Groceries ${info.project.name}`);
  await page
    .getByRole("button", { name: "Add ingredients", exact: true })
    .last()
    .click();
  await expect(
    page.getByRole("heading", { name: `Groceries ${info.project.name}` }),
  ).toBeVisible();
  await expect(page.getByText("200 g", { exact: true })).toBeVisible();
  await page.getByRole("checkbox", { name: "Rice" }).click();
  await expect(page.getByRole("checkbox", { name: "Rice" })).toBeChecked();
  await page.getByRole("textbox", { name: "Item name" }).fill("Milk");
  await page.getByRole("spinbutton", { name: "Item quantity" }).fill("2");
  await page.getByRole("textbox", { name: "Item unit" }).fill("L");
  await page.getByRole("button", { name: "Add shopping item" }).click();
  await expect(page.getByRole("checkbox", { name: "Milk" })).toBeVisible();
  await page.reload();
  await page
    .getByRole("button", { name: "Shopping lists", exact: true })
    .click();
  await page
    .getByRole("button", {
      name: `Groceries ${info.project.name}`,
      exact: true,
    })
    .click();
  await expect(page.getByRole("checkbox", { name: "Rice" })).toBeChecked();
  await expect(page.getByRole("checkbox", { name: "Milk" })).not.toBeChecked();
  await page.screenshot({
    path: `../artifacts/shopping-${info.project.name}.png`,
    fullPage: true,
  });
  await page.getByRole("button", { name: "Recipe book", exact: true }).click();
  await page.getByRole("textbox", { name: "Search recipes" }).fill(name);
  await page.screenshot({
    path: `../artifacts/recipes-${info.project.name}.png`,
    fullPage: true,
  });
  expect(errors).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
});

test("weekly wishlist stays flexible and shopping skips cooked meals", async ({
  page,
}, info) => {
  const title = `Weekly rice ${info.project.name}`;
  const response = await page.request.post("/api/recipes", {
    data: {
      title,
      servings: 2,
      ingredients: [{ name: "Weekly rice", quantity: 100, unit: "g" }],
    },
  });
  const recipe = await response.json();
  await page.goto("/");
  await page.getByRole("button", { name: new RegExp(title) }).click();
  await page.getByRole("button", { name: "Add to week", exact: true }).click();
  await page
    .getByLabel("Week of", { exact: true })
    .fill(info.project.name === "desktop" ? "2026-11-02" : "2026-11-09");
  await page
    .getByRole("spinbutton", { name: "Servings", exact: true })
    .fill("4");
  await expect(page.getByLabel("Date", { exact: true })).not.toBeVisible();
  await page
    .getByRole("button", { name: "Add to wishlist", exact: true })
    .click();
  const wish = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: title, exact: true }) });
  await expect(wish).toBeVisible();
  await expect(wish.getByText("SOMETIME THIS WEEK")).toBeVisible();
  const week = await page.getByLabel("Week of", { exact: true }).inputValue();
  const extra = await page.request.post("/api/wishes", {
    data: { recipe_id: recipe.id, week, servings: 10 },
  });
  const cooked = await extra.json();
  await page.request.patch(`/api/wishes/${cooked.id}`, {
    data: { cooked: true },
  });
  await page.reload();
  await page.getByRole("button", { name: "Meal plan", exact: true }).click();
  await page.getByLabel("Week of", { exact: true }).fill(week);
  await expect(
    page.getByRole("checkbox", { name: `Cooked ${title}` }),
  ).toHaveCount(2);
  await page
    .getByRole("button", { name: "Shop this week", exact: true })
    .click();
  await page
    .getByRole("combobox", { name: "Shopping list", exact: true })
    .selectOption("0");
  await page
    .getByRole("textbox", { name: "New list name" })
    .fill(`Weekly shop ${info.project.name}`);
  await page
    .getByRole("button", { name: "Add week’s ingredients", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: `Weekly shop ${info.project.name}` }),
  ).toBeVisible();
  const row = page.locator(".shopping-item").filter({
    has: page.getByRole("checkbox", { name: "Weekly rice", exact: true }),
  });
  await expect(row.getByText("200 g", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Meal plan", exact: true }).click();
  await page.screenshot({
    path: `../artifacts/wishlist-${info.project.name}.png`,
    fullPage: true,
  });
  await page.getByRole("button", { name: "Next week", exact: true }).click();
  await expect(
    page.getByRole("checkbox", { name: `Cooked ${title}` }),
  ).toHaveCount(0);
  await page
    .getByRole("button", { name: "Previous week", exact: true })
    .click();
  await expect(
    page.getByRole("checkbox", { name: `Cooked ${title}` }),
  ).toHaveCount(2);
});

test("recipe ratings and cook history persist and recalculate after edits", async ({
  page,
}, info) => {
  const title = `Cook journal ${info.project.name}`;
  await page.request.post("/api/recipes", {
    data: {
      title,
      servings: 2,
      ingredients: [{ name: "Rice", quantity: 100, unit: "g" }],
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: new RegExp(title) }).click();
  await page
    .getByRole("combobox", { name: "Recipe rating", exact: true })
    .selectOption("4");
  await expect(
    page.getByRole("combobox", { name: "Recipe rating", exact: true }),
  ).toHaveValue("4");
  await page.getByRole("button", { name: "Log a cook", exact: true }).click();
  await page.getByLabel("Cook date", { exact: true }).fill("2026-10-21");
  await page
    .getByRole("combobox", { name: "Cook rating", exact: true })
    .selectOption("5");
  await page.getByLabel("Cooked by (optional)", { exact: true }).fill("Both");
  await page.getByLabel("Went well", { exact: true }).fill("Great texture");
  await page.getByLabel("To improve", { exact: true }).fill("Less salt");
  await page
    .getByRole("textbox", { name: "Notes", exact: true })
    .fill("Used stock");
  await page.getByRole("button", { name: "Save cook", exact: true }).click();
  await expect(
    page.getByRole("dialog").getByText("1 cook", { exact: true }),
  ).toBeVisible();
  await expect(
    page
      .getByRole("dialog")
      .getByText("Average cook rating: 5/5", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("dialog").getByText("Great texture", { exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Log a cook", exact: true }).click();
  await page.getByLabel("Cook date", { exact: true }).fill("2026-10-24");
  await page
    .getByRole("combobox", { name: "Cook rating", exact: true })
    .selectOption("3");
  await page
    .getByRole("textbox", { name: "Notes", exact: true })
    .fill("Try a shorter simmer");
  await page.getByRole("button", { name: "Save cook", exact: true }).click();
  await expect(
    page.getByRole("dialog").getByText("2 cooks", { exact: true }),
  ).toBeVisible();
  await expect(
    page
      .getByRole("dialog")
      .getByText("Average cook rating: 4/5", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Edit cook on 2026-10-24", exact: true })
    .click();
  await expect(
    page.getByRole("combobox", { name: "Cook rating", exact: true }),
  ).toHaveValue("3");
  await page
    .getByRole("combobox", { name: "Cook rating", exact: true })
    .selectOption("2");
  await page
    .getByRole("textbox", { name: "Notes", exact: true })
    .fill("Changed my mind: too soft");
  await page.getByRole("button", { name: "Save cook", exact: true }).click();
  await expect(
    page
      .getByRole("dialog")
      .getByText("Average cook rating: 3.5/5", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("dialog").getByText("2 cooks", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("combobox", { name: "Recipe rating", exact: true }),
  ).toHaveValue("4");
  await page.screenshot({
    path: `../artifacts/cooks-${info.project.name}.png`,
    fullPage: true,
  });
  page.once("dialog", (d) => d.accept());
  await page
    .getByRole("button", { name: "Delete cook on 2026-10-24", exact: true })
    .click();
  await expect(
    page
      .getByRole("dialog")
      .getByText("Average cook rating: 5/5", { exact: true }),
  ).toBeVisible();
  await page.reload();
  await page.getByRole("button", { name: new RegExp(title) }).click();
  await expect(
    page.getByRole("dialog").getByText("1 cook", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("dialog").getByText("Great texture", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("dialog").getByText("Less salt", { exact: true }),
  ).toBeVisible();
  await expect(
    page
      .getByRole("dialog")
      .getByText("Last cooked: 2026-10-21", { exact: true }),
  ).toBeVisible();
  await page
    .getByRole("combobox", { name: "Recipe rating", exact: true })
    .selectOption("");
  await expect(
    page.getByRole("combobox", { name: "Recipe rating", exact: true }),
  ).toHaveValue("");
  await expect(
    page
      .getByRole("dialog")
      .getByText("Average cook rating: 5/5", { exact: true }),
  ).toBeVisible();
});
