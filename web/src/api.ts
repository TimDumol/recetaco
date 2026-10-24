import { accessToken } from "./auth";
export interface Ingredient {
  name: string;
  quantity: number;
  unit: string;
}
export interface RecipeInput {
  title: string;
  servings: number;
  instructions: string;
  source_url: string;
  ingredients: Ingredient[];
}
export interface Recipe extends RecipeInput {
  id: number;
  photo_version?: number;
  rating?: number | null;
  cook_count?: number;
  average_cook_rating?: number | null;
  last_cooked?: string | null;
}
export interface Meal {
  id: number;
  recipe_id: number;
  title: string;
  date: string;
  slot: string;
  servings: number;
}
export interface ShoppingList {
  id: number;
  name: string;
}
export interface Item extends Ingredient {
  id: number;
  list_id: number;
  checked: boolean;
}
export interface ListDetail extends ShoppingList {
  items: Item[];
}
export async function api<T>(
  path: string,
  method = "GET",
  data?: unknown,
): Promise<T> {
  const token = await accessToken();
  const response = await fetch(`/api${path}`, {
    method,
    headers: {
      ...(data !== undefined ? { "Content-Type": "application/json" } : {}),
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: data !== undefined ? JSON.stringify(data) : undefined,
  });
  if (!response.ok) {
    const body = await response.text();
    let message = body;
    try {
      message = JSON.parse(body).error || body;
    } catch {
      /* validation may be plain text */
    }
    throw new Error(
      response.status === 401
        ? "Your session expired. Sign in again, or check Connection settings."
        : message || `Request failed (${response.status})`,
    );
  }
  return response.status === 204 ? (undefined as T) : response.json();
}
export function today(): string {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}
export const quantity = (n: number) => Number(n.toFixed(3)).toLocaleString();

export interface Wish {
  id: number;
  recipe_id: number;
  title: string;
  week: string;
  servings: number;
  cooked: boolean;
}
export function monday(value = today()): string {
  const date = new Date(value + "T12:00:00");
  date.setDate(date.getDate() - ((date.getDay() + 6) % 7));
  return localDate(date);
}
function localDate(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}
export function shiftWeek(week: string, days: number): string {
  const d = new Date(week + "T12:00:00");
  d.setDate(d.getDate() + days);
  return localDate(d);
}
