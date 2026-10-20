<div align="center">
  <img src="https://tuquet.com/icons/cli.svg" width="76" height="76" alt="CLI Logo" />
  <h1>Master CLI (`tuquet`)</h1>
  <p><strong>Interactive Scoped Shell, Service Multiplexer & High-Performance Automation Terminal in Rust</strong></p>

  <p>
    <a href="https://docs.tuquet.com/en/tuquet/commands/"><img src="https://img.shields.io/badge/Docs-VitePress%20Hub-blue.svg" alt="Documentation Hub" /></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-Clap%2FRatatui-orange.svg" alt="Rust" /></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License" /></a>
  </p>

  <p>
    <strong><a href="https://docs.tuquet.com/en/tuquet/commands/">📖 Đọc toàn bộ tài liệu 96+ lệnh CLI tại Documentation Hub &rarr;</a></strong>
  </p>
</div>

---

## 📌 Tổng Quan (Overview)

**Master CLI** (`tuquet`, alias: `tq`) là điểm vào thống nhất (Single Entrypoint) điều phối toàn bộ hệ sinh thái Tuquet. Viết bằng Rust thuần túy với thời gian khởi động sub-millisecond, CLI cung cấp cả giao diện dòng lệnh truyền thống và một **Interactive Scoped REPL** giúp chuyển đổi linh hoạt giữa các subsystem mà không làm phân mảnh công cụ.

* **96+ Lệnh Chuẩn hóa**: Bao quát toàn diện Developer Productivity Suite và 7 trụ cột microservice (`dev`, `system`, `browser`, `runner`, `automa`, `bridge`, `faker`, `cloud`, `inbox`).
* **Lưu trữ SSOT**: Tự động giải quyết cấu hình và trạng thái về gốc chuẩn duy nhất `~/.tuquet/`.

## 📦 Cài Đặt Nhanh (Modern One-Liner Installation)

### Cách 1: Cài đặt qua Scoop Package Manager (Khuyên dùng - Auto-update & Quản lý Shims):
```powershell
scoop bucket add tuquet https://github.com/tuquet/scoop-bucket
scoop install tuquet
```

### Cách 2: Cài đặt trực tiếp standalone binary không cần package manager:
```powershell
# Windows 10/11 (PowerShell)
powershell -c "irm https://raw.githubusercontent.com/tuquet/cli/main/install.ps1 | iex"
```

```bash
# macOS & Linux (curl)
curl -fsSL https://raw.githubusercontent.com/tuquet/cli/main/install.sh | sh
```

## ⚡ Sử Dụng Nhanh (Quickstart)

```bash
# Khởi chạy Interactive Scoped Shell
tq
# hoặc: tuquet

# Quét toàn máy & chạy script dự án tương tác (Node/Rust/Java/Docker)
tq dev

# Kiểm tra sức khỏe toàn diện của hệ sinh thái
tq doctor
```

## 🌳 Cây Lệnh CLI Thống Nhất (CLI Command Tree)

Dưới đây là sơ đồ cây lệnh phân cấp giúp tra cứu và bao quát nhanh toàn bộ hệ thống lệnh của `tq` (hoặc `tuquet`):

```text
tq (tuquet)
├── 🛠️ Developer Productivity Suite
│   ├── dev                       # Quét đa dự án (Node/Rust/Java/Docker) & chạy script tương tác
│   ├── port [query]              # Quét cổng TCP đang lắng nghe & diệt tiến trình xung đột (-k)
│   ├── code [project]            # Tìm kiếm dự án toàn máy & mở nhanh trong VS Code / Cursor
│   ├── git [repo] [action]       # Radar Git đa repo & đồng bộ an toàn qua Bridge
│   └── env [project]             # So khớp .env vs .env.example, đồng bộ key & chuyển profile
│
├── ⚡ Quick Global Commands
│   ├── status                    # Dashboard tổng quan Cloud, Runner daemon & Browser
│   ├── doctor [--fix]            # Kiểm tra phụ thuộc, công cụ hệ thống & trạng thái sức khỏe
│   ├── bootstrap [--force]       # Onboarding workstation & khởi tạo SSOT (~/.tuquet/) 1 lệnh
│   ├── config [service]          # Xem và chỉnh sửa cấu hình các trụ cột microservice
│   ├── whoami                    # Xem danh tính đăng nhập Cloud & Machine ID
│   ├── login / logout            # Xác thực operator với Supabase / Cloud
│   ├── upgrade                   # Tự động cập nhật binary CLI lên phiên bản mới nhất
│   └── schema [commands|config]  # Kiểm tra manifest & xuất đặc tả JSON schema chuẩn hóa
│
├── 🧩 Microservice Subsystems (Pillars)
│   ├── automa                    # Automation Engine & Workflow Runner
│   │   ├── run <file.json>       # Thực thi kịch bản tự động hóa trực tiếp với CDP
│   │   ├── inspect <file.json>   # Phân tích cú pháp workflow DAG & các node
│   │   ├── probe                 # Kiểm tra tính sẵn sàng của automation worker
│   │   ├── studio                # Mở Automa Visual Web Studio trên trình duyệt
│   │   └── workflow              # Quản lý kho kịch bản (list, info, import, export, delete)
│   │
│   ├── browser                   # Quản lý Trình duyệt Antidetect & Profile
│   │   ├── install / list / use  # Cài đặt, liệt kê và chuyển đổi phiên bản Chromium LTS
│   │   ├── launch / stop / ps    # Khởi chạy profile sandbox, dừng tiến trình & kiểm tra RAM
│   │   ├── verify                # Demo trực tiếp khả năng ẩn danh & stealth fingerprint
│   │   ├── ext                   # Quản lý extension (list, install, catalog, enable, disable)
│   │   └── profile               # Quản lý sandbox profiles cục bộ & đồng bộ Cloud Profile
│   │
│   ├── bridge                    # Mesh Đa VPS & Mạng Proxy Bảo mật
│   │   ├── status / check        # Giám sát trạng thái cụm VPS, đường hầm & độ trễ
│   │   ├── start / stop          # Bật/tắt background daemon mesh tunnels
│   │   ├── enable / disable      # Kích hoạt hoặc tạm ngưng route tới VPS cụ thể
│   │   └── proxy probe <url>     # Kiểm tra IP egress công khai, giao thức và ping proxy
│   │
│   ├── runner                    # Daemon Worker & Trạm Điều Phối Cloud
│   │   ├── start / stop / restart# Vận hành daemon nền phục vụ BFF Gateway (127.0.0.1:8765)
│   │   ├── status / probe / logs # Theo dõi PID, năng lực xử lý và stream log supervisor
│   │   └── worker                # Khởi chạy worker node gia nhập cụm phân tán Cloud Fleet
│   │
│   ├── cloud                     # Định danh Đa Khách thuê (Multi-Tenant)
│   │   ├── login / logout / whoami
│   │   └── tenant [list|switch|current] # Liệt kê, chuyển đổi và ghim workspace làm việc
│   │
│   ├── faker                     # Sinh Dữ liệu Giả lập Doanh nghiệp Chuẩn hóa
│   │   ├── generate              # Sinh profile người dùng chuẩn (CCCD, địa chỉ 3 cấp, SĐT)
│   │   ├── card                  # Xuất thẻ định danh persona chi tiết
│   │   ├── id                    # Sinh số CCCD/CMND độc lập kiểm tra thuật toán Luhn
│   │   └── nationalities         # Danh mục quốc tịch và schema được hỗ trợ
│   │
│   └── inbox                     # Hòm thư Catch-All & Trích xuất OTP Tự động
│       ├── otp <email>           # Bắt mã OTP 4-8 số thời gian thực từ email
│       ├── link <email>          # Trích xuất magic link hoặc link kích hoạt tài khoản
│       ├── list                  # Xem danh sách email và mã xác thực nhận được
│       ├── start / stop / status # Vận hành webhook daemon nhận mail cục bộ
│       ├── test                  # Chạy acceptance test kiểm thử trọn gói luồng OTP
│       └── setup-worker          # Tự động cấu hình Cloudflare Email Routing Worker
│
└── 🤖 Tích Hợp AI & Scoped REPL Shell
    ├── mcp                       # Chạy MCP JSON-RPC 2.0 stdio server cho Claude, Cursor, Antigravity
    └── shell [scope]             # Mở Interactive Scoped REPL Terminal (chuyển ngữ cảnh linh hoạt)
```

## 🏛️ Kiến Trúc Embedded BFF Gateway (`127.0.0.1:8765`)

Bên cạnh vai trò CLI & Scoped REPL Terminal, `tuquet` (cùng core crate `automa_core`) đóng vai trò là **Embedded Backend for Frontend (BFF) Gateway** duy nhất cho toàn bộ hệ sinh thái Tuquet:

```text
┌─────────────────────────────────────────────────────────────┐
│                      CLIENT LAYER                           │
│  Tauri Desktop App  │  Browser Extensions  │  Studio │
│  (tauri.localhost)  │ (chrome-extension://)│  (*.tuquet.com)│
└──────────────────────────────┬──────────────────────────────┘
                               │ HTTP / WebSocket / SSE (Full CORS)
                               ▼
┌─────────────────────────────────────────────────────────────┐
│          EMBEDDED RUST BFF GATEWAY (127.0.0.1:8765)         │
│  - In-Process Daemon (Tauri) / Standalone Daemon (`tuquet`) │
│  - Contract-First: OpenAPI v3 (`utoipa`) -> `/swagger-ui/`  │
│  - Zero Secret Leakage: Quản lý RLS, Session & Token ngầm   │
└──────────────┬──────────────────────────────┬───────────────┘
               │                              │ Async Sync
               ▼                              ▼
┌──────────────────────────────┐ ┌────────────────────────────┐
│   LOCAL-FIRST STORAGE (SSOT) │ │   CLOUD CONTROL PLANE      │
│   ~/.tuquet/automa/         │ │   Supabase Fleet Engine    │
│   automa.sqlite (Sub-ms R/W) │ │   (Tenant Auth, Telemetry) │
└──────────────────────────────┘ └────────────────────────────┘
```

### 1. Invariants Cốt Lõi của BFF Gateway
- **Cấm Frontend gọi trực tiếp Cloud Supabase**: Desktop frontend, browser extensions và external web tools **tuyệt đối không nhúng `@supabase/supabase-js`** hoặc lưu trữ Supabase service/anon keys. Mọi thao tác truy xuất dữ liệu, cấu hình và kích hoạt tác vụ bắt buộc phải đi qua Rust BFF tại `http://127.0.0.1:8765/api/v1`.
- **Local-First & Đồng Bộ Ngầm**: Dữ liệu đọc/ghi tức thì vào SQLite cục bộ (`~/.tuquet/automa/automa.sqlite`) với độ trễ sub-millisecond, đảm bảo ứng dụng hoạt động 100% offline-ready. Tiến trình nền Rust tự động quản lý luồng đồng bộ hai chiều lên Supabase Cloud Control Plane.
- **Contract-First qua OpenAPI (`utoipa`)**: Mọi endpoint mới trong `cli/src/api/` bắt buộc được gán macro `#[utoipa::path(...)]`. Đặc tả OpenAPI v3 được xuất tự động tại `http://127.0.0.1:8765/api-docs/openapi.json` và UI tương tác tại `http://127.0.0.1:8765/swagger-ui/`, cho phép frontend tự động sinh type TypeScript chuẩn xác (`orval` / `@hey-api/openapi-ts`).
- **Bảo Mật & Zero Secret Leakage**: Toàn bộ session tokens, refresh tokens và Tenant RLS được Rust quản lý trong secure local store (`~/.tuquet/system/`). Frontend không bao giờ tiếp xúc với secret keys.
- **Full CORS cho Tauri & Extensions**: Cổng BFF đã cấu hình CORS cho phép các origin:
  - Tauri Native Desktop: `tauri://*`, `http://tauri.localhost`, `https://tauri.localhost`, `*://asset.localhost`
  - Web Portal & Admin: `https://admin.tuquet.com`, `https://tuquet.tuquet.com`, `http://tuquet.tuquet.com`, `https://*.tuquet.com`, `https://automa-studio.vercel.app`
  - Browser Extensions: `chrome-extension://*`, `moz-extension://*`
  - Local Dev Servers: `http://localhost:*`, `http://127.0.0.1:*`

## 📚 Tài Liệu Kỹ Thuật Tập Trung (SSOT)

Toàn bộ danh mục 96+ lệnh CLI, bảng tham số cờ, hướng dẫn tương tác REPL và phím tắt được bảo trì duy nhất tại Documentation Hub:

👉 **[https://docs.tuquet.com/en/tuquet/commands/](https://docs.tuquet.com/en/tuquet/commands/)**
