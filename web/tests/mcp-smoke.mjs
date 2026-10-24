import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";
import assert from "node:assert/strict";
const client = new Client({ name: "recetaco-smoke", version: "1.0.0" });
const transport = new StreamableHTTPClientTransport(
  new URL(process.env.RECETACO_MCP_URL || "http://127.0.0.1:18081/mcp"),
  {
    requestInit: process.env.RECETACO_TOKEN
      ? { headers: { Authorization: `Bearer ${process.env.RECETACO_TOKEN}` } }
      : undefined,
  },
);
try {
  await client.connect(transport);
  const { tools } = await client.listTools();
  assert.equal(tools.length, 20);
  const name = `MCP smoke ${Date.now()}`;
  const r = await client.callTool({
    name: "create_recipe",
    arguments: {
      title: name,
      servings: 2,
      ingredients: [{ name: "Rice", quantity: 100, unit: "g" }],
    },
  });
  assert.equal(r.isError, false);
  const recipe = JSON.parse(r.content[0].text);
  const l = await client.callTool({
    name: "create_shopping_list",
    arguments: { name },
  });
  const list = JSON.parse(l.content[0].text);
  const added = await client.callTool({
    name: "add_recipe_to_shopping_list",
    arguments: { list_id: list.id, recipe_id: recipe.id, servings: 4 },
  });
  assert.equal(JSON.parse(added.content[0].text).items[0].quantity, 200);
  const wishResult = await client.callTool({
    name: "add_to_weekly_wishlist",
    arguments: { recipe_id: recipe.id, week: "2026-11-02", servings: 4 },
  });
  const wish = JSON.parse(wishResult.content[0].text);
  assert.equal(wishResult.isError, false);
  const weekly = await client.callTool({
    name: "list_weekly_wishlist",
    arguments: { week: "2026-11-02" },
  });
  assert.ok(JSON.parse(weekly.content[0].text).some((w) => w.id === wish.id));
  await client.callTool({
    name: "mark_wishlist_cooked",
    arguments: { wish_id: wish.id, cooked: true },
  });
  const skipped = await client.callTool({
    name: "add_week_to_shopping_list",
    arguments: { list_id: list.id, week: "2026-11-02" },
  });
  assert.equal(JSON.parse(skipped.content[0].text).items[0].quantity, 200);
  await client.callTool({
    name: "mark_wishlist_cooked",
    arguments: { wish_id: wish.id, cooked: false },
  });
  const weekAdded = await client.callTool({
    name: "add_week_to_shopping_list",
    arguments: { list_id: list.id, week: "2026-11-02" },
  });
  assert.equal(JSON.parse(weekAdded.content[0].text).items[0].quantity, 400);
  const rated = await client.callTool({
    name: "rate_recipe",
    arguments: { recipe_id: recipe.id, rating: 4 },
  });
  assert.equal(JSON.parse(rated.content[0].text).rating, 4);
  const cookResult = await client.callTool({
    name: "log_cook",
    arguments: {
      recipe_id: recipe.id,
      cooked_on: "2026-10-24",
      rating: 5,
      went_well: "Rice texture",
      to_improve: "Less salt",
      notes: "Used stock",
    },
  });
  const cook = JSON.parse(cookResult.content[0].text);
  assert.equal(cookResult.isError, false);
  const history = await client.callTool({
    name: "list_cooks",
    arguments: { recipe_id: recipe.id },
  });
  assert.equal(JSON.parse(history.content[0].text)[0].to_improve, "Less salt");
  await client.callTool({
    name: "update_cook",
    arguments: {
      recipe_id: recipe.id,
      cook_id: cook.id,
      cooked_on: "2026-10-24",
      rating: 3,
      notes: "Next time: rinse rice",
    },
  });
  const after = await client.callTool({
    name: "get_recipe",
    arguments: { recipe_id: recipe.id },
  });
  assert.equal(JSON.parse(after.content[0].text).average_cook_rating, 3);
  assert.equal(JSON.parse(after.content[0].text).rating, 4);
  await client.callTool({
    name: "delete_cook",
    arguments: { recipe_id: recipe.id, cook_id: cook.id },
  });
  const cleared = await client.callTool({
    name: "get_recipe",
    arguments: { recipe_id: recipe.id },
  });
  assert.equal(JSON.parse(cleared.content[0].text).cook_count, 0);
  console.log(
    "Official MCP SDK: 20 tools; shopping, weekly wishes, recipe ratings, cook history CRUD and averages — passed.",
  );
} finally {
  await client.close();
}
