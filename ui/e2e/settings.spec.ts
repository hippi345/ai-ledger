import { expect, test } from "@playwright/test";
import { calls, mockTauri } from "./tauri-mock";

test("hashing existing project names needs explicit confirmation", async ({
  page,
}) => {
  await mockTauri(page, { hash_existing_projects: 2 });
  await page.goto("/#/settings");
  await page
    .getByRole("button", { name: "Hash existing project names…" })
    .click();
  const dialog = page.getByRole("alertdialog", {
    name: "Hash existing project names?",
  });
  await expect(dialog).toContainText("can't be undone");
  await dialog.getByRole("button", { name: "Cancel" }).click();
  expect(
    (await calls(page)).some((c) => c.cmd === "hash_existing_projects"),
  ).toBe(false);

  await page
    .getByRole("button", { name: "Hash existing project names…" })
    .click();
  await page.getByRole("button", { name: "Hash names permanently" }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Hashed 2 project names." }),
  ).toBeVisible();
});

test("retention choice is saved", async ({ page }) => {
  await mockTauri(page);
  await page.goto("/#/settings");
  await page.getByLabel("Keep raw events").selectOption("90");
  await expect
    .poll(async () => (await calls(page)).find((c) => c.cmd === "set_setting"))
    .toMatchObject({ args: { key: "retention.raw_event_days", value: 90 } });
});

test("filters from the URL show as chips and can be cleared", async ({
  page,
}) => {
  await mockTauri(page);
  await page.goto("/#/?model=%5B%22gpt-5%22%5D");
  const bar = page.getByRole("toolbar", { name: "Filters" });
  await expect(
    bar.getByRole("button", { name: "Remove Model gpt-5" }),
  ).toBeVisible();
  await bar.getByRole("button", { name: "Clear filters" }).click();
  await expect(bar).toHaveCount(0);
  await expect(page).not.toHaveURL(/gpt-5/);
});
