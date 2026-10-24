# Approximate inventory and recipe costs

Inventory is shared by the kitchen. The Inventory page lets you add or replace a count, mark it out (zero), or forget it. Forgetting stock preserves historical purchase prices. Update counts after cooking or other purchases; cook logs and shopping checkboxes do not change inventory.

When recipes or uncooked weekly wishes are added to a shopping list, known inventory reduces matching ingredient quantities. Names and units match after trimming and lowercasing. No fuzzy matching, unit conversion, expiry, or assumption about untracked ingredients is applied. A manual shopping item always adds its full quantity, allowing you to override a pantry guess.

Stock is allocated only once per list, including across repeated recipe/week additions. Allocations are estimates at the time of adding; editing counts later does not rewrite an existing list. Separate lists each consider the current pantry independently. Start a fresh list for a new shopping trip.

## Receipts through ChatGPT / MCP

Connect ChatGPT to the existing `/mcp` endpoint, attach a receipt in ChatGPT, and ask it to import the groceries. Recetaco accepts structured data, not receipt images: ChatGPT reads the receipt and calls `import_receipt`. First read `list_recipes` and `list_inventory` to choose consistent ingredient names and units. Convert packs into grams/ml only when their contents are known. Leave uncertain prices null; ask about ambiguous quantities rather than inventing them.

Example tool arguments:

```json
{
  "receipt_id": "grocer-2026-10-24-transaction-123",
  "purchased_on": "2026-10-24",
  "currency": "EUR",
  "items": [
    {"name": "rice", "quantity": 1000, "unit": "g", "total_cost": 2.49},
    {"name": "eggs", "quantity": 6, "unit": "", "total_cost": null}
  ]
}
```

`total_cost` is the line total after discounts, not the unit price. Import adds inventory and purchase history in one transaction. Retrying identical contents with the same receipt ID is safe. Reusing the ID with different contents fails. Different IDs represent different purchases, so reuse a stable identifier. Inventory is an approximate current count: importing an old receipt also adds stock, which you can correct on the Inventory page.

Other tools: `set_inventory` replaces an absolute count, `list_purchase_history` reads purchases, and `estimate_recipe_cost` returns cost coverage, ingredient estimates and currency-separated totals/per-serving estimates.

## Cost estimates

Recipe details show estimates based on the latest dated purchase with a known price for each matching ingredient and unit. Missing prices remain unknown; partial totals are labelled with their coverage. Currencies are never combined or converted. Estimates include ingredients already in inventory and represent ingredient cost, not the cost of buying whole packs. Purchase dates are available under Prices used.

Authenticated HTTP routes: `GET/PUT/DELETE /api/inventory`, `POST /api/receipts`, `GET /api/purchases`, `GET /api/recipes/{id}/cost`. PUT replaces a count; DELETE takes the same stock object and forgets it. Export version 4 includes inventory, purchase history and original structured receipts.
