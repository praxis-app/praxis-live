import {
  expect,
  type Locator,
  type Page,
  type Response,
} from '@playwright/test';

interface Options {
  page: Page;
  scrollContainer: Locator;
  pageSize: number;
  totalItems: number;
  direction: 'up' | 'down';
  matchesPageResponse: (response: Response, offset: number) => boolean;
  onPageLoaded?: (offset: number) => Promise<void>;
}

export async function scrollThroughAllPages({
  page,
  scrollContainer,
  pageSize,
  totalItems,
  direction,
  matchesPageResponse,
  onPageLoaded,
}: Options) {
  if (totalItems <= pageSize) return;

  const delta = direction === 'down' ? 10_000 : -10_000;
  await scrollContainer.hover();

  for (let offset = pageSize; offset < totalItems; offset += pageSize) {
    const nextPageResponse = page.waitForResponse(
      (response) => matchesPageResponse(response, offset),
      { timeout: 10_000 },
    );

    await page.mouse.wheel(0, delta);
    await nextPageResponse;
    await onPageLoaded?.(offset);
  }

  await expect
    .poll(() =>
      scrollContainer.evaluate((element) => Math.abs(element.scrollTop)),
    )
    .toBeGreaterThan(0);
}
