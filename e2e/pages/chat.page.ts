import { expect, type Locator, type Page } from "@playwright/test";
import { Buffer } from "node:buffer";

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

  async attachImage() {
    const pixelPng = Buffer.from(
      "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+/p9sAAAAASUVORK5CYII=",
      "base64"
    );

    await this.page.getByTestId("image-input").setInputFiles({
      name: "pixel.png",
      mimeType: "image/png",
      buffer: pixelPng,
    });

    await expect(this.page.getByTestId("attached-image-preview")).toBeVisible();
  }

  async expectAttachedImage() {
    await expect(
      this.page.getByRole("img", { name: "Attached image" })
    ).toBeVisible();
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
