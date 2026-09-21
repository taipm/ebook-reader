# UX audit — vùng NOTES (`src/routes/+page.svelte` @ 11ea49c)

Phạm vi: cột `.notes` (L600–670), editor (L610–630), card (L638–665), logic L233–323. Giai đoạn 2 thực thi trong `NotesPanel.svelte`. Màu: chỉ dùng token `--*` của 0f, không giữ hex cứng ở L1082–1207.

## P0 — mất dữ liệu / luồng gãy

1. **Editor vô hình ở reader mode → note "ma".** `addNoteFromSelection` (L259) tạo annotation + `editingAnnotationId`, nhưng cột notes chỉ render khi `columnWidths.notes > 0` (L600). Ở mode `reader` (⌘2) người dùng bấm "📝 Note" và không thấy gì; annotation rỗng vẫn nằm trong list.
   → Trong `addNoteFromSelection`: nếu `columnWidths.notes === 0` thì `setFocusMode("normal")` trước khi mở editor. Hoặc chuyển editor thành popover cạnh selection (không phụ thuộc cột).
2. **Đổi chương giữa lúc gõ = mất draft.** `selectChapter` (L92) xoá `noteDraft` nhưng giữ `editingAnnotationId` → editor vẫn mở, textarea trống, annotation rỗng còn lại.
   → Bỏ dòng `noteDraft = ""` ở L92 (draft thuộc annotation, không thuộc chương). Hoặc gọi `cancelNote()` (đã dọn annotation rỗng).
3. **Xoá 1 click, không undo, không xác nhận.** `deleteAnnotation` (L307) gắn vào nút `×` 22px trong header card (L644–648), hover mới đỏ.
   → Undo kiểu toast 5s: giữ `lastDeleted = {ann, index}`, nút "Hoàn tác" chèn lại đúng vị trí; không dùng `confirm()` (chặn WebView). Nút `×` thêm `aria-label="Xoá ghi chú"`.
4. **Mất toàn bộ khi mở sách khác / reload.** `annotations = []` ở L79 và L113, không có persistence.
   → Ngoài phạm vi NotesPanel nhưng phải ghi rõ: NotesPanel nhận `annotations` qua prop + callback, không tự giữ state, để việc lưu (localStorage/Tauri store) gắn ở +page mà không đụng component.

## P1 — số click & phím tắt & sửa được

5. **Tạo note từ selection = 4 thao tác** (kéo chọn → "Note" → click vào textarea → Save). Textarea L619 không autofocus.
   → `<textarea autofocus>` hoặc `$effect` focus khi `editingAnnotationId` đổi; ⌘/Ctrl+Enter = `saveNote`, Esc = `cancelNote` (`onkeydown` trên textarea, `stopPropagation` để không dính ⌘1–3 ở L381). Mục tiêu: kéo chọn → "Note" → gõ → ⌘Enter = 2 click.
6. **Save với draft rỗng tạo card lệch kind.** `saveNote` (L285) không trim; annotation `kind:"note"` nhưng `note:""` → card hiện "🟡 Highlight" vì L641 nhìn `ann.note` chứ không nhìn `ann.kind`, màu lại là `#FCA5A5` của note.
   → `saveNote`: nếu `!noteDraft.trim()` → gọi `cancelNote()`. Nhãn/màu card lấy từ `ann.kind`; khi lưu note lên highlight thì đổi `kind` sang `"note"`.
7. **Không sửa được note đã có nội dung.** Nút "Add note" chỉ hiện khi `!ann.note` (L657).
   → Luôn hiện "Sửa" (mở editor với `noteDraft = ann.note`); `cancelNote` giữ nguyên note cũ (đã đúng vì chỉ xoá khi `note.trim()` rỗng).
8. **Editor tách rời card đang sửa.** Editor render một chỗ trên đầu cột (L610), card gốc vẫn nằm dưới → sửa card thứ 15 phải nhìn lên đầu.
   → Render editor inline thay chỗ card có `ann.id === editingAnnotationId`; note mới (chưa có card) render ở đầu nhóm chương hiện tại.
9. **"Jump to source" im lặng thất bại khi note ở chương khác.** `scrollToAnnotation` (L315–319) `querySelector` không thấy block → `return`.
   → Nếu `ann.location.chapter_id !== currentChapter.id`: `await selectChapter(chap)` rồi scroll sau `tick()`.
10. **List theo thứ tự tạo, không nhóm.** `allNotes = annotations` (L410), `{#each}` phẳng (L637).
    → `$derived` nhóm theo `location.chapter_id` theo thứ tự `doc.chapters`, trong nhóm sort `block_id`/`char_start`; header nhóm sticky = tên chương + số lượng, click header → `selectChapter`. Chương hiện tại mở, chương khác thu gọn khi > 20 note.

## P2 — trạng thái & nội dung

11. **Empty state** (L634) chỉ có 1 câu. → Hai dòng: "Chưa có ghi chú" + "Bôi đen đoạn văn trong sách rồi chọn Highlight hoặc Note. ⌘3 mở rộng cột này." Không dùng mũi tên `→` trong copy.
12. **`editingAnnotationId` trỏ tới annotation đã xoá.** `deleteAnnotation` đã dọn (L309–312); nhưng sau `annotations = []` (L79/L113) id stale còn lại — `{#if editing}` (L612) chỉ ẩn, không reset. → Reset `editingAnnotationId`/`noteDraft` cùng chỗ với `annotations = []`; trong NotesPanel, `$effect`: nếu `editingAnnotationId` không còn trong `annotations` thì báo `onCancel()`.
13. **Tag không hiển thị** dù `Annotation.tags` có (types.ts L55). → Chỉ render chip khi `tags.length > 0`; chưa làm input tag (YAGNI cho tới khi có nguồn tag).
14. **Focus mode "notes" (⌘3) chỉ nới cột 40%**, không toggle về (⌘1/⌘0 có toggle, ⌘3 không). → ⌘3 lần 2 về `normal`. Mode notes nên kèm ô lọc text (search trong `highlighted_text` + `note`) khi > 10 note.
15. **Card thiếu thời gian**; `slice(0,30)` cắt tên chương bằng JS trong khi CSS đã ellipsis (L1108–1114). → Bỏ slice, hiện `updated_at` tương đối ("2 phút trước") ở góc phải header.
16. **Đường viền/nút hover chỉ có màu**: `.icon-btn:hover` đỏ nhưng không có focus-visible (L1116–1130); `.resize-handle role=separator` không có `tabindex`. → thêm `:focus-visible` dùng token accent; nút xoá ẩn tới khi card hover/focus-within để giảm nhiễu.

## Thứ tự làm ở giai đoạn 2
P0-1, P0-2, P1-5, P1-6 (cùng một chỗ: hàm tạo/lưu/huỷ) → P0-3 undo → P1-7, P1-8 inline editor → P1-10 nhóm chương + P1-9 jump → P2.
