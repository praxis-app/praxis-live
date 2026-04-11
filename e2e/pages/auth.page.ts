import { expect, type Page } from "@playwright/test";

import type { TestUser } from "../support/test-data";

export class AuthPage {
  constructor(private readonly page: Page) {}

  async gotoLanding() {
    await this.page.goto("/");
  }

  async gotoSignup() {
    await this.page.goto("/signup");
  }

  async followSignupLink() {
    await this.page.getByRole("link", { name: "Sign up" }).click();
  }

  async signUp(user: TestUser) {
    await this.page.getByLabel("Name").fill(user.name);
    await this.page.getByLabel("Email").fill(user.email);
    await this.page.getByLabel("Password").fill(user.password);
    await this.page.getByRole("button", { name: "Create account" }).click();
  }

  async expectSignedUp() {
    await expect(this.page).toHaveURL(/\/chat\/?/);
  }
}
