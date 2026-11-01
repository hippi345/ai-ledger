import { expect, test } from "@playwright/test";
import { mockTauri } from "./tauri-mock";

test.beforeEach(async ({ page }) => {
  await mockTauri(page);
});

test("Overview loads with the summary and widgets", async ({ page }) => {
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Overview", level: 2 }),
  ).toBeVisible();
  await expect(page.getByText("Today: 12K tokens, ~$0.42.")).toBeVisible();
  await expect(
    page.getByRole("article", { name: "Tokens over time" }),
  ).toBeVisible();
});

test("every page in the sidebar opens", async ({ page }) => {
  await page.goto("/");
  for (const name of [
    "Apps",
    "Models",
    "Tools & MCP",
    "Projects",
    "Sessions",
    "Costs",
    "Settings",
  ]) {
    await page
      .getByRole("navigation", { name: "Main" })
      .getByRole("link", { name })
      .click();
    await expect(page.getByRole("heading", { name, level: 2 })).toBeVisible();
  }
});

test("the date range is kept in the URL across pages", async ({ page }) => {
  await page.goto("/");
  await page
    .getByRole("group", { name: "Date range" })
    .getByRole("combobox")
    .selectOption("7d");
  await page.getByRole("link", { name: "Models" }).click();
  await expect(page).toHaveURL(/range=7d/);
  await expect(
    page.getByRole("group", { name: "Date range" }).getByRole("combobox"),
  ).toHaveValue("7d");
});
