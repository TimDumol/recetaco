import { useEffect, useState } from "react";
import { accessToken } from "./auth";
import type { Recipe } from "./api";
export async function photoRequest(
  id: number,
  method: "PUT" | "DELETE",
  jpeg?: Blob,
): Promise<void> {
  const token = await accessToken();
  const response = await fetch(`/api/recipes/${id}/photo`, {
    method,
    headers: {
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...(jpeg ? { "Content-Type": "image/jpeg" } : {}),
    },
    body: jpeg,
  });
  if (!response.ok)
    throw new Error(
      `Recipe saved, but the photo could not be ${method === "PUT" ? "uploaded" : "removed"}. Open Edit recipe to retry.`,
    );
}
export async function resizePhoto(file: File): Promise<Blob> {
  if (!["image/jpeg", "image/png", "image/webp"].includes(file.type))
    throw new Error("Choose a JPEG, PNG, or WebP photo.");
  const image = await createImageBitmap(file);
  const ratio = Math.min(1, 1200 / image.width, 1200 / image.height);
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.round(image.width * ratio));
  canvas.height = Math.max(1, Math.round(image.height * ratio));
  const context = canvas.getContext("2d");
  if (!context) {
    image.close();
    throw new Error("Photo processing is unavailable in this browser.");
  }
  context.fillStyle = "#fff";
  context.fillRect(0, 0, canvas.width, canvas.height);
  context.drawImage(image, 0, 0, canvas.width, canvas.height);
  image.close();
  return new Promise((resolve, reject) =>
    canvas.toBlob(
      (blob) =>
        blob ? resolve(blob) : reject(new Error("Could not prepare photo.")),
      "image/jpeg",
      0.82,
    ),
  );
}
export function RecipePhoto({
  recipe,
  className = "",
}: {
  recipe: Recipe;
  className?: string;
}) {
  const [source, setSource] = useState("");
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let cancelled = false;
    let objectUrl = "";
    setSource("");
    setFailed(false);
    if (recipe.photo_version)
      void (async () => {
        const token = await accessToken();
        const response = await fetch(
          `/api/recipes/${recipe.id}/photo?v=${recipe.photo_version}`,
          {
            headers: token ? { Authorization: `Bearer ${token}` } : {},
            cache: "no-store",
          },
        );
        if (!response.ok) throw new Error("Photo unavailable");
        const blob = await response.blob();
        if (!cancelled) {
          objectUrl = URL.createObjectURL(blob);
          setSource(objectUrl);
        }
      })().catch(() => {
        if (!cancelled) setFailed(true);
      });
    return () => {
      cancelled = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [recipe.id, recipe.photo_version]);
  return source ? (
    <img
      className={`recipe-photo ${className}`}
      src={source}
      alt={recipe.title}
      loading="lazy"
    />
  ) : (
    <span className="photo-placeholder">
      {failed ? "Photo unavailable" : "Loading photo…"}
    </span>
  );
}
