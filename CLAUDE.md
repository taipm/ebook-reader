# CLAUDE.md

## Team sessions (session2team)

- Khi có ≥2 việc rời nhau về file VÀ `ListAgents` thấy session `<basename cwd>-*`
  khác đang mở → dùng `/session2team` để chia việc, đừng tự làm tuần tự.
- Lead = session kích hoạt `/session2team`. Chỉ lead commit/push.
- Worker: chỉ sửa trong "Được sửa" của brief; không commit/checkout/stash/pull;
  xong thì SendMessage `DONE <việc>` về lead kèm `git status --short <paths>`.
- Tên package ≠ tên thư mục (tra `cargo metadata` / go.mod / pyproject trước khi viết brief).
