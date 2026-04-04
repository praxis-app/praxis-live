import { expect, test } from "@playwright/test";

test("user can sign up on a fresh instance", async ({ page }) => {
  const uniqueSuffix = Date.now().toString().slice(-6);
  const name = `E2E User ${uniqueSuffix}`;
  const email = `e2e_${uniqueSuffix}@example.com`;
  const password = "Password123!";

  await page.goto("/");

  await page.getByRole("button", { name: "Sign up" }).click();
  await page.getByLabel("Name").fill(name);
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(password);
  await page.getByRole("button", { name: "Create account" }).click();

  await expect(page.getByText(`Account created for ${name}.`)).toBeVisible();
  await expect(page.getByRole("heading", { name })).toBeVisible();
  await expect(page.getByText(email)).toBeVisible();
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("accessToken")))
    .not.toBeNull();
});
