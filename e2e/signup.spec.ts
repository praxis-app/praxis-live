import { expect, test } from "@playwright/test";

test("user can sign up and send a basic chat message", async ({ page }) => {
  const uniqueSuffix = Date.now().toString().slice(-6);
  const name = `E2E User ${uniqueSuffix}`;
  const email = `e2e_${uniqueSuffix}@example.com`;
  const password = "Password123!";
  const message = `Hello from chat ${uniqueSuffix}`;

  await page.goto("/");

  await page.getByRole("link", { name: "Sign up" }).click();
  await page.getByLabel("Name").fill(name);
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(password);
  await page.getByRole("button", { name: "Create account" }).click();

  await expect(page).toHaveURL(/\/chat\/?/);
  await expect(page.getByRole("heading", { name: "general" })).toBeVisible();
  await expect(page.locator(".sidebar__user-name")).toHaveText(name);
  await expect(page.getByText(email)).toBeVisible();
  await expect(page.getByText("No messages yet")).toBeVisible();

  await page.getByRole("textbox", { name: "Message" }).fill(message);
  await page.getByRole("button", { name: "Send message" }).click();

  await expect(page.getByText(message)).toBeVisible();
  await expect(page.locator(".message-row").first()).toContainText(name);
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("access_token")))
    .not.toBeNull();
});
