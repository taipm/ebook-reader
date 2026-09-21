# UX audit — vùng Reader (`src/routes/+page.svelte` @ 11ea49c)

Phạm vi: reader-content, block/highlight, handleMouseUp/computeCharOffset, selection-toolbar, scrollToAnnotation, empty/loading/error.
Màu/type: **không đặt giá trị cứng** — dùng token `--*` trong `src/app.css` / `docs/DESIGN.md`. Đích giai đoạn 2: `Reader.svelte` + `SelectionToolbar.svelte`.

## P0 — chặn trải nghiệm đọc

1. **Cột đọc không căn giữa, measure quá rộng** — `.reader-content` L1010 `max-width: 720px` không có `margin: 0 auto`; `.reader` L999 padding cố định → chữ dính mép trái khi ẩn sidebar (mode reader). Đề xuất: `max-inline-size: 65ch; margin-inline: auto` (token `--measure`), `.reader` padding theo `--space-*`, fluid theo bề rộng cột.
2. **Toolbar chọn chữ trôi theo scroll** — `.selection-toolbar` L1039 `position: fixed` với toạ độ chụp một lần ở mouseup (L165–170); cuộn `.col` (overflow-y L916) là toolbar lệch khỏi đoạn chọn, lên sát đỉnh thì `top - 50` âm → bị cắt. Đề xuất: trong `SelectionToolbar.svelte` dùng `position: absolute` neo vào container cuộn (toạ độ = rect − containerRect + scrollTop), lật xuống dưới khi thiếu chỗ trên; ẩn khi `scroll`, `Escape`, click ngoài, hoặc selection đổi (`selectionchange`).
3. **Trạng thái "đang mở" vô hình trong reader** — `status === "opening"` chỉ đổi chữ nút Open (L425–426); vùng đọc vẫn hiện sách cũ/empty-state không dấu hiệu. Đề xuất: thêm nhánh `{:else if status === "opening"}` trong section reader — skeleton 6–8 dòng (`--surface-2`) + tên file; giữ `aria-busy="true"` trên section.

## P1 — sai/thiếu hành vi

4. **`<mark onclick>` không có bàn phím** (svelte-check L570, L601) — click chỉ nháy khối 1.5s (`handleHighlightClick` L228), không mở note. Đề xuất: đổi thành `<mark><button class="hl" …>` hoặc `<mark role="button" tabindex="0" onkeydown={Enter/Space}>`; click/Enter → chọn annotation trong cột Notes + focus editor. Bỏ `title=` (tooltip native chậm, không a11y) — hiện note dưới dạng popover khi hover/focus.
5. **Chọn vắt qua 2 đoạn bị nuốt im lặng** — `computeCharOffset` L171 walker chỉ trong `blockEl` của start; end ở đoạn khác → `-1` → `return` L158 không phản hồi. Đề xuất: clamp `char_end = block.text.length` khi end ra ngoài block (tối thiểu), hoặc toolbar hiện ở trạng thái disabled kèm nhắc "Chọn trong một đoạn".
6. **Màu highlight-có-note trùng ngữ nghĩa lỗi** — `.hl.note-hl` L1035 nền hồng + gạch đỏ đọc như "sai". Đề xuất: 1 màu highlight (`--hl`) + biểu tượng/gạch chấm cho note (`text-decoration: underline dotted var(--accent)`), hover: `--hl-hover` nhẹ, không `filter`.
7. **Jump-to-source nháy cả đoạn, không phải mark** — `scrollToAnnotation` L315 chỉ tìm `[data-block-id]`, `.block.flash` L1021 tô cả paragraph. Đề xuất: thêm `data-annotation-id` lên `<mark>`, scroll tới mark, pulse mark (`outline` 2 nhịp, tôn trọng `prefers-reduced-motion`).
8. **Error banner dump `String(e)`** L507–509 dạng `<pre>` mã đỏ full-width, không đóng được, không hành động. Đề xuất: banner có nút "Đóng" + "Thử lại"; thông điệp người dùng (`Không mở được "tên file"`), chi tiết kỹ thuật thu gọn (`<details>`).
9. **Empty-state chứa đường dẫn dev** L548–551 (`~/GitHub/ebook-reader/test-data/...`). Đề xuất: lời mời hành động + nút Open ngay tại chỗ; nếu có `bundled-books/` thì liệt kê 2–3 sách mở nhanh; bỏ path máy dev.

## P2 — đánh bóng

10. **Typography đọc lâu** — body dùng system sans (L682), `line-height 1.7`, `1.05rem`, đoạn cách `1.25rem`. Đề xuất: serif từ `--font-reading` (DESIGN.md), `font-size` từ `--text-reading`, `line-height` ~1.6 cho serif, đoạn: `text-indent` 1.2em + margin 0 (kiểu sách) hoặc giữ cách đoạn nhưng ≤ 0.9em; `hyphens: auto; text-wrap: pretty`; `.reader-title` L1003 cùng họ serif, nhỏ hơn, cách dưới theo `--space-*`.
11. **Toolbar: emoji làm icon, nhãn lẫn ngôn ngữ** L590–594 (🟡/📝/✕). Đề xuất: icon SVG inline + label ngắn nhất quán ("Highlight", "Note"), nút ✕ có `aria-label="Bỏ chọn"`; `role="toolbar"`, focus vào nút đầu khi hiện.
12. **`data-block-text`** L568 nhân đôi toàn bộ text vào DOM attribute (bloat cho sách dài, không dùng ở đâu). Đề xuất: bỏ.
13. **Không có focus-visible / reduced-motion** cho `.hl`, `.tool-btn`, `.block.flash`. Đề xuất: `:focus-visible` outline từ `--focus-ring`; bọc transition trong `@media (prefers-reduced-motion: no-preference)`.
14. **Ngưỡng chọn `< 2` ký tự** L140 — chọn 1 từ ngắn ("I", "a") không được highlight. Đề xuất: bỏ ngưỡng, chỉ chặn chuỗi rỗng sau `trim()`.
15. **Chapter dài không có mốc tiến độ** — không có % / "còn N phút" trong reader. Đề xuất: thanh mảnh 2px dưới header hoặc số trang ước lượng ở footer cột đọc (từ `doc.total_chars`).

## Thứ tự làm ở giai đoạn 2
P0 #1–3 → P1 #4, #6, #7 (cùng chạm `<mark>`) → #5, #8, #9 → P2.
