import { useEffect, useState } from "react";
import { api, quantity, type Recipe } from "./api";
type Stock = {
  name: string;
  unit: string;
  quantity: number;
  updated_at?: string;
};
type Purchase = Stock & {
  id: number;
  receipt_id: string;
  total_cost: number | null;
  currency: string;
  purchased_on: string;
};
export function Inventory() {
  const [items, setItems] = useState<Stock[]>([]);
  const [purchases, setPurchases] = useState<Purchase[]>([]);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [draft, setDraft] = useState<Stock>({
    name: "",
    quantity: 1,
    unit: "",
  });
  async function load() {
    const [stock, history] = await Promise.all([
      api<Stock[]>("/inventory"),
      api<Purchase[]>("/purchases"),
    ]);
    setItems(stock);
    setPurchases(history);
  }
  useEffect(() => {
    void load().catch((e) => setError(String(e)));
  }, []);
  async function save(stock: Stock, method = "PUT") {
    setBusy(true);
    setError("");
    try {
      setItems(await api<Stock[]>("/inventory", method, stock));
      return true;
    } catch (e) {
      setError(String(e));
      return false;
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="inventory-page">
      <p>
        Your best guess is enough. Untracked ingredients are unknown. Recipe and
        weekly shopping additions subtract matching stock once per list; manual
        shopping items always add in full.
      </p>
      <p>
        Use the same names and units as your recipes. Adjust counts after
        cooking, or set an item to zero when it runs out. Forget removes
        tracking; purchase prices stay in history.
      </p>
      <p>
        Give ChatGPT a receipt and ask it to import it into Recetaco using MCP.
        Pack quantities should be converted to recipe units only when the
        receipt gives enough information.
      </p>
      {error && (
        <p role="alert" className="alert">
          {error}
        </p>
      )}
      <h2>Add or replace a count</h2>
      <form
        className="inventory-row"
        onSubmit={async (e) => {
          e.preventDefault();
          if (await save(draft)) setDraft({ name: "", quantity: 1, unit: "" });
        }}
      >
        <label>
          Ingredient
          <input
            required
            maxLength={200}
            value={draft.name}
            onChange={(e) => setDraft({ ...draft, name: e.target.value })}
          />
        </label>
        <label>
          Quantity
          <input
            type="number"
            min="0"
            max="1000000"
            step="any"
            required
            value={draft.quantity}
            onChange={(e) =>
              setDraft({ ...draft, quantity: e.target.valueAsNumber })
            }
          />
        </label>
        <label>
          Unit
          <input
            maxLength={80}
            placeholder="g, ml, or leave blank"
            value={draft.unit}
            onChange={(e) => setDraft({ ...draft, unit: e.target.value })}
          />
        </label>
        <button className="primary" disabled={busy}>
          Save count
        </button>
      </form>
      <h2>On hand</h2>
      {!items.length && (
        <p>No counts yet. Add a few staples or import a receipt to start.</p>
      )}
      {items.map((stock) => (
        <StockRow
          key={`${stock.name}/${stock.unit}`}
          stock={stock}
          busy={busy}
          save={save}
        />
      ))}
      <h2>Purchase history</h2>
      <p>
        Recipe estimates use the latest price for a matching ingredient and
        unit. Missing prices stay unknown, and currencies are kept separate.
      </p>
      {!purchases.length && <p>No purchases recorded yet.</p>}
      {purchases.map((p) => (
        <div className="inventory-row" key={p.id}>
          <span>
            {p.purchased_on} · {p.name}
          </span>
          <span>
            {quantity(p.quantity)} {p.unit}
          </span>
          <span>
            {p.total_cost === null
              ? "Price unknown"
              : `${p.total_cost.toFixed(2)} ${p.currency}`}
          </span>
          <small>{p.receipt_id}</small>
        </div>
      ))}
    </div>
  );
}
function StockRow({
  stock,
  busy,
  save,
}: {
  stock: Stock;
  busy: boolean;
  save: (s: Stock, method?: string) => Promise<boolean>;
}) {
  const [count, setCount] = useState(stock.quantity);
  useEffect(() => setCount(stock.quantity), [stock.quantity]);
  return (
    <form
      className="inventory-row"
      onSubmit={(e) => {
        e.preventDefault();
        void save({ ...stock, quantity: count });
      }}
    >
      <strong>{stock.name}</strong>
      <label>
        Quantity
        <input
          aria-label={`${stock.name} quantity`}
          type="number"
          min="0"
          max="1000000"
          step="any"
          required
          value={count}
          onChange={(e) => setCount(e.target.valueAsNumber)}
        />
      </label>
      <span>{stock.unit}</span>
      <small>Count updated {stock.updated_at}</small>
      <button disabled={busy}>Save</button>
      <button
        type="button"
        disabled={busy}
        onClick={() => void save({ ...stock, quantity: 0 })}
      >
        Out
      </button>
      <button
        type="button"
        disabled={busy}
        onClick={() => void save(stock, "DELETE")}
      >
        Forget
      </button>
    </form>
  );
}
type Estimate = {
  totals: Record<string, number>;
  per_serving: Record<string, number>;
  priced_ingredients: number;
  total_ingredients: number;
  complete: boolean;
  ingredients: {
    name: string;
    cost: number | null;
    currency?: string;
    price_date?: string;
  }[];
};
export function RecipeCost({ recipe }: { recipe: Recipe }) {
  const [cost, setCost] = useState<Estimate>();
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    void api<Estimate>(`/recipes/${recipe.id}/cost`)
      .then((v) => {
        if (active) setCost(v);
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
    };
  }, [recipe]);
  return (
    <section>
      <h3>Estimated ingredient cost</h3>
      {error ? (
        <p role="alert">{error}</p>
      ) : !cost ? (
        <p>Loading prices…</p>
      ) : (
        <>
          <p>
            {cost.priced_ingredients} of {cost.total_ingredients} ingredients
            priced{cost.complete ? "" : " · partial estimate"}
          </p>
          {Object.entries(cost.totals).map(([currency, total]) => (
            <p key={currency}>
              <strong>
                {total.toFixed(2)} {currency}
              </strong>{" "}
              · {cost.per_serving[currency].toFixed(2)} per serving
            </p>
          ))}
          <details>
            <summary>Prices used</summary>
            {cost.ingredients.map((i, n) => (
              <p key={n}>
                {i.name}:{" "}
                {i.cost === null
                  ? "price unknown"
                  : `${i.cost.toFixed(2)} ${i.currency} (purchase ${i.price_date})`}
              </p>
            ))}
          </details>
        </>
      )}
    </section>
  );
}
