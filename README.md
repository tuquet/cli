<div align="center">
  <img src="https://tuquet.com/icons/cli.svg" width="76" height="76" alt="CLI Logo" />
  <h1>Specter Master CLI (`specter`)</h1>
  <p><strong>Interactive Scoped Shell, Service Multiplexer & High-Performance Automation Terminal in Rust</strong></p>

  <p>
    <a href="https://docs.tuquet.com/en/specter/commands/"><img src="https://img.shields.io/badge/Docs-VitePress%20Hub-blue.svg" alt="Documentation Hub" /></a>
    <a href="https://github.com/tuquet/scoop-bucket"><img src="https://img.shields.io/badge/Scoop-specter-brightgreen.svg" alt="Scoop" /></a>
    <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-Clap%2FRatatui-orange.svg" alt="Rust" /></a>
    <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License" /></a>
  </p>

  <p>
    <strong><a href="https://docs.tuquet.com/en/specter/commands/">📖 Đọc toàn bộ tài liệu 68 lệnh CLI tại Documentation Hub &rarr;</a></strong>
  </p>
</div>

---

## 📌 Tổng Quan (Overview)

**Specter Master CLI** (`specter`) là điểm vào thống nhất (Single Entrypoint) điều phối toàn bộ hệ sinh thái Specter. Viết bằng Rust thuần túy với thời gian khởi động sub-millisecond, Specter CLI cung cấp cả giao diện dòng lệnh truyền thống và một **Interactive Scoped REPL** giúp chuyển đổi linh hoạt giữa các subsystem mà không làm phân mảnh công cụ.

* **68 Lệnh Chuẩn hóa**: Bao quát toàn bộ 5 trụ cột microservice (`browser`, `runner`, `automa`, `bridge`, `faker`, `cloud`, `system`).
* **Lưu trữ SSOT**: Tự động giải quyết cấu hình và trạng thái về gốc chuẩn duy nhất `~/.specter/`.

## ⚡ Sử Dụng Nhanh (Quickstart)

```bash
# Khởi chạy Interactive Scoped Shell
specter

# Kiểm tra sức khỏe toàn diện của hệ thống
specter doctor
```

## 🏛️ Kiến Trúc Embedded BFF Gateway (`127.0.0.1:8765`)

Bên cạnh vai trò CLI & Scoped REPL Terminal, `specter` (cùng core crate `automa_core`) đóng vai trò là **Embedded Backend for Frontend (BFF) Gateway** duy nhất cho toàn bộ hệ sinh thái Specter:

```text
┌─────────────────────────────────────────────────────────────┐
│                      CLIENT LAYER                           │
│  Tauri Desktop App  │  Browser Extensions  │  Tuquet Studio │
│  (tauri.localhost)  │ (chrome-extension://)│  (*.tuquet.com)│
└──────────────────────────────┬──────────────────────────────┘
                               │ HTTP / WebSocket / SSE (Full CORS)
                               ▼
┌─────────────────────────────────────────────────────────────┐
│          EMBEDDED RUST BFF GATEWAY (127.0.0.1:8765)         │
│  - In-Process Daemon (Tauri) / Standalone Daemon (`specter`) │
│  - Contract-First: OpenAPI v3 (`utoipa`) -> `/swagger-ui/`  │
│  - Zero Secret Leakage: Quản lý RLS, Session & Token ngầm   │
└──────────────┬──────────────────────────────┬───────────────┘
               │                              │ Async Sync
               ▼                              ▼
┌──────────────────────────────┐ ┌────────────────────────────┐
│   LOCAL-FIRST STORAGE (SSOT) │ │   CLOUD CONTROL PLANE      │
│   ~/.specter/automa/         │ │   Supabase Fleet Engine    │
│   automa.sqlite (Sub-ms R/W) │ │   (Tenant Auth, Telemetry) │
└──────────────────────────────┘ └────────────────────────────┘
```

### 1. Invariants Cốt Lõi của BFF Gateway
- **Cấm Frontend gọi trực tiếp Cloud Supabase**: Desktop frontend, browser extensions và external web tools **tuyệt đối không nhúng `@supabase/supabase-js`** hoặc lưu trữ Supabase service/anon keys. Mọi thao tác truy xuất dữ liệu, cấu hình và kích hoạt tác vụ bắt buộc phải đi qua Rust BFF tại `http://127.0.0.1:8765/api/v1`.
- **Local-First & Đồng Bộ Ngầm**: Dữ liệu đọc/ghi tức thì vào SQLite cục bộ (`~/.specter/automa/automa.sqlite`) với độ trễ sub-millisecond, đảm bảo ứng dụng hoạt động 100% offline-ready. Tiến trình nền Rust tự động quản lý luồng đồng bộ hai chiều lên Supabase Cloud Control Plane.
- **Contract-First qua OpenAPI (`utoipa`)**: Mọi endpoint mới trong `cli/src/api/` bắt buộc được gán macro `#[utoipa::path(...)]`. Đặc tả OpenAPI v3 được xuất tự động tại `http://127.0.0.1:8765/api-docs/openapi.json` và UI tương tác tại `http://127.0.0.1:8765/swagger-ui/`, cho phép frontend tự động sinh type TypeScript chuẩn xác (`orval` / `@hey-api/openapi-ts`).
- **Bảo Mật & Zero Secret Leakage**: Toàn bộ session tokens, refresh tokens và Tenant RLS được Rust quản lý trong secure local store (`~/.specter/system/`). Frontend không bao giờ tiếp xúc với secret keys.
- **Full CORS cho Tauri & Extensions**: Cổng BFF đã cấu hình CORS cho phép các origin:
  - Tauri Native Desktop: `tauri://*`, `http://tauri.localhost`, `https://tauri.localhost`, `*://asset.localhost`
  - Web Portal & Studio: `https://specter.tuquet.com`, `http://specter.tuquet.com`, `https://*.tuquet.com`, `https://automa-studio.vercel.app`
  - Browser Extensions: `chrome-extension://*`, `moz-extension://*`
  - Local Dev Servers: `http://localhost:*`, `http://127.0.0.1:*`

## 📚 Tài Liệu Kỹ Thuật Tập Trung (SSOT)

Toàn bộ danh mục 68 lệnh CLI, bảng tham số cờ, hướng dẫn tương tác REPL và phím tắt được bảo trì duy nhất tại Documentation Hub:

👉 **[https://docs.tuquet.com/en/specter/commands/](https://docs.tuquet.com/en/specter/commands/)**
