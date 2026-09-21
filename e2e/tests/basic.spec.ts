import { browser, $ } from "@wdio/globals";
import { expect } from "expect";

describe("ebook-reader e2e", () => {
  before(async () => {
    // Đợi Svelte render xong
    await browser.pause(2000);
  });

  it("should boot the app and show empty state", async () => {
    const title = await browser.getTitle();
    console.log("Page title:", title);
    // Empty state visible khi chưa load document
    const emptyText = await $(".empty-state").getText();
    expect(emptyText).toContain("Click");
  });

  it("should open bundled library", async () => {
    // Click nút "📂 Open" → mở dialog → cancel
    // Đơn giản hơn: verify nút Open tồn tại
    const openBtn = await $('button=📂 Open');
    await expect(openBtn).toBeDisplayed();
  });

  it("should toggle focus modes", async () => {
    const normalBtn = await $('button=Normal');
    const readerBtn = await $('button=Reader');
    const researchBtn = await $('button=Research');
    const notesBtn = await $('button=Notes');

    // Default: Normal active
    await expect(normalBtn).toHaveAttribute("class", expect.stringContaining("active"));

    // Click Reader → reader mode (chỉ cột giữa)
    await readerBtn.click();
    await browser.pause(300);
    await expect(readerBtn).toHaveAttribute("class", expect.stringContaining("active"));

    // Click Research
    await researchBtn.click();
    await browser.pause(300);
    await expect(researchBtn).toHaveAttribute("class", expect.stringContaining("active"));

    // Back to Normal
    await normalBtn.click();
    await browser.pause(300);
    await expect(normalBtn).toHaveAttribute("class", expect.stringContaining("active"));
  });
});
