import { expect, type Locator, type Page } from "@playwright/test";

export class ChatPage {
  constructor(private readonly page: Page) {}

  messageFeed(): Locator {
    return this.page.getByLabel("Message feed");
  }

  async goto() {
    await this.page.goto("/");
  }

  async sendMessage(message: string) {
    const messageInput = this.page.getByPlaceholder("Send a message...");
    await messageInput.fill(message);
    await messageInput.press("Enter");
  }

  async expectChannel(channelName: string) {
    await expect(this.page.getByText(channelName, { exact: true }).first()).toBeVisible();
  }

  async expectEmptyFeed() {
    await expect(this.page.getByText("No messages yet")).toBeVisible();
  }

  async expectMessage(message: string, author: string) {
    await expect(this.page.getByText(message)).toBeVisible();
    await expect(this.page.getByText(author).first()).toBeVisible();
  }
}
