# Kiến Trúc Unified Clack UI & Dual-Brain Agent Engine (`spc`)

## 1. Problem Statement
Làm thế nào để thống nhất toàn bộ 96+ lệnh của `spc` lên một đường ray tương tác `@clack/prompts` (`┌ │ ◆ ◇ └`) chuyên nghiệp kèm cơ chế Zero-Arg Interactive cho con người, đồng thời tự động chuyển sang chế độ siêu nén (Zero-Hang, Token-Compact) khi AI Agent gọi lệnh?

## 2. Recommended Direction (Kiến Trúc Chốt)

### A. Tầng Core UI (`src/ui/prompt.rs`, `src/ui/notify.rs`, `src/ui/card.rs`, `src/ui/table.rs`)
Xây dựng bộ **Clack Primitives hoàn chỉnh** hoạt động trên 1 trục dọc chuẩn (`2 spaces + symbol + 2 spaces`):

```text
  ┌  spc bridge  Multi-VPS Mesh & Tunnel Controller
  │
  ◇  Skipped server 'my-pc' (disabled in bridge.json)
  │
  ◇  Select target VPS to activate
  │  my-vps (103.x.x.x • SOCKS5 :1080)
  │
  ◐  Connecting to 'my-vps' via SSH SOCKS5...          <-- RAII ClackSpinner (tự gập thành ◇ khi xong)
  │
  ├  Connection Summary ────────────────────────────╮  <-- clack_note / clack_table gắn liền trục │
  │  │ Server    : my-vps                           │
  │  │ Endpoint  : socks5://127.0.0.1:1080          │
  │  │ Latency   : 28ms                             │
  ├─────────────────────────────────────────────────╯
  │
  └  ● READY  Active servers: 1/2 online.
```

**Danh mục Clack Primitives chuẩn hóa:**
1. `clack_intro(badge, subtitle)` & `clack_outro(msg)` / `clack_cancel(msg)`
2. `clack_step_ok(msg)` (`◇` xanh), `clack_step_warn(msg)` (`▲` vàng), `clack_step_err(msg)` (`■` đỏ), `clack_step_info(msg)` (`◆` lam), `clack_step_skip(msg)` (`○` xám)
3. `clack_select(msg, options, default, max_rows)` — Chọn 1 mục + gõ phím lọc trực tiếp (live type-to-filter).
4. `clack_multiselect(msg, options, defaults, max_rows)` — Chọn nhiều mục bằng `Space` (`◼` / `◻`), `a` (chọn hết), `Enter` (xác nhận).
5. `clack_text(msg, placeholder, default)` — Nhập văn bản trực tiếp trên trục `│`, hỗ trợ gợi ý mờ (placeholder) và giá trị mặc định.
6. `clack_password(msg)` — Nhập mật khẩu/token hiển thị ký tự che `▪` trên trục `│` (thay thế hoàn toàn `prompt_password` cũ).
7. `clack_confirm(msg, default_bool)` — Chọn nhanh `● Yes / ○ No` bằng phím mũi tên trái/phải, `Tab`, hoặc `y`/`n`.
8. `ClackSpinner::start(msg)` — RAII Guard chạy hiệu ứng xoay `◒ ◐ ◓ ◑` bất đồng bộ; khi `.finish(msg)` / `.fail(msg)` hoặc `Drop` tự động ghi đè thành `◇  {msg}` và khôi phục con trỏ chuột an toàn.
9. `clack_note(title, body)` & `clack_block(rendered_str)` — Nhúng `Card` hoặc `Table` vào bên phải trục `│`.

### B. Tầng Dual-Brain: Tiết Kiệm Token & Chống Treo cho AI Agent
Khi phát hiện môi trường **Non-TTY** (`!stdout.is_terminal()` hoặc `!stdin.is_terminal()`) hoặc có cờ `--json`:
- **Zero-Hang Prompt Guard**: Mọi hàm `clack_select`, `clack_multiselect`, `clack_text`, `clack_password`, `clack_confirm` khi gọi ở Non-TTY sẽ **KHÔNG bao giờ treo chờ `stdin`**, mà trả về lỗi `io::ErrorKind::WouldBlock` kèm danh sách các `options` hợp lệ để Agent đọc được từ thông báo lỗi và truyền thẳng vào tham số CLI ở lệnh tiếp theo.
- **Token-Compact Log**: `ClackSpinner` không in frame xoay lặp dòng; `clack_step_*`, `clack_intro` và `clack_outro` xuất trên 1 dòng phẳng không mã màu ANSI thừa khi chạy ở Non-TTY, giảm ~80% số lượng token mà LLM phải đọc.

### C. Lộ Trình Refactor Subsystems & Zero-Arg Interactive
1. **System & Cloud (`login`, `whoami`, `tenant`, `doctor`, `bootstrap`, `status`, `config`, `upgrade`)**:
   - Thay thế toàn bộ `prompt_input` / `prompt_password` trong `spc login` bằng `clack_select` + `clack_text` + `clack_password` + `ClackSpinner`.
   - `spc tenant switch` (không truyền tên) -> tự bật `clack_select` chọn workspace.
2. **Developer Suite (`dev`, `port`, `code`, `git`, `env`)**:
   - Nâng cấp `spc port` và `spc git` hỗ trợ `clack_multiselect` (kill nhiều port hoặc thao tác nhiều repo cùng lúc).
   - Bọc quá trình quét ổ đĩa bằng `ClackSpinner`.
   - Sửa lỗi `spc env -l` hỏi chọn project khi chạy cờ `-l`: khi truyền `-l` mà không chỉ định project, tự động hiển thị bảng tổng hợp tất cả các project có `.env*` trên máy.
3. **Trụ Cột Microservice (`bridge`, `browser`, `automa`, `runner`, `faker`, `inbox`)**:
   - Chuyển đổi log rời rạc của `spc bridge start/stop/enable/disable`, `spc browser launch/stop`, `spc automa run`, `spc runner start/stop`, `spc inbox` sang timeline Clack.
   - Hỗ trợ **Zero-Arg Interactive**: gõ `spc bridge enable`, `spc bridge disable`, `spc browser launch`, `spc browser stop`, `spc automa run` không tham số sẽ tự liệt kê danh sách sẵn có vào `clack_select`.

## 3. Key Assumptions to Validate
- [x] Trên Windows PowerShell 5.1 / 7 và Windows Terminal, `ClackSpinner` xóa dòng (`\r\x1b[2K`) sạch sẽ không để lại rác ký tự.
- [x] Khi người dùng bấm `Ctrl+C` hoặc `Esc` ở bất kỳ prompt nào (`text`, `password`, `confirm`, `multiselect`, `spinner`), raw mode đều được tắt và `ENABLE_PROCESSED_INPUT | ENABLE_ECHO_INPUT` được khôi phục 100%.
- [x] Các lệnh `-l` / `--list` khi chạy ở chế độ không tham số in thẳng báo cáo tổng hợp mà không bật prompt hỏi người dùng.

## 4. MVP Scope
1. Nâng cấp toàn diện `src/ui/prompt.rs` & `src/ui/notify.rs` với đầy đủ Clack Primitives + Non-TTY Zero-Hang Guard + Token-Compact output.
2. Tích hợp `clack_note` / `clack_block` cho `Card` và `Table`.
3. Refactor các lệnh tương tác & vận hành (`login`, `tenant`, `bridge`, `env`, `port`, `git`, `dev`, `code`, `doctor`, `browser`, `automa`, `runner`, `inbox`) sang chuẩn Clack Timeline & Zero-Arg Interactive.

## 5. Not Doing (Và Lý Do Từ Chối)
- **Không dùng thư viện TUI toàn màn hình (`ratatui` alternate screen) cho CLI commands**: Alternate screen xóa mất lịch sử kết quả trên terminal khi thoát lệnh, làm người dùng không copy-paste được IP/Port/OTP. Inline Clack timeline giữ lại lịch sử sạch đẹp trên màn hình.
- **Không phá vỡ cấu trúc JSON của `--json` và MCP Server**: Mọi công cụ MCP (`src/commands/mcp/`) và cờ `--json` giữ nguyên 100% schema JSON hiện tại để không làm gãy tích hợp với Tauri / Agent.
- **Không bắt buộc nhập tương tác khi đã truyền đủ cờ CLI**: Nếu người dùng hoặc script gõ `spc bridge enable my-vps`, lệnh chạy thẳng trong 1 bước mà không dừng lại hỏi xác nhận thừa.
