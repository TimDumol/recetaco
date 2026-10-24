import {
  CookForm,
  CookHistory,
  CookSummary,
  ratingLabel,
  type Cook,
} from "./cooks";
import { RecipePhoto, photoRequest, resizePhoto } from "./photos";
import { monday, shiftWeek, type Wish } from "./api";
import { AuthGate, authConfig, signOut } from "./auth";
import React, { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import {
  BookOpen,
  CalendarDays,
  ShoppingBasket,
  Plus,
  Search,
  ArrowUpRight,
  X,
  Minus,
  Settings,
  Flame,
  Utensils,
  Check,
  Trash2,
  Download,
  ArrowRight,
  Leaf,
  Pencil,
  LoaderCircle,
} from "lucide-react";
import {
  api,
  today,
  quantity,
  type Recipe,
  type RecipeInput,
  type Meal,
  type ShoppingList,
  type ListDetail,
  type Ingredient,
} from "./api";
import "@fontsource/dm-sans/latin-400.css";
import "@fontsource/dm-sans/latin-500.css";
import "@fontsource/dm-sans/latin-600.css";
import "@fontsource/dm-sans/latin-700.css";
import "@fontsource/libre-caslon-display/latin-400.css";
import "./styles.css";

type Page = "recipes" | "plan" | "shopping";
type Dialog =
  | { kind: "edit"; recipe?: Recipe }
  | { kind: "recipe"; recipe: Recipe }
  | { kind: "cook"; recipe: Recipe; cook?: Cook }
  | { kind: "plan"; recipe?: Recipe }
  | { kind: "wish"; recipe?: Recipe }
  | { kind: "week-shop" }
  | { kind: "shop"; recipe: Recipe }
  | { kind: "list" }
  | { kind: "settings" };
const blankIngredient = (): Ingredient => ({ name: "", quantity: 1, unit: "" });
const sample: RecipeInput = {
  title: "Lemony chickpea bowls",
  servings: 2,
  source_url: "",
  instructions:
    "1. Cook the rice in lightly salted water.\n2. Warm the chickpeas with olive oil and a pinch of salt.\n3. Divide between bowls. Add sliced cucumber, a squeeze of lemon, and a little more olive oil.",
  ingredients: [
    { name: "Chickpeas", quantity: 1, unit: "can" },
    { name: "Rice", quantity: 150, unit: "g" },
    { name: "Cucumber", quantity: 1, unit: "" },
    { name: "Lemon", quantity: 1, unit: "" },
    { name: "Olive oil", quantity: 2, unit: "tbsp" },
  ],
};
function Modal({
  title,
  close,
  children,
}: {
  title: string;
  close: () => void;
  children: React.ReactNode;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    ref.current?.showModal();
  }, []);
  return (
    <dialog
      ref={ref}
      onCancel={(e) => {
        e.preventDefault();
        close();
      }}
      onClick={(e) => {
        if (e.target === ref.current) close();
      }}
    >
      <header className="dialog-header">
        <h2>{title}</h2>
        <button
          className="icon-button"
          aria-label="Close dialog"
          onClick={close}
        >
          <X size={20} />
        </button>
      </header>
      {children}
    </dialog>
  );
}
function App() {
  const [page, setPage] = useState<Page>("recipes");
  const [recipes, setRecipes] = useState<Recipe[]>([]);
  const [meals, setMeals] = useState<Meal[]>([]);
  const [week, setWeek] = useState(monday());
  const [wishes, setWishes] = useState<Wish[]>([]);
  const [lists, setLists] = useState<ShoppingList[]>([]);
  const [selectedList, setSelectedList] = useState<number | null>(null);
  const [detail, setDetail] = useState<ListDetail | null>(null);
  const [search, setSearch] = useState("");
  const [dialog, setDialog] = useState<Dialog | null>(null);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [ready, setReady] = useState(false);
  const [manual, setManual] = useState<Ingredient>(blankIngredient);
  async function refresh(selectedWeek = week) {
    const [r, m, l, w] = await Promise.all([
      api<Recipe[]>("/recipes"),
      api<Meal[]>("/meals"),
      api<ShoppingList[]>("/lists"),
      api<Wish[]>(`/wishes?week=${selectedWeek}`),
    ]);
    setRecipes(r);
    setWishes(w);
    setMeals(m);
    setLists(l);
    setSelectedList((id) =>
      l.some((x) => x.id === id) ? id : (l[0]?.id ?? null),
    );
    setReady(true);
  }
  useEffect(() => {
    refresh()
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);
  useEffect(() => {
    let cancelled = false;
    setWishes([]);
    api<Wish[]>(`/wishes?week=${week}`)
      .then((w) => {
        if (!cancelled) setWishes(w);
      })
      .catch((e) => {
        if (!cancelled) setError(e.message);
      });
    return () => {
      cancelled = true;
    };
  }, [week]);
  useEffect(() => {
    let cancelled = false;
    setDetail(null);
    if (selectedList !== null)
      api<ListDetail>(`/lists/${selectedList}`)
        .then((d) => {
          if (!cancelled) setDetail(d);
        })
        .catch((e) => {
          if (!cancelled) setError(e.message);
        });
    return () => {
      cancelled = true;
    };
  }, [selectedList]);
  useEffect(() => {
    if (notice) {
      const t = setTimeout(() => setNotice(""), 4000);
      return () => clearTimeout(t);
    }
  }, [notice]);
  async function act(fn: () => Promise<void>, message?: string) {
    setBusy(true);
    setError("");
    try {
      await fn();
      if (message) setNotice(message);
    } catch (e) {
      setError(
        e instanceof Error
          ? e.message
          : "Something went wrong. Please try again.",
      );
    } finally {
      setBusy(false);
    }
  }
  async function reloadList(id: number) {
    const d = await api<ListDetail>(`/lists/${id}`);
    setDetail(d);
  }
  const visible = recipes.filter((r) =>
    r.title.toLowerCase().includes(search.toLowerCase()),
  );
  const upcoming = meals.filter((m) => m.date >= today());
  const nav = [
    { id: "recipes" as const, label: "Recipe book", icon: BookOpen },
    { id: "plan" as const, label: "Meal plan", icon: CalendarDays },
    { id: "shopping" as const, label: "Shopping lists", icon: ShoppingBasket },
  ];
  const close = () => {
    if (!busy) setDialog(null);
  };
  const onSubmit = (e: React.FormEvent, fn: () => Promise<void>) => {
    e.preventDefault();
    void act(fn);
  };
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <a
          className="brand"
          href="#"
          onClick={(e) => {
            e.preventDefault();
            setPage("recipes");
          }}
        >
          <span className="brand-icon">
            <Flame size={25} />
          </span>{" "}
          recetaco<span className="brand-dot">.</span>
        </a>
        <p className="brand-caption">GOOD FOOD. LESS FUSS.</p>
        <nav aria-label="Main navigation">
          {nav.map(({ id, label, icon: Icon }) => (
            <button
              key={id}
              className={page === id ? "nav-item active" : "nav-item"}
              onClick={() => setPage(id)}
            >
              <Icon size={20} />
              <span>{label}</span>
              {page === id && <span className="nav-dot" />}
            </button>
          ))}
        </nav>
        <div className="sidebar-note">
          <Leaf size={25} />
          <p>
            A little planning.
            <br />
            More room for living.
          </p>
          <span>FROM YOUR KITCHEN, WITH LOVE</span>
        </div>
        <button
          className="nav-item settings-link"
          onClick={() => setDialog({ kind: "settings" })}
        >
          <Settings size={18} />
          <span>Connection settings</span>
        </button>
      </aside>
      <main>
        <div className="topline">
          <span>YOUR EVERYDAY KITCHEN COMPANION</span>
          <span className="private-badge">
            <span /> Your kitchen, your data
          </span>
        </div>
        <header className="page-header">
          <div>
            <div className="eyebrow">MAKE ROOM FOR SOMETHING GOOD</div>
            <h1>
              {page === "recipes"
                ? "Keep the good ones."
                : page === "plan"
                  ? "A week worth looking forward to."
                  : "A little list. A lighter week."}
            </h1>
            <p>
              {page === "recipes"
                ? "Your favorite recipes, all in one happy place."
                : page === "plan"
                  ? "A few meals you’d like to cook, whenever the mood takes you."
                  : "Everything you need, ready when you are."}
            </p>
          </div>
          <button
            className="primary"
            disabled={busy || !ready}
            onClick={() =>
              setDialog(
                page === "recipes"
                  ? { kind: "edit" }
                  : page === "plan"
                    ? { kind: "wish" }
                    : { kind: "list" },
              )
            }
          >
            <Plus size={18} />
            {page === "recipes"
              ? "New recipe"
              : page === "plan"
                ? "Add to week"
                : "New list"}
          </button>
        </header>
        {error && !dialog && (
          <div role="alert" className="alert">
            {error}
            <button onClick={() => void act(refresh)}>Retry</button>
          </div>
        )}
        {notice && (
          <div className="toast" role="status">
            <Check size={18} />
            {notice}
          </div>
        )}
        {loading ? (
          <div className="empty">
            <LoaderCircle className="spin" />
            <h2>Opening your kitchen…</h2>
          </div>
        ) : page === "recipes" ? (
          <>
            <section className="kitchen-banner">
              <div className="banner-copy">
                <span className="eyebrow">
                  A COLLECTION THAT FEELS LIKE HOME
                </span>
                <h2>Good meals start here.</h2>
                <p>
                  Save a favorite. Plan a dinner. Make the list.
                  <br />
                  The rest is the good part.
                </p>
                <button className="text-link" onClick={() => setPage("plan")}>
                  Let’s plan something <ArrowRight size={17} />
                </button>
              </div>
              <div className="plate-art" aria-hidden="true">
                <div className="plate">
                  <span className="leaf leaf1" />
                  <span className="leaf leaf2" />
                  <span className="leaf leaf3" />
                  <span className="tomato t1" />
                  <span className="tomato t2" />
                  <span className="tomato t3" />
                  <span className="grain g1" />
                  <span className="grain g2" />
                  <span className="grain g3" />
                </div>
                <span className="art-label">a pinch of inspiration</span>
              </div>
            </section>
            <div className="section-toolbar">
              <h2>
                Your recipe book <span className="count">{recipes.length}</span>
              </h2>
              <label className="search">
                <Search size={17} />
                <input
                  aria-label="Search recipes"
                  placeholder="Find a favorite…"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                />
              </label>
            </div>
            {!visible.length ? (
              <div className="empty">
                <BookOpen size={32} />
                <h2>
                  {search
                    ? "No recipes found"
                    : "Every kitchen starts with a favorite."}
                </h2>
                <p>
                  {search
                    ? "Try another name."
                    : "Add your first recipe, or try a simple chickpea bowl."}
                </p>
                {!search && (
                  <button
                    className="secondary"
                    disabled={busy || !ready}
                    onClick={() =>
                      void act(async () => {
                        await api("/recipes", "POST", sample);
                        await refresh();
                      }, "Your first recipe is ready.")
                    }
                  >
                    Try a sample recipe <ArrowUpRight size={16} />
                  </button>
                )}
              </div>
            ) : (
              <div className="recipe-grid">
                {visible.map((r, i) => (
                  <button
                    key={r.id}
                    className="recipe-card"
                    onClick={() => setDialog({ kind: "recipe", recipe: r })}
                  >
                    <div className={`recipe-art art-${i % 4}`}>
                      {r.photo_version ? (
                        <RecipePhoto recipe={r} />
                      ) : (
                        <Utensils size={44} strokeWidth={1} />
                      )}
                      <span>{r.ingredients.length} ingredients</span>
                    </div>
                    <div className="recipe-card-body">
                      <span className="eyebrow">FROM YOUR RECIPE BOOK</span>
                      <h3>{r.title}</h3>
                      <div className="recipe-rating-meta">
                        <span
                          aria-label={`Recipe rating ${ratingLabel(r.rating)}`}
                        >
                          ★ {ratingLabel(r.rating)}
                        </span>
                        <span>
                          {r.cook_count || 0}{" "}
                          {(r.cook_count || 0) === 1 ? "cook" : "cooks"}
                        </span>
                      </div>
                      <div className="card-meta">
                        <span>Serves {quantity(r.servings)}</span>
                        <ArrowUpRight size={19} />
                      </div>
                    </div>
                  </button>
                ))}
              </div>
            )}
          </>
        ) : page === "plan" ? (
          <>
            <div className="week-toolbar">
              <button
                className="icon-button"
                disabled={busy}
                aria-label="Previous week"
                onClick={() => setWeek(shiftWeek(week, -7))}
              >
                <ArrowRight className="flip" size={20} />
              </button>
              <label>
                Week of
                <input
                  type="date"
                  disabled={busy}
                  value={week}
                  onChange={(e) => {
                    if (e.target.value) setWeek(monday(e.target.value));
                  }}
                />
              </label>
              <button
                className="icon-button"
                disabled={busy}
                aria-label="Next week"
                onClick={() => setWeek(shiftWeek(week, 7))}
              >
                <ArrowRight size={20} />
              </button>
              <button
                className="secondary"
                disabled={busy}
                onClick={() => setWeek(monday())}
              >
                This week
              </button>
            </div>
            <div className="section-toolbar">
              <div>
                <h2>
                  This week’s wishlist{" "}
                  <span className="count">
                    {wishes.filter((w) => !w.cooked).length} to cook
                  </span>
                </h2>
                <p className="muted">
                  {week} – {shiftWeek(week, 6)} · Pick what sounds good. Decide
                  the day later.
                </p>
              </div>
              <button
                className="secondary"
                disabled={busy || !wishes.some((w) => !w.cooked)}
                onClick={() => setDialog({ kind: "week-shop" })}
              >
                <ShoppingBasket size={17} />
                Shop this week
              </button>
            </div>
            {!wishes.length ? (
              <div className="empty">
                <Leaf size={30} />
                <h2>A few things you’d love to cook.</h2>
                <p>Add meals for the week. No dates or meal slots needed.</p>
                <button
                  className="primary"
                  disabled={!recipes.length || busy}
                  onClick={() => setDialog({ kind: "wish" })}
                >
                  Add to week
                </button>
              </div>
            ) : (
              <div className="meal-list">
                {wishes.map((w) => (
                  <article
                    className={`meal-row ${w.cooked ? "cooked" : ""}`}
                    key={w.id}
                  >
                    <label className="wish-check">
                      <input
                        type="checkbox"
                        checked={w.cooked}
                        aria-label={`Cooked ${w.title}`}
                        disabled={busy}
                        onChange={() =>
                          void act(async () => {
                            await api(`/wishes/${w.id}`, "PATCH", {
                              cooked: !w.cooked,
                            });
                            await refresh();
                          })
                        }
                      />
                      <span className="sr-only">Cooked</span>
                    </label>
                    <div className="meal-title">
                      <span className="eyebrow">
                        {w.cooked ? "COOKED · NICE ONE" : "SOMETIME THIS WEEK"}
                      </span>
                      <h3>{w.title}</h3>
                      <span className="muted">
                        Serves {quantity(w.servings)}
                      </span>
                    </div>
                    <button
                      className="secondary"
                      aria-label={`Shop ingredients for ${w.title}`}
                      disabled={busy}
                      onClick={() => {
                        const r = recipes.find((r) => r.id === w.recipe_id);
                        if (r)
                          setDialog({
                            kind: "shop",
                            recipe: { ...r, servings: w.servings },
                          });
                      }}
                    >
                      <ShoppingBasket size={17} />
                    </button>
                    <button
                      className="icon-button"
                      aria-label={`Remove ${w.title} from wishlist`}
                      disabled={busy}
                      onClick={() =>
                        void act(async () => {
                          await api(`/wishes/${w.id}`, "DELETE");
                          await refresh();
                        })
                      }
                    >
                      <X size={18} />
                    </button>
                  </article>
                ))}
              </div>
            )}
            <details className="dated-plans">
              <summary>Optional plans for exact dates</summary>
              <button
                className="secondary"
                disabled={busy}
                onClick={() => setDialog({ kind: "plan" })}
              >
                <Plus size={17} />
                Plan a dated meal
              </button>

              <div className="section-toolbar">
                <h2>
                  On the menu{" "}
                  <span className="count">{upcoming.length} upcoming</span>
                </h2>
                <span className="muted">
                  Plan your pace. Leave room for leftovers.
                </span>
              </div>
              {!meals.length ? (
                <div className="empty">
                  <CalendarDays size={32} />
                  <h2>A delicious blank slate.</h2>
                  <p>Add a recipe to a date to start your meal plan.</p>
                </div>
              ) : (
                <div className="meal-list">
                  {meals.map((m) => (
                    <article
                      className={`meal-row ${m.date < today() ? "past" : ""}`}
                      key={m.id}
                    >
                      <div className="date-badge">
                        <strong>
                          {new Date(m.date + "T12:00:00").toLocaleDateString(
                            undefined,
                            { day: "numeric" },
                          )}
                        </strong>
                        <span>
                          {new Date(m.date + "T12:00:00").toLocaleDateString(
                            undefined,
                            { month: "short" },
                          )}
                        </span>
                      </div>
                      <div className="meal-title">
                        <span className="eyebrow">
                          {m.date} · {m.slot}
                        </span>
                        <h3>{m.title}</h3>
                        <span className="muted">
                          Serves {quantity(m.servings)}
                        </span>
                      </div>
                      <button
                        className="secondary"
                        aria-label="Add ingredients"
                        disabled={busy}
                        onClick={() => {
                          const r = recipes.find((r) => r.id === m.recipe_id);
                          if (r)
                            setDialog({
                              kind: "shop",
                              recipe: { ...r, servings: m.servings },
                            });
                        }}
                      >
                        <ShoppingBasket size={17} />
                        <span>Add ingredients</span>
                      </button>
                      <button
                        className="icon-button"
                        aria-label={`Remove ${m.title} from plan`}
                        disabled={busy}
                        onClick={() =>
                          void act(async () => {
                            await api(`/meals/${m.id}`, "DELETE");
                            await refresh();
                          })
                        }
                      >
                        <X size={19} />
                      </button>
                    </article>
                  ))}
                </div>
              )}
            </details>
          </>
        ) : (
          <>
            <div className="list-tabs" aria-label="Shopping lists">
              {lists.map((l) => (
                <button
                  className={
                    selectedList === l.id ? "list-tab selected" : "list-tab"
                  }
                  key={l.id}
                  onClick={() => setSelectedList(l.id)}
                >
                  <ShoppingBasket size={16} />
                  {l.name}
                </button>
              ))}
            </div>
            {!lists.length ? (
              <div className="empty">
                <ShoppingBasket size={34} />
                <h2>Less remembering. More cooking.</h2>
                <p>
                  Create a shopping list, then add ingredients from any recipe.
                </p>
              </div>
            ) : (
              detail && (
                <section className="shopping-panel">
                  <div className="section-toolbar">
                    <div>
                      <h2>{detail.name}</h2>
                      <p className="muted">
                        {detail.items.filter((i) => !i.checked).length} left to
                        pick up · {detail.items.filter((i) => i.checked).length}{" "}
                        in the basket
                      </p>
                    </div>
                    <button
                      className="icon-button"
                      aria-label="Delete shopping list"
                      disabled={busy}
                      onClick={() => {
                        if (confirm(`Delete “${detail.name}” and its items?`))
                          void act(async () => {
                            await api(`/lists/${detail.id}`, "DELETE");
                            setDetail(null);
                            await refresh();
                          });
                      }}
                    >
                      <Trash2 size={18} />
                    </button>
                  </div>
                  <form
                    className="manual-item"
                    onSubmit={(e) =>
                      onSubmit(e, async () => {
                        const result = await api<ListDetail>(
                          `/lists/${detail.id}/items`,
                          "POST",
                          manual,
                        );
                        setDetail(result);
                        setManual(blankIngredient());
                      })
                    }
                  >
                    <input
                      required
                      maxLength={200}
                      aria-label="Item name"
                      placeholder="Add something to the list…"
                      value={manual.name}
                      onChange={(e) =>
                        setManual({ ...manual, name: e.target.value })
                      }
                    />
                    <input
                      className="qty-input"
                      aria-label="Item quantity"
                      type="number"
                      min="0.001"
                      max="1000000"
                      step="any"
                      required
                      value={manual.quantity}
                      onChange={(e) =>
                        setManual({
                          ...manual,
                          quantity: Number(e.target.value),
                        })
                      }
                    />
                    <input
                      className="unit-input"
                      aria-label="Item unit"
                      placeholder="unit"
                      maxLength={80}
                      value={manual.unit}
                      onChange={(e) =>
                        setManual({ ...manual, unit: e.target.value })
                      }
                    />
                    <button
                      className="primary"
                      disabled={busy}
                      aria-label="Add shopping item"
                    >
                      <Plus size={18} />
                    </button>
                  </form>
                  {!detail.items.length && (
                    <p className="empty-list">
                      Your list is ready for something good. Add an item above
                      or ingredients from your recipe book.
                    </p>
                  )}
                  <div className="shopping-items">
                    {detail.items.map((i) => (
                      <div
                        key={i.id}
                        className={`shopping-item ${i.checked ? "checked" : ""}`}
                      >
                        <label>
                          <input
                            type="checkbox"
                            checked={i.checked}
                            disabled={busy}
                            onChange={() =>
                              void act(async () => {
                                await api(
                                  `/lists/${detail.id}/items/${i.id}`,
                                  "PATCH",
                                  { checked: !i.checked },
                                );
                                await reloadList(detail.id);
                              })
                            }
                          />
                          <span>{i.name}</span>
                        </label>
                        <span className="item-quantity">
                          {quantity(i.quantity)} {i.unit}
                        </span>
                        <button
                          className="icon-button"
                          aria-label={`Remove ${i.name}`}
                          disabled={busy}
                          onClick={() =>
                            void act(async () => {
                              await api(
                                `/lists/${detail.id}/items/${i.id}`,
                                "DELETE",
                              );
                              await reloadList(detail.id);
                            })
                          }
                        >
                          <X size={17} />
                        </button>
                      </div>
                    ))}
                  </div>
                  <div className="list-footer">
                    <Leaf size={16} /> Just what you need. Nothing to inventory.
                  </div>
                </section>
              )
            )}
          </>
        )}
        <footer className="page-footer">
          <span>Made for everyday cooking.</span>
          <span>Keep it simple. Make it yours.</span>
        </footer>
      </main>
      {dialog && (
        <Modal
          title={
            dialog.kind === "edit"
              ? dialog.recipe
                ? "Edit recipe"
                : "A new favorite"
              : dialog.kind === "recipe"
                ? dialog.recipe.title
                : dialog.kind === "cook"
                  ? dialog.cook
                    ? "Edit cook"
                    : "Log a cook"
                  : dialog.kind === "wish"
                    ? "Add to a weekly wishlist"
                    : dialog.kind === "week-shop"
                      ? "Shop this week’s wishlist"
                      : dialog.kind === "plan"
                        ? "Put it on the menu"
                        : dialog.kind === "shop"
                          ? "Add ingredients to a list"
                          : dialog.kind === "list"
                            ? "Make a little list"
                            : "Connection settings"
          }
          close={close}
        >
          {error && (
            <div className="alert" role="alert">
              {error}
            </div>
          )}
          {dialog.kind === "edit" ? (
            <RecipeForm
              recipe={dialog.recipe}
              busy={busy}
              save={(data, photo) =>
                void act(async () => {
                  const saved = await api<Recipe>(
                    `/recipes${dialog.recipe ? `/${dialog.recipe.id}` : ""}`,
                    dialog.recipe ? "PUT" : "POST",
                    data,
                  );
                  setDialog({ kind: "edit", recipe: saved });
                  if (photo === null) await photoRequest(saved.id, "DELETE");
                  else if (photo) await photoRequest(saved.id, "PUT", photo);
                  await refresh();
                  setDialog(null);
                }, "Recipe saved.")
              }
            />
          ) : dialog.kind === "recipe" ? (
            <div className="recipe-detail">
              {!!dialog.recipe.photo_version && (
                <RecipePhoto recipe={dialog.recipe} className="detail-photo" />
              )}
              <div className="detail-meta">
                <span>Serves {quantity(dialog.recipe.servings)}</span>
                <span>{dialog.recipe.ingredients.length} ingredients</span>
              </div>
              <section className="recipe-ratings">
                <label>
                  Recipe rating
                  <select
                    aria-label="Recipe rating"
                    value={dialog.recipe.rating ?? ""}
                    disabled={busy}
                    onChange={(e) => {
                      const rating = e.target.value
                        ? Number(e.target.value)
                        : null;
                      void act(async () => {
                        const updated = await api<Recipe>(
                          `/recipes/${dialog.recipe.id}/rating`,
                          "PUT",
                          { rating },
                        );
                        setDialog({ kind: "recipe", recipe: updated });
                        await refresh();
                      }, "Recipe rating saved.");
                    }}
                  >
                    <option value="">Not rated</option>
                    {[1, 2, 3, 4, 5].map((n) => (
                      <option value={n} key={n}>
                        {n}/5
                      </option>
                    ))}
                  </select>
                </label>
                <CookSummary recipe={dialog.recipe} />
              </section>
              <h3>What you’ll need</h3>
              <ul className="ingredient-list">
                {dialog.recipe.ingredients.map((i, n) => (
                  <li key={n}>
                    <span>{i.name}</span>
                    <strong>
                      {quantity(i.quantity)} {i.unit}
                    </strong>
                  </li>
                ))}
              </ul>
              <h3>Let’s cook</h3>
              <p className="instructions">
                {dialog.recipe.instructions || "No instructions added yet."}
              </p>
              {dialog.recipe.source_url && (
                <a
                  href={dialog.recipe.source_url}
                  target="_blank"
                  rel="noreferrer"
                  className="text-link"
                >
                  Original recipe <ArrowUpRight size={16} />
                </a>
              )}
              <div className="dialog-actions">
                <button
                  className="secondary"
                  onClick={() =>
                    setDialog({ kind: "wish", recipe: dialog.recipe })
                  }
                >
                  <Leaf size={17} />
                  Add to week
                </button>
                <button
                  className="secondary"
                  onClick={() =>
                    setDialog({ kind: "plan", recipe: dialog.recipe })
                  }
                >
                  <CalendarDays size={17} />
                  Plan this meal
                </button>
                <button
                  className="primary"
                  onClick={() =>
                    setDialog({ kind: "shop", recipe: dialog.recipe })
                  }
                >
                  <ShoppingBasket size={17} />
                  Add ingredients
                </button>
              </div>
              <section className="cook-section">
                <div className="section-toolbar">
                  <h2>Cook history</h2>
                  <button
                    className="primary"
                    disabled={busy}
                    onClick={() =>
                      setDialog({ kind: "cook", recipe: dialog.recipe })
                    }
                  >
                    <Plus size={17} />
                    Log a cook
                  </button>
                </div>
                <CookHistory
                  recipe={dialog.recipe}
                  busy={busy}
                  edit={(cook) =>
                    setDialog({ kind: "cook", recipe: dialog.recipe, cook })
                  }
                  remove={(cook) => {
                    if (confirm(`Delete the cook logged on ${cook.cooked_on}?`))
                      void act(async () => {
                        await api(
                          `/recipes/${dialog.recipe.id}/cooks/${cook.id}`,
                          "DELETE",
                        );
                        setDialog({
                          kind: "recipe",
                          recipe: {
                            ...dialog.recipe,
                            cook_count: Math.max(
                              0,
                              (dialog.recipe.cook_count || 0) - 1,
                            ),
                          },
                        });
                        const updated = await api<Recipe>(
                          `/recipes/${dialog.recipe.id}`,
                        );
                        setDialog({ kind: "recipe", recipe: updated });
                        await refresh();
                      }, "Cook removed.");
                  }}
                />
              </section>
              <div className="detail-bottom">
                <button
                  className="text-link"
                  onClick={() =>
                    setDialog({ kind: "edit", recipe: dialog.recipe })
                  }
                >
                  <Pencil size={15} />
                  Edit recipe
                </button>
                <button
                  className="text-link danger"
                  disabled={busy}
                  onClick={() => {
                    if (
                      confirm(
                        "Delete this recipe and its meal-plan entries? Shopping items will stay.",
                      )
                    )
                      void act(async () => {
                        await api(`/recipes/${dialog.recipe.id}`, "DELETE");
                        await refresh();
                        setDialog(null);
                      });
                  }}
                >
                  <Trash2 size={15} />
                  Delete
                </button>
              </div>
            </div>
          ) : dialog.kind === "cook" ? (
            <CookForm
              cook={dialog.cook}
              busy={busy}
              submit={(data) =>
                void act(async () => {
                  const saved = await api<Cook>(
                    `/recipes/${dialog.recipe.id}/cooks${dialog.cook ? `/${dialog.cook.id}` : ""}`,
                    dialog.cook ? "PUT" : "POST",
                    data,
                  );
                  setDialog({
                    kind: "cook",
                    recipe: dialog.recipe,
                    cook: saved,
                  });
                  const updated = await api<Recipe>(
                    `/recipes/${dialog.recipe.id}`,
                  );
                  await refresh();
                  setDialog({ kind: "recipe", recipe: updated });
                }, "Cook saved.")
              }
            />
          ) : dialog.kind === "wish" ? (
            <WishForm
              recipes={recipes}
              initial={dialog.recipe}
              week={week}
              busy={busy}
              submit={(data) =>
                void act(async () => {
                  await api("/wishes", "POST", data);
                  const targetWeek = monday(data.week);
                  setWeek(targetWeek);
                  await refresh(targetWeek);
                  setDialog(null);
                  setPage("plan");
                }, "Added to your weekly wishlist.")
              }
            />
          ) : dialog.kind === "week-shop" ? (
            <WeekShoppingForm
              lists={lists}
              busy={busy}
              submit={(id, name) =>
                void act(async () => {
                  const target =
                    id ||
                    (await api<ShoppingList>("/lists", "POST", { name })).id;
                  await api(`/lists/${target}/weeks/${week}`, "POST", {});
                  await refresh();
                  setSelectedList(target);
                  await reloadList(target);
                  setDialog(null);
                  setPage("shopping");
                }, "This week’s uncooked meals added to your list.")
              }
            />
          ) : dialog.kind === "plan" ? (
            <PlanForm
              recipes={recipes}
              initial={dialog.recipe}
              busy={busy}
              submit={(data) =>
                void act(async () => {
                  await api("/meals", "POST", data);
                  await refresh();
                  setDialog(null);
                  setPage("plan");
                }, "On the menu.")
              }
            />
          ) : dialog.kind === "shop" ? (
            <ShoppingForm
              lists={lists}
              recipe={dialog.recipe}
              busy={busy}
              submit={(id, servings, name) =>
                void act(async () => {
                  const target =
                    id ||
                    (await api<ShoppingList>("/lists", "POST", { name })).id;
                  await api(
                    `/lists/${target}/recipes/${dialog.recipe.id}`,
                    "POST",
                    { servings },
                  );
                  await refresh();
                  setSelectedList(target);
                  await reloadList(target);
                  setDialog(null);
                  setPage("shopping");
                }, "Ingredients added to your list.")
              }
            />
          ) : dialog.kind === "list" ? (
            <form
              onSubmit={(e) =>
                onSubmit(e, async () => {
                  const data = new FormData(e.target as HTMLFormElement);
                  const l = await api<ShoppingList>("/lists", "POST", {
                    name: data.get("name"),
                  });
                  await refresh();
                  setSelectedList(l.id);
                  setDialog(null);
                })
              }
            >
              <label>
                List name
                <input
                  name="name"
                  placeholder="This week’s groceries"
                  required
                  maxLength={200}
                  autoFocus
                />
              </label>
              <div className="dialog-actions">
                <button className="primary" disabled={busy}>
                  Create list <ArrowRight size={17} />
                </button>
              </div>
            </form>
          ) : (
            <SettingsForm
              busy={busy}
              save={(token) =>
                void act(async () => {
                  sessionStorage.setItem("recetaco-token", token);
                  await refresh();
                  if (selectedList) await reloadList(selectedList);
                  setDialog(null);
                }, "Connected.")
              }
              exportData={() =>
                void act(async () => {
                  const data = await api("/export");
                  const url = URL.createObjectURL(
                    new Blob([JSON.stringify(data, null, 2)], {
                      type: "application/json",
                    }),
                  );
                  const a = document.createElement("a");
                  a.href = url;
                  a.download = `recetaco-${today()}.json`;
                  a.click();
                  URL.revokeObjectURL(url);
                })
              }
            />
          )}
        </Modal>
      )}
    </div>
  );
}
function RecipeForm({
  recipe,
  busy,
  save,
}: {
  recipe?: Recipe;
  busy: boolean;
  save: (data: RecipeInput, photo?: Blob | null) => void;
}) {
  const [data, setData] = useState<RecipeInput>(
    recipe || {
      title: "",
      servings: 2,
      instructions: "",
      source_url: "",
      ingredients: [blankIngredient()],
    },
  );
  const [photo, setPhoto] = useState<Blob | null | undefined>();
  const [photoError, setPhotoError] = useState("");
  const [preparing, setPreparing] = useState(false);
  const [preview, setPreview] = useState("");
  useEffect(() => {
    if (!photo) {
      setPreview("");
      return;
    }
    const url = URL.createObjectURL(photo);
    setPreview(url);
    return () => URL.revokeObjectURL(url);
  }, [photo]);
  const changeIngredient = (n: number, patch: Partial<Ingredient>) =>
    setData({
      ...data,
      ingredients: data.ingredients.map((i, index) =>
        index === n ? { ...i, ...patch } : i,
      ),
    });
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        save(data, photo);
      }}
    >
      <label>
        Recipe name
        <input
          required
          maxLength={200}
          autoFocus
          placeholder="Something you love to cook"
          value={data.title}
          onChange={(e) => setData({ ...data, title: e.target.value })}
        />
      </label>
      <div className="form-row">
        <label>
          Servings
          <input
            type="number"
            min="0.001"
            max="1000000"
            step="any"
            required
            value={data.servings}
            onChange={(e) =>
              setData({ ...data, servings: Number(e.target.value) })
            }
          />
        </label>
        <label>
          Source link <span className="muted">(optional)</span>
          <input
            type="url"
            maxLength={2000}
            placeholder="https://…"
            value={data.source_url}
            onChange={(e) => setData({ ...data, source_url: e.target.value })}
          />
        </label>
      </div>
      <label>
        Recipe photo
        <input
          type="file"
          accept="image/jpeg,image/png,image/webp"
          disabled={busy || preparing}
          onChange={(e) => {
            const file = e.target.files?.[0];
            if (!file) return;
            setPreparing(true);
            setPhotoError("");
            resizePhoto(file)
              .then(setPhoto)
              .catch((e) => setPhotoError(e.message))
              .finally(() => setPreparing(false));
          }}
        />
      </label>
      {photoError && (
        <p role="alert" className="alert">
          {photoError}
        </p>
      )}
      {preview ? (
        <img
          src={preview}
          alt="Selected recipe photo"
          className="photo-preview"
        />
      ) : photo !== null && recipe?.photo_version ? (
        <RecipePhoto recipe={recipe} className="photo-preview" />
      ) : null}
      {(photo || (photo !== null && recipe?.photo_version)) && (
        <button
          type="button"
          className="text-link danger"
          onClick={() => setPhoto(null)}
        >
          Remove photo
        </button>
      )}
      <p className="form-hint">
        JPEG, PNG, or WebP. Photos are resized to 1200 pixels and stored with
        your recipe.
      </p>
      <h3>Ingredients</h3>
      <div className="ingredient-labels">
        <span>Ingredient</span>
        <span>Quantity</span>
        <span>Unit</span>
      </div>
      {data.ingredients.map((i, n) => (
        <div className="ingredient-editor" key={n}>
          <input
            aria-label={`Ingredient ${n + 1} name`}
            required
            maxLength={200}
            placeholder="e.g. Chickpeas"
            value={i.name}
            onChange={(e) => changeIngredient(n, { name: e.target.value })}
          />
          <input
            aria-label={`Ingredient ${n + 1} quantity`}
            type="number"
            min="0.001"
            max="1000000"
            step="any"
            required
            value={i.quantity}
            onChange={(e) =>
              changeIngredient(n, { quantity: Number(e.target.value) })
            }
          />
          <input
            aria-label={`Ingredient ${n + 1} unit`}
            maxLength={80}
            placeholder="g, can…"
            value={i.unit}
            onChange={(e) => changeIngredient(n, { unit: e.target.value })}
          />
          <button
            type="button"
            className="icon-button"
            aria-label={`Remove ingredient ${n + 1}`}
            disabled={data.ingredients.length === 1}
            onClick={() =>
              setData({
                ...data,
                ingredients: data.ingredients.filter((_, k) => k !== n),
              })
            }
          >
            <Minus size={17} />
          </button>
        </div>
      ))}
      <button
        type="button"
        className="text-link"
        disabled={data.ingredients.length >= 200}
        onClick={() =>
          setData({
            ...data,
            ingredients: [...data.ingredients, blankIngredient()],
          })
        }
      >
        <Plus size={16} />
        Add ingredient
      </button>
      <label>
        Instructions
        <textarea
          rows={5}
          placeholder="One delicious step at a time…"
          maxLength={100000}
          value={data.instructions}
          onChange={(e) => setData({ ...data, instructions: e.target.value })}
        />
      </label>
      <div className="dialog-actions">
        <button className="primary" disabled={busy || preparing}>
          {busy || preparing ? "Preparing…" : "Save recipe"}
          <Check size={17} />
        </button>
      </div>
    </form>
  );
}
function PlanForm({
  recipes,
  initial,
  busy,
  submit,
}: {
  recipes: Recipe[];
  initial?: Recipe;
  busy: boolean;
  submit: (data: unknown) => void;
}) {
  const [id, setId] = useState(initial?.id || recipes[0]?.id || 0);
  const [servings, setServings] = useState(
    initial?.servings || recipes[0]?.servings || 2,
  );
  return !recipes.length ? (
    <p>Add a recipe to your book first, then come back to plan a meal.</p>
  ) : (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        const f = new FormData(e.currentTarget);
        submit({
          recipe_id: id,
          date: f.get("date"),
          slot: f.get("slot"),
          servings,
        });
      }}
    >
      <label>
        Recipe
        <select
          value={id}
          onChange={(e) => {
            const r = recipes.find((r) => r.id === Number(e.target.value))!;
            setId(r.id);
            setServings(r.servings);
          }}
        >
          {recipes.map((r) => (
            <option value={r.id} key={r.id}>
              {r.title}
            </option>
          ))}
        </select>
      </label>
      <div className="form-row">
        <label>
          Date
          <input name="date" type="date" required defaultValue={today()} />
        </label>
        <label>
          Meal
          <select name="slot" defaultValue="dinner">
            {["breakfast", "lunch", "dinner", "snack"].map((s) => (
              <option key={s}>{s}</option>
            ))}
          </select>
        </label>
      </div>
      <label>
        Servings
        <input
          type="number"
          required
          min="0.001"
          max="1000000"
          step="any"
          value={servings}
          onChange={(e) => setServings(Number(e.target.value))}
        />
      </label>
      <div className="dialog-actions">
        <button className="primary" disabled={busy}>
          Plan meal <CalendarDays size={17} />
        </button>
      </div>
    </form>
  );
}
function ShoppingForm({
  lists,
  recipe,
  busy,
  submit,
}: {
  lists: ShoppingList[];
  recipe: Recipe;
  busy: boolean;
  submit: (id: number, servings: number, name: string) => void;
}) {
  const [id, setId] = useState(lists[0]?.id || 0);
  const [name, setName] = useState("This week");
  const [servings, setServings] = useState(recipe.servings);
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        submit(id, servings, name);
      }}
    >
      <p className="muted">
        From <strong>{recipe.title}</strong>. Ingredients scale to your chosen
        servings.
      </p>
      <label>
        Shopping list
        <select value={id} onChange={(e) => setId(Number(e.target.value))}>
          {lists.map((l) => (
            <option key={l.id} value={l.id}>
              {l.name}
            </option>
          ))}
          <option value={0}>Create a new list…</option>
        </select>
      </label>
      {id === 0 && (
        <label>
          New list name
          <input
            required
            value={name}
            maxLength={200}
            onChange={(e) => setName(e.target.value)}
          />
        </label>
      )}
      <label>
        Servings
        <input
          type="number"
          required
          min="0.001"
          max="1000000"
          step="any"
          value={servings}
          onChange={(e) => setServings(Number(e.target.value))}
        />
      </label>
      <p className="form-hint">
        Matching unchecked ingredients with the same unit will be combined. Each
        addition adds quantities again.
      </p>
      <div className="dialog-actions">
        <button className="primary" disabled={busy}>
          Add ingredients <ShoppingBasket size={17} />
        </button>
      </div>
    </form>
  );
}
function SettingsForm({
  busy,
  save,
  exportData,
}: {
  busy: boolean;
  save: (token: string) => void;
  exportData: () => void;
}) {
  const [token, setToken] = useState(
    sessionStorage.getItem("recetaco-token") || "",
  );
  const [mode, setMode] = useState("none");
  useEffect(() => {
    authConfig().then((c) => setMode(c.mode));
  }, []);
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        save(token.trim());
      }}
    >
      <p className="muted">
        This browser connects to the server hosting Recetaco.
      </p>
      {mode === "oidc" ? (
        <p className="form-hint">
          You’re signed in through Pocket ID. Everyone permitted to use this
          Recetaco application shares the same kitchen.
        </p>
      ) : (
        <>
          <label>
            Development API token
            <input
              type="password"
              autoComplete="off"
              value={token}
              onChange={(e) => setToken(e.target.value)}
              placeholder="Only needed for legacy token mode"
            />
          </label>
          <p className="form-hint">
            This token is used only by development servers without Pocket ID.
          </p>
        </>
      )}
      <div className="dialog-actions">
        <button
          type="button"
          className="secondary"
          disabled={busy}
          onClick={exportData}
        >
          <Download size={17} />
          Export JSON
        </button>
        {mode === "oidc" ? (
          <button
            type="button"
            className="primary"
            disabled={busy}
            onClick={() => void signOut().catch(() => location.reload())}
          >
            Sign out
          </button>
        ) : (
          <button className="primary" disabled={busy}>
            Connect <ArrowRight size={17} />
          </button>
        )}
      </div>
    </form>
  );
}

function WishForm({
  recipes,
  initial,
  week,
  busy,
  submit,
}: {
  recipes: Recipe[];
  initial?: Recipe;
  week: string;
  busy: boolean;
  submit: (data: { recipe_id: number; week: string; servings: number }) => void;
}) {
  const [id, setId] = useState(initial?.id || recipes[0]?.id || 0);
  const [servings, setServings] = useState(
    initial?.servings || recipes[0]?.servings || 2,
  );
  const [selectedWeek, setSelectedWeek] = useState(week);
  return !recipes.length ? (
    <p>Add a recipe to your book first, then add it to a weekly wishlist.</p>
  ) : (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        submit({ recipe_id: id, week: selectedWeek, servings });
      }}
    >
      <p className="muted">
        Something you’d like to cook this week. No day or meal slot to decide.
      </p>
      <label>
        Recipe
        <select
          value={id}
          onChange={(e) => {
            const r = recipes.find((r) => r.id === Number(e.target.value))!;
            setId(r.id);
            setServings(r.servings);
          }}
        >
          {recipes.map((r) => (
            <option key={r.id} value={r.id}>
              {r.title}
            </option>
          ))}
        </select>
      </label>
      <label>
        Week of
        <input
          type="date"
          required
          value={selectedWeek}
          onChange={(e) => {
            if (e.target.value) setSelectedWeek(monday(e.target.value));
          }}
        />
      </label>
      <label>
        Servings
        <input
          type="number"
          min="0.001"
          max="1000000"
          step="any"
          required
          value={servings}
          onChange={(e) => setServings(Number(e.target.value))}
        />
      </label>
      <div className="dialog-actions">
        <button className="primary" disabled={busy}>
          Add to wishlist <Leaf size={17} />
        </button>
      </div>
    </form>
  );
}
function WeekShoppingForm({
  lists,
  busy,
  submit,
}: {
  lists: ShoppingList[];
  busy: boolean;
  submit: (id: number, name: string) => void;
}) {
  const [id, setId] = useState(lists[0]?.id || 0);
  const [name, setName] = useState("This week");
  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        submit(id, name);
      }}
    >
      <p className="muted">
        Add ingredients from every uncooked meal in this week’s wishlist, scaled
        to its servings. Cooked meals are skipped.
      </p>
      <label>
        Shopping list
        <select value={id} onChange={(e) => setId(Number(e.target.value))}>
          {lists.map((l) => (
            <option key={l.id} value={l.id}>
              {l.name}
            </option>
          ))}
          <option value={0}>Create a new list…</option>
        </select>
      </label>
      {id === 0 && (
        <label>
          New list name
          <input
            required
            maxLength={200}
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </label>
      )}
      <p className="form-hint">
        Each addition adds quantities again. Review the list before repeating.
      </p>
      <div className="dialog-actions">
        <button className="primary" disabled={busy}>
          Add week’s ingredients <ShoppingBasket size={17} />
        </button>
      </div>
    </form>
  );
}

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <AuthGate>
      <App />
    </AuthGate>
  </React.StrictMode>,
);
