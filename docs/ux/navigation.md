# UX audit — Navigation & Shell (`src/routes/+page.svelte` @ 11ea49c + working tree)

Phạm vi: app-header, library popover, focus toggles/⌘ shortcuts, cột Contents (TOC), three-col + resize handle, error banner.
Màu: dùng token `--*` từ `src/app.css` / `docs/DESIGN.md` (0f), không đặt hex cứng — mọi hex trong file hiện tại (`#0a0e14`, `#1e3a8a`, `#2563eb`…) sẽ thay bằng token khi tách component.

## P0 — sai/hỏng, sửa trước

1. **Phím tắt không khớp nhãn** — `onKeyDown` L381-402 vs `title` L477-501.
   Code: ⌘1 toggle normal↔reader, ⌘2 reader, ⌘3 **notes**, ⌘0 toggle reader↔normal. Nhãn: ⌘1 Normal, ⌘2 Reader, ⌘3 **Research**, ⌘0 Notes. Research không có phím nào.
   → Một bảng duy nhất `SHORTCUTS = {"1":"normal","2":"reader","3":"research","4":"notes"}`; ⌘0 = toggle reader↔mode trước đó (nhớ `lastMode`). Nhãn nút render từ bảng này, không gõ tay.

2. **Lỗi đổi chương bị nuốt** — `selectChapter` L98-99 set `errorMsg` nhưng không set `status = "error"`, banner (L507) chỉ hiện khi `status === "error"`. Bấm TOC lỗi → im lặng, chương cũ vẫn hiện tiêu đề mới (L90 gán `currentChapter` trước khi fetch).
   → Gán `currentChapter` sau khi fetch thành công; catch → `status = "error"`. Cho `TocPanel` nhận `error` prop hoặc dùng banner chung.

3. **Error banner không đóng được** — L507-511: chỉ biến mất khi `handleOpen` chạy lại. Lỗi mở file xong vẫn chắn màn hình đến lần mở sau.
   → Nút `×` (`aria-label="Dismiss error"`) set `status = "idle"`; Esc cũng đóng. Nội dung `<pre>` giới hạn `max-height: 6rem; overflow:auto`.

## P1 — trải nghiệm chính

4. **Chưa mở sách: popover thay vì trang Library** — L441 popover 1 cột, `.empty-state` L548-551 chỉ nói "Click Open" và hard-code đường dẫn dev `~/GitHub/ebook-reader/test-data/dracula.epub`.
   → Khi `!doc`: render `LibraryPopover` nội dung ở giữa reader (grid bìa/tiêu đề/tác giả, nút "Open file…"), bỏ hint đường dẫn. Popover trong header chỉ dùng khi đã có sách (đổi nhanh). Cùng component, khác `variant="page" | "popover"`.

5. **TOC không nhấn mạnh & không cuộn theo chương đang đọc** — `.toc-item.active` L983 chỉ đổi nền; không `scrollIntoView`; không `aria-current`.
   → `aria-current="page"` trên item active + viền trái 2px `--accent` + chữ đậm; `$effect` khi `currentChapter` đổi → `el.scrollIntoView({block:"nearest"})`. Bỏ badge `L{n}` (L526, nhiễu) — thụt lề theo `level` đã đủ; giữ `title` attr cho tiêu đề bị cắt.

6. **Không có prev/next chương** — reader kết thúc chương là hết, phải quay lại TOC.
   → Thanh dưới reader: `← Chương trước · Chương sau →` (tính từ index trong `doc.chapters`), phím `[` / `]` hoặc ⌥←/⌥→. Thuộc `+page.svelte`/reader (b9), TocPanel chỉ export `prev/next` helper.

7. **Focus mode: nhãn không nói gì về hiệu ứng** — "Normal/Reader/Research/Notes" (L477-501) không cho biết cột nào ẩn; phím tắt chỉ trong `title` (hover mới thấy).
   → Nút dạng segmented với icon 3 ô (▯▯▯ / ▯ / ▯▯▯ / ▯▯), nhãn + `<kbd>⌘2</kbd>` mờ bên cạnh khi header ≥ 900px; `aria-pressed`. Khi ở `reader` mode nút toggles vẫn phải hiện (hiện tại OK) nhưng nên thu gọn thành icon.

8. **Resize handle khó bắt, không min-width tuyệt đối, không phím** — L946 `width: 4px`; L358/L361 clamp 10–40% (10% ở 1024px = 102px, TOC vô dụng); không hỗ trợ bàn phím dù `role="separator"`.
   → Vùng bắt 10px (`::after` mở rộng, hiển thị 1px); clamp theo px `max(160px, 10%)`–`min(480px, 40%)`; `tabindex="0"` + ←/→ ±16px, `aria-valuenow`. Double-click reset về `widthsForMode`.

## P2 — tinh chỉnh

9. **Header khi cửa sổ hẹp** — `.brand` không có `min-width:0`, `.title` không truncate → ở < ~760px header tràn/đẩy toggles. Không có `@media` nào trong file.
   → `.brand{min-width:0} .title{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}`; < 760px: ẩn nhãn text các nút (chỉ icon + `aria-label`), focus toggles thành 1 nút dropdown; < 560px: ẩn `format-badge`.

10. **Library popover: đóng bằng click-outside chỉ, không Esc, không focus trap** — L52-58 `handleWindowClick`; `role="menu"` nhưng item không phải `role="menuitem"`, không điều hướng ↑↓.
    → Esc đóng + trả focus về trigger; item `role="menuitem"`, ↑↓ chuyển, Enter mở. `(library.length)` trong nhãn nút → chuyển thành badge nhỏ.

11. **Library item thiếu trạng thái "đang mở"** — không đánh dấu sách hiện tại trong list.
    → `aria-current="true"` + dấu ✓ khi `book.id === doc?.source_id` (cần b9 expose id nguồn).

12. **Col-title chữ HOA tracking** (L926-933) + emoji đầu mỗi tiêu đề cột/nút (📚 📝 📂) — hai "tell" template; emoji render khác nhau giữa OS.
    → Sentence case, bỏ emoji hoặc thay icon SVG 14px đơn sắc màu `--fg-muted`.

## Bàn giao giai đoạn 2 (2a → 3 component)
- `AppHeader.svelte`: #1 (bảng SHORTCUTS export để +page dùng chung), #7, #9, #3 (banner slot hoặc prop `error` + `ondismiss`).
- `LibraryPopover.svelte`: #4 (`variant`), #10, #11.
- `TocPanel.svelte`: #5, #2 (nhận `error`), helper prev/next cho #6; #8 handle ở +page (b9) — chỉ đề xuất số px.
