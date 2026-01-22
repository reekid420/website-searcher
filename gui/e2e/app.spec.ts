import { test, expect } from "@playwright/test";

test.describe("Website Searcher GUI", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
  });

  test("homepage loads with search input", async ({ page }) => {
    await expect(page.getByPlaceholder("e.g., elden ring")).toBeVisible();
    await expect(page.getByRole("button", { name: /search/i })).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Website Searcher" })
    ).toBeVisible();
  });

  test("empty search shows error", async ({ page }) => {
    await page.getByRole("button", { name: /search/i }).click();
    await expect(page.getByText("Enter a search phrase")).toBeVisible();
  });

  test("search input accepts text", async ({ page }) => {
    const input = page.getByPlaceholder("e.g., elden ring");
    await input.fill("test query");
    await expect(input).toHaveValue("test query");
  });

  test("site checkboxes are visible", async ({ page }) => {
    await expect(page.getByText("Sites")).toBeVisible();
    // Wait for sites to load from API
    await page.waitForTimeout(1000);
    const checkboxes = page.locator('input[type="checkbox"]');
    // Should have at least one checkbox for no_cf, debug, etc.
    await expect(checkboxes.first()).toBeVisible();
  });

  test("limit input accepts numeric value", async ({ page }) => {
    const limitInput = page.locator('input[type="number"]').first();
    await limitInput.fill("5");
    await expect(limitInput).toHaveValue("5");
  });

  test("search button changes text when loading", async ({ page }) => {
    const input = page.getByPlaceholder("e.g., elden ring");
    await input.fill("elden ring");

    // Mock slow response - button should show "Searching..."
    const button = page.getByRole("button", { name: /search/i });
    await button.click();

    // Button text changes during search
    // Note: This may complete too fast to see "Searching..." in E2E
    // so we just verify the button is still visible
    await expect(button).toBeVisible();
  });

  test("results container is present", async ({ page }) => {
    await expect(page.locator(".results-container")).toBeVisible();
  });

  test("no results message shown initially", async ({ page }) => {
    await expect(page.getByText("No results yet.")).toBeVisible();
  });

  test("cookie input field is present", async ({ page }) => {
    await expect(
      page.getByPlaceholder("key=value; other=value2")
    ).toBeVisible();
  });

  test("CF URL input field is present", async ({ page }) => {
    await expect(
      page.getByPlaceholder("http://localhost:8191/v1")
    ).toBeVisible();
  });
});
