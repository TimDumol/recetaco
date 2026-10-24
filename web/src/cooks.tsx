import { useEffect, useState } from "react";
import { api, today, type Recipe } from "./api";
export interface CookInput {
  cooked_on: string;
  rating: number | null;
  cook_name: string;
  went_well: string;
  to_improve: string;
  notes: string;
}
export interface Cook extends CookInput {
  id: number;
  recipe_id: number;
}
export function ratingLabel(value: number | null | undefined): string {
  return value ? `${value}/5` : "Not rated";
}
export function CookSummary({ recipe }: { recipe: Recipe }) {
  return (
    <div className="cook-summary">
      <span>
        {recipe.cook_count || 0}{" "}
        {(recipe.cook_count || 0) === 1 ? "cook" : "cooks"}
      </span>
      <span>
        Average cook rating:{" "}
        {recipe.average_cook_rating
          ? `${Number(recipe.average_cook_rating.toFixed(1))}/5`
          : "Not rated"}
      </span>
      {recipe.last_cooked && <span>Last cooked: {recipe.last_cooked}</span>}
    </div>
  );
}
export function CookHistory({
  recipe,
  busy,
  edit,
  remove,
}: {
  recipe: Recipe;
  busy: boolean;
  edit: (cook: Cook) => void;
  remove: (cook: Cook) => void;
}) {
  const [cooks, setCooks] = useState<Cook[]>([]);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const [attempt, retry] = useState(0);
  useEffect(() => {
    let active = true;
    setLoading(true);
    setError("");
    api<Cook[]>(`/recipes/${recipe.id}/cooks`)
      .then((data) => {
        if (active) setCooks(data);
      })
      .catch((e) => {
        if (active) setError(e.message);
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [recipe.id, recipe.cook_count, attempt]);
  if (loading) return <p className="muted">Loading cook history…</p>;
  if (error)
    return (
      <div className="alert" role="alert">
        {error}
        <button onClick={() => retry((n) => n + 1)}>Retry</button>
      </div>
    );
  return (
    <div className="cook-history">
      {!cooks.length && (
        <p className="muted">
          No cooks logged yet. Keep a note of what worked for next time.
        </p>
      )}
      {cooks.map((c) => (
        <article className="cook-entry" key={c.id}>
          <div className="cook-entry-heading">
            <strong>
              {c.cooked_on}
              {c.cook_name ? ` · ${c.cook_name}` : ""}
            </strong>
            <span className="rating-badge">{ratingLabel(c.rating)}</span>
          </div>
          {c.went_well && (
            <div>
              <h4>Went well</h4>
              <p>{c.went_well}</p>
            </div>
          )}
          {c.to_improve && (
            <div>
              <h4>To improve</h4>
              <p>{c.to_improve}</p>
            </div>
          )}
          {c.notes && (
            <div>
              <h4>Notes</h4>
              <p>{c.notes}</p>
            </div>
          )}
          <div className="cook-entry-actions">
            <button
              className="text-link"
              disabled={busy}
              onClick={() => edit(c)}
              aria-label={`Edit cook on ${c.cooked_on}`}
            >
              Edit
            </button>
            <button
              className="text-link danger"
              disabled={busy}
              onClick={() => remove(c)}
              aria-label={`Delete cook on ${c.cooked_on}`}
            >
              Delete
            </button>
          </div>
        </article>
      ))}
    </div>
  );
}
export function CookForm({
  cook,
  busy,
  submit,
}: {
  cook?: Cook;
  busy: boolean;
  submit: (data: CookInput) => void;
}) {
  const [data, setData] = useState<CookInput>(
    cook || {
      cooked_on: today(),
      rating: null,
      cook_name: "",
      went_well: "",
      to_improve: "",
      notes: "",
    },
  );
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        submit(data);
      }}
    >
      <div className="form-row">
        <label>
          Cook date
          <input
            type="date"
            required
            value={data.cooked_on}
            onChange={(e) => setData({ ...data, cooked_on: e.target.value })}
          />
        </label>
        <label>
          Cook rating
          <select
            value={data.rating ?? ""}
            onChange={(e) =>
              setData({
                ...data,
                rating: e.target.value ? Number(e.target.value) : null,
              })
            }
          >
            <option value="">Not rated</option>
            {[1, 2, 3, 4, 5].map((n) => (
              <option key={n} value={n}>
                {n}/5
              </option>
            ))}
          </select>
        </label>
      </div>
      <label>
        Cooked by (optional)
        <input
          maxLength={100}
          placeholder="Your name, your partner, or both"
          value={data.cook_name}
          onChange={(e) => setData({ ...data, cook_name: e.target.value })}
        />
      </label>
      <label>
        Went well
        <textarea
          rows={3}
          maxLength={10000}
          placeholder="What would you repeat?"
          value={data.went_well}
          onChange={(e) => setData({ ...data, went_well: e.target.value })}
        />
      </label>
      <label>
        To improve
        <textarea
          rows={3}
          maxLength={10000}
          placeholder="What would you change next time?"
          value={data.to_improve}
          onChange={(e) => setData({ ...data, to_improve: e.target.value })}
        />
      </label>
      <label>
        Notes
        <textarea
          rows={3}
          maxLength={10000}
          placeholder="Substitutions, timing, or anything else to remember"
          value={data.notes}
          onChange={(e) => setData({ ...data, notes: e.target.value })}
        />
      </label>
      <p className="form-hint">
        This records one cooking occasion. Wishlist checkboxes and the shared
        recipe rating stay separate.
      </p>
      <div className="dialog-actions">
        <button className="primary" disabled={busy}>
          {busy ? "Saving…" : "Save cook"}
        </button>
      </div>
    </form>
  );
}
