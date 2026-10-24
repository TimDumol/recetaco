import { test, expect } from "@playwright/test";
test("Pocket ID code + PKCE sign-in, authenticated API, reload and sign-out", async ({
  page,
}) => {
  await page.goto("http://127.0.0.1:18083");
  await expect(
    page.getByRole("button", { name: "New recipe", exact: true }),
  ).not.toBeVisible();
  await page
    .getByRole("button", { name: "Sign in with Pocket ID", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Keep the good ones." }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Try a sample recipe" }).click();
  await expect(
    page.getByRole("heading", { name: "Lemony chickpea bowls" }),
  ).toBeVisible();
  await page.getByRole("button", { name: /Lemony chickpea bowls/ }).click();
  await page.getByRole("button", { name: "Edit recipe", exact: true }).click();
  const image = await page.evaluate(() => {
    const canvas = document.createElement("canvas");
    canvas.width = 1600;
    canvas.height = 800;
    const c = canvas.getContext("2d")!;
    c.fillStyle = "#bc5632";
    c.fillRect(0, 0, 1600, 800);
    return canvas.toDataURL("image/png").split(",")[1];
  });
  await page.getByLabel("Recipe photo", { exact: true }).setInputFiles({
    name: "recipe.png",
    mimeType: "image/png",
    buffer: Buffer.from(image, "base64"),
  });
  await page.getByRole("button", { name: "Save recipe", exact: true }).click();
  await expect(page.getByRole("dialog")).not.toBeVisible();
  await expect(
    page.getByRole("img", { name: "Lemony chickpea bowls", exact: true }),
  ).toHaveAttribute("src", /^blob:/);
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Lemony chickpea bowls" }),
  ).toBeVisible();
  await expect(
    page.getByRole("img", { name: "Lemony chickpea bowls", exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: "../artifacts/recipe-photo.png",
    fullPage: true,
  });
  await page.getByRole("button", { name: "Connection settings" }).click();
  await expect(
    page.getByRole("textbox", { name: "Development API token" }),
  ).not.toBeVisible();
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Sign in with Pocket ID", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "New recipe", exact: true }),
  ).not.toBeVisible();
});
