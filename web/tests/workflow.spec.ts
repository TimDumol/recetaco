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

test("inventory counts can be corrected without losing receipt prices", async ({
  page,
  request,
}, info) => {
  const name = `inventory rice ${info.project.name}`;
  const imported = await request.post("/api/receipts", {
    data: {
      receipt_id: `inventory-${info.project.name}`,
      purchased_on: "2026-10-24",
      currency: "EUR",
      items: [{ name, quantity: 1000, unit: "g", total_cost: 2.5 }],
    },
  });
  expect(imported.ok()).toBeTruthy();
  await page.goto("/");
  await page.getByRole("button", { name: "Inventory", exact: true }).click();
  const row = page
    .locator("form")
    .filter({ has: page.getByText(name, { exact: true }) });
  await expect(row.getByRole("spinbutton")).toHaveValue("1000");
  await row.getByRole("spinbutton").fill("250");
  await row.getByRole("button", { name: "Save", exact: true }).click();
  await expect
    .poll(
      async () =>
        (await (await request.get("/api/inventory")).json()).find(
          (s: { name: string }) => s.name === name,
        )?.quantity,
    )
    .toBe(250);
  await row.getByRole("button", { name: "Out", exact: true }).click();
  await expect(row.getByRole("spinbutton")).toHaveValue("0");
  await row.getByRole("button", { name: "Forget", exact: true }).click();
  await expect(row).toHaveCount(0);
  await expect(
    page
      .locator(".inventory-row")
      .filter({ hasText: name })
      .getByText("2.50 EUR", { exact: true }),
  ).toBeVisible();
});

test("cooking flow keeps divided ingredient amounts next to dependent steps", async ({
  page,
  request,
}, info) => {
  const title = `Cooking flow ${info.project.name}`;
  const response = await request.post("/api/recipes", {
    data: {
      title,
      servings: 2,
      instructions: "Cook pasta while making sauce. Reserve oil for serving.",
      source_url: "",
      ingredients: [
        { name: "Pasta", quantity: 200, unit: "g" },
        { name: "Oil", quantity: 20, unit: "ml" },
      ],
      steps: [
        {
          text: "Boil pasta. Stir occasionally until tender.",
          ingredients: [{ ingredient: 0 }],
          after: [],
        },
        {
          text: "Make sauce",
          ingredients: [{ ingredient: 1, quantity: 15 }],
          after: [],
        },
        {
          text: "Combine and serve",
          ingredients: [{ ingredient: 1, quantity: 5 }],
          after: [0, 1],
        },
      ],
    },
  });
  expect(response.ok()).toBeTruthy();
  await page.goto("/");
  await page.getByRole("button", { name: new RegExp(title) }).click();
  const steps = page.locator(".cooking-step");
  await expect(steps.nth(0)).toContainText("200 g");
  await expect(steps.nth(1)).toContainText("15 ml");
  await expect(steps.nth(2)).toContainText("5 ml");
  await expect(steps.nth(2)).toContainText("After step 1 + step 2");
  await page
    .getByRole("dialog")
    .screenshot({ path: `../artifacts/cooking-flow-${info.project.name}.png` });
  await expect(
    page.getByText("2 parallel branches", { exact: false }),
  ).toBeVisible();
  await expect(
    steps.nth(0).locator(".flow-directions .instructions"),
  ).not.toBeVisible();
  await steps
    .nth(0)
    .getByText("Full directions for step 1", { exact: true })
    .click();
  await expect(
    steps.nth(0).locator(".flow-directions .instructions"),
  ).toContainText("Stir occasionally");
  await steps.nth(0).getByRole("checkbox").check();
  await page.getByRole("button", { name: "Method", exact: true }).click();
  await expect(steps.nth(0).getByRole("checkbox")).toBeChecked();
  await expect(page.locator(".flow-stage-heading")).toHaveCount(0);
  await expect(page.locator(".flow-directions")).toHaveCount(0);
  await expect(steps.nth(0).locator(".instructions")).toBeVisible();
  await expect(steps.nth(0)).toContainText("Stir occasionally");
  await page.getByRole("button", { name: "Flow", exact: true }).click();
  await expect(page.locator(".flow-stage-heading")).toHaveCount(2);
  await expect(steps.nth(0).getByRole("checkbox")).toBeChecked();

  await page.reload();
  await page.getByRole("button", { name: new RegExp(title) }).click();
  await expect(page.locator(".cooking-step").nth(2)).toContainText("5 ml");
});

test("edit recipe to link step amounts and keep them after saving", async ({
  page,
}, info) => {
  const title = `Linked steps ${info.project.name}`;
  await page.goto("/");
  await page.getByRole("button", { name: "New recipe", exact: true }).click();
  await page.getByRole("textbox", { name: "Recipe name" }).fill(title);
  await page.getByRole("textbox", { name: "Ingredient 1 name" }).fill("Oil");
  await page
    .getByRole("spinbutton", { name: "Ingredient 1 quantity" })
    .fill("20");
  await page.getByRole("textbox", { name: "Ingredient 1 unit" }).fill("ml");
  await page
    .getByRole("textbox", { name: "Instructions", exact: true })
    .fill("Heat oil.\n\nServe with oil.");
  await page
    .getByRole("button", { name: "Make steps from instructions" })
    .click();
  await page
    .getByRole("group", { name: "Step 1", exact: true })
    .getByRole("checkbox", { name: "Oil" })
    .check();
  await page
    .getByRole("spinbutton", { name: "Amount in step 1: Oil" })
    .fill("15");
  await page
    .getByRole("group", { name: "Step 2", exact: true })
    .getByRole("checkbox", { name: "Oil" })
    .check();
  await page
    .getByRole("spinbutton", { name: "Amount in step 2: Oil" })
    .fill("5");
  await page.getByRole("button", { name: "Save recipe", exact: true }).click();
  await expect(page.getByRole("dialog")).not.toBeVisible();
  await page.getByRole("button", { name: new RegExp(title) }).click();
  await expect(page.locator(".cooking-step").nth(0)).toContainText("15 ml");
  await expect(page.locator(".cooking-step").nth(1)).toContainText("5 ml");
});
