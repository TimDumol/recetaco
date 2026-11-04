import { useState } from "react";
import {
  quantity,
  type Recipe,
  type RecipeInput,
  type RecipeStep,
} from "./api";

export function CookingMethod({ recipe }: { recipe: Recipe }) {
  const [view, setView] = useState("flow");
  const [done, setDone] = useState<number[]>([]);
  const steps = recipe.steps || [];
  const levels: number[] = [];
  steps.forEach((step, n) => {
    levels[n] = step.after.length
      ? 1 + Math.max(...step.after.map((i) => levels[i] ?? 0))
      : 0;
  });
  return (
    <section className="cooking-method">
      {!!steps.length && (
        <div className="method-switch" role="group" aria-label="Cooking view">
          {["method", "flow"].map((v) => (
            <button
              key={v}
              className="secondary"
              aria-pressed={view === v}
              onClick={() => setView(v)}
            >
              {v === "method" ? "Method" : "Flow"}
            </button>
          ))}
        </div>
      )}
      <details className="amount-reference">
        <summary>Ingredient amounts</summary>
        <ul>
          {recipe.ingredients.map((i, n) => (
            <li key={n}>
              {quantity(i.quantity)} {i.unit} {i.name}
            </li>
          ))}
        </ul>
      </details>
      {!steps.length ? (
        <>
          <p className="form-hint">
            Edit this recipe to link ingredient amounts to each cooking step.
          </p>
          <p className="instructions">
            {recipe.instructions || "No instructions added yet."}
          </p>
        </>
      ) : (
        <>
          {recipe.instructions && (
            <details>
              <summary>Full recipe, tips and alternatives</summary>
              <p className="instructions">{recipe.instructions}</p>
            </details>
          )}
          <p className="form-hint" aria-live="polite">
            {view === "flow"
              ? "Follow the branches from top to bottom. Steps in the same group can run in parallel; open a step for full directions."
              : "Read the full directions in recipe order, with ingredient amounts beside each step."}
          </p>
          <div className={view === "flow" ? "step-flow" : "step-method"}>
            {(view === "flow"
              ? Array.from({ length: Math.max(...levels) + 1 }, (_, level) =>
                  steps.map((_, n) => n).filter((n) => levels[n] === level),
                )
              : [steps.map((_, n) => n)]
            ).map((group, level) => (
              <div
                className={view === "flow" ? "flow-stage" : "method-stage"}
                key={level}
              >
                {view === "flow" && (
                  <div className="flow-stage-heading">
                    {level > 0 && <span aria-hidden="true">↓ </span>}
                    {level === 0
                      ? "Start here"
                      : `Continue · stage ${level + 1}`}
                    {group.length > 1 && (
                      <strong> · {group.length} parallel branches</strong>
                    )}
                  </div>
                )}
                {group.map((n) => {
                  const step = steps[n];
                  return (
                    <article className="cooking-step" key={n}>
                      {view === "flow" && (
                        <div className="step-dependencies">
                          {step.after.length
                            ? `After ${step.after.map((i) => `step ${i + 1}`).join(" + ")} ↓`
                            : "Can start independently"}
                        </div>
                      )}
                      <label className="step-check">
                        <input
                          type="checkbox"
                          checked={done.includes(n)}
                          onChange={(e) =>
                            setDone(
                              e.target.checked
                                ? [...done, n]
                                : done.filter((i) => i !== n),
                            )
                          }
                        />
                        <strong>
                          Step {n + 1}
                          {done.includes(n) ? " · Done" : ""}
                        </strong>
                      </label>
                      {view === "flow" ? (
                        <>
                          <p className="flow-step-summary">
                            {step.text.split(/(?<=[.!?])\s/)[0]}
                          </p>
                          <details className="flow-directions">
                            <summary>Full directions for step {n + 1}</summary>
                            <p className="instructions">{step.text}</p>
                          </details>
                        </>
                      ) : (
                        <p className="instructions">{step.text}</p>
                      )}
                      {!!step.ingredients.length && (
                        <ul className="step-amounts">
                          {step.ingredients.map((use) => {
                            const i = recipe.ingredients[use.ingredient];
                            return (
                              i && (
                                <li key={use.ingredient}>
                                  <strong>
                                    {quantity(use.quantity ?? i.quantity)}{" "}
                                    {i.unit}
                                  </strong>{" "}
                                  {i.name}
                                </li>
                              )
                            );
                          })}
                        </ul>
                      )}
                    </article>
                  );
                })}
              </div>
            ))}
          </div>
        </>
      )}
    </section>
  );
}

export function StepEditor({
  data,
  change,
}: {
  data: RecipeInput;
  change: (data: RecipeInput) => void;
}) {
  const steps = data.steps || [];
  const update = (n: number, patch: Partial<RecipeStep>) =>
    change({
      ...data,
      steps: steps.map((s, k) => (k === n ? { ...s, ...patch } : s)),
    });
  return (
    <section className="step-editor">
      <h3>Cooking steps with amounts</h3>
      <p className="form-hint">
        Link the ingredients used in each step. Leave an amount blank to use the
        recipe amount; enter a smaller amount for divided ingredients. Choose
        earlier steps that must finish first.
      </p>
      {!steps.length && data.instructions.trim() && (
        <button
          type="button"
          className="secondary"
          onClick={() =>
            change({
              ...data,
              steps: data.instructions
                .split(/\n\s*\n|\n/)
                .map((s) => s.trim())
                .filter(Boolean)
                .map((text, n) => ({
                  text,
                  ingredients: [],
                  after: n ? [n - 1] : [],
                })),
            })
          }
        >
          Make steps from instructions
        </button>
      )}
      {steps.map((step, n) => (
        <fieldset className="cooking-step" key={n}>
          <legend>Step {n + 1}</legend>
          <label>
            Step {n + 1} instruction
            <textarea
              required
              maxLength={10000}
              value={step.text}
              onChange={(e) => update(n, { text: e.target.value })}
            />
          </label>
          <p>Ingredients used</p>
          {data.ingredients.map((i, k) => {
            const use = step.ingredients.find((u) => u.ingredient === k);
            return (
              <div className="step-ingredient" key={k}>
                <label>
                  <input
                    type="checkbox"
                    checked={!!use}
                    onChange={(e) =>
                      update(n, {
                        ingredients: e.target.checked
                          ? [...step.ingredients, { ingredient: k }]
                          : step.ingredients.filter((u) => u.ingredient !== k),
                      })
                    }
                  />
                  {i.name || `Ingredient ${k + 1}`} · {quantity(i.quantity)}{" "}
                  {i.unit}
                </label>
                {use && (
                  <label>
                    Amount in step {n + 1}: {i.name}
                    <input
                      type="number"
                      min="0.001"
                      max={i.quantity}
                      step="any"
                      placeholder={String(i.quantity)}
                      value={use.quantity ?? ""}
                      onChange={(e) =>
                        update(n, {
                          ingredients: step.ingredients.map((u) =>
                            u.ingredient === k
                              ? {
                                  ...u,
                                  quantity: e.target.value
                                    ? Number(e.target.value)
                                    : null,
                                }
                              : u,
                          ),
                        })
                      }
                    />
                  </label>
                )}
              </div>
            );
          })}
          {n > 0 && (
            <>
              <p>Start after</p>
              {steps.slice(0, n).map((_, k) => (
                <label className="step-check" key={k}>
                  <input
                    type="checkbox"
                    checked={step.after.includes(k)}
                    onChange={(e) =>
                      update(n, {
                        after: e.target.checked
                          ? [...step.after, k]
                          : step.after.filter((i) => i !== k),
                      })
                    }
                  />
                  Step {k + 1}
                </label>
              ))}
            </>
          )}
          <button
            type="button"
            className="text-link danger"
            onClick={() =>
              change({
                ...data,
                steps: steps
                  .filter((_, k) => k !== n)
                  .map((s) => ({
                    ...s,
                    after: s.after
                      .filter((i) => i !== n)
                      .map((i) => (i > n ? i - 1 : i)),
                  })),
              })
            }
          >
            Remove step {n + 1}
          </button>
        </fieldset>
      ))}
      <button
        type="button"
        className="text-link"
        disabled={steps.length >= 200}
        onClick={() =>
          change({
            ...data,
            steps: [
              ...steps,
              {
                text: "",
                ingredients: [],
                after: steps.length ? [steps.length - 1] : [],
              },
            ],
          })
        }
      >
        Add cooking step
      </button>
    </section>
  );
}
