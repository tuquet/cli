# ⚡ Tuquet CLI (`tuquet`)

> **Unified Master Control CLI, Cloud Runner Daemon & Distributed Browser Automation Engine for the Tuquet Ecosystem**.  
> Điều phối quy trình làm việc, quản lý Chromium Profile, tự động hóa luồng duyệt web, đồng bộ đám mây và kết nối lưu trữ SQLite cục bộ.

---

## 🚀 HƯỚNG DẪN CÀI ĐẶT (INSTALLATION)

### 1. Cài đặt qua Scoop (Khuyến nghị trên Windows)
```powershell
scoop bucket add tuquet https://github.com/tuquet/scoop-bucket
scoop install tuquet
```

### 2. Cài đặt từ nguồn (Build from Source)
```powershell
git clone https://github.com/tuquet/cli.git
cd cli
cargo build --release
# File nhị phân sinh ra tại: target/release/tuquet.exe
```

---

## 🖥️ PHIÊN TƯƠNG TÁC (INTERACTIVE SCOPED SHELL)

Gõ trực tiếp `tuquet` trong terminal để mở phiên Shell tương tác hỗ trợ **Smart Tab-Completion**, quản lý ngữ cảnh theo phân tầng (Hierarchical Scope) và lưu trữ lịch sử lệnh:

```powershell
tuquet
```

### 1. Vào thẳng phạm vi dịch vụ (Direct Scoped Launch)
Bạn có thể mở shell và đi thẳng vào ngữ cảnh của dịch vụ mong muốn:

```powershell
tuquet automa    # Vào phạm vi Automa: tuquet(automa)>
tuquet runner    # Vào phạm vi Runner: tuquet(runner)> (alias: daemon, worker)
tuquet cloud     # Vào phạm vi Cloud:  tuquet(cloud)>  (alias: auth)
tuquet browser   # Vào phạm vi Browser: tuquet(browser)>
```

### 2. Điều hướng và Phím tắt trong Shell
- **Chuyển ngữ cảnh**: `use <automa | runner | cloud | browser | global>`
- **Trở về phạm vi Global**: Gõ `back` hoặc `cd ..` hoặc `exit` (nếu đang ở sub-scope)
- **Thoát chương trình**: Gõ `exit` hoặc `quit` tại phạm vi Global (hoặc nhấn `Ctrl+D`)
- **Xóa màn hình**: `clear` hoặc `cls`
- **Xem trợ giúp ngữ cảnh**: `help` hoặc `?`
- **Smart Autocomplete (Tab)**: Tự động gợi ý lệnh, cờ tham số (`--headless`, `--timeout`), và **quét tự động danh sách workflow** có trong vault `~/.tuquet/workflows/`.
- **Dung sai tiền tố (Prefix Tolerance)**: Nếu đang ở trong `tuquet(automa)>`, bạn có thể gõ `run flow.json` hoặc `automa run flow.json` đều hoạt động chính xác.

---

## 📋 HƯỚNG DẪN SỬ DỤNG DÒNG LỆNH (CLI REFERENCE)

Hỗ trợ chạy trực tiếp từ dòng lệnh / script CI mà không cần vào Shell:

### 1. Quản lý trạng thái chung (Global Commands)
```powershell
tuquet status               # Kiểm tra sức khỏe toàn diện: Browser, Runner Daemon, Cloud Pairing
tuquet login [token]        # Đăng nhập và xác thực workstation với Tuquet Cloud
tuquet whoami               # Xem thông tin định danh và pairing máy trạm
tuquet logout               # Hủy kết nối và xóa thông tin phiên cloud trên máy
```

### 2. Tự động hóa trình duyệt (Automa Engine)
```powershell
# Chạy workflow (hỗ trợ đường dẫn file .json hoặc workflow ID đã lưu trong vault/DB)
tuquet automa run ./my_workflow.json --headless
tuquet automa run <workflow-id> --timeout 60

# Quản lý danh sách workflows
tuquet automa list                      # Liệt kê workflows trong vault và database
tuquet automa list "scraping"           # Tìm kiếm workflow theo từ khóa
tuquet automa inspect ./my_flow.json    # Kiểm tra tính hợp lệ của cấu trúc đồ thị workflow
tuquet automa import ./backup.json      # Nạp workflow vào lưu trữ cục bộ
tuquet automa export <workflow-id>      # Xuất workflow ra file JSON
tuquet automa delete <workflow-id>      # Xóa workflow khỏi hệ thống

# Mở Web Studio thiết kế trực quan trên trình duyệt
tuquet automa studio
```

### 3. Điều phối Daemon & Cloud Worker (Runner Engine)
```powershell
tuquet runner start --port 8765         # Khởi chạy Runner Daemon ở tiền cảnh (Foreground)
tuquet runner status                    # Kiểm tra trạng thái máy chủ Runner cục bộ
tuquet runner probe                     # Xem bản kê năng lực phần cứng & driver
tuquet runner export-openapi spec.json  # Xuất đặc tả OpenAPI v3 ra file
tuquet runner setup-ext                 # Tiện ích dev khởi động trình duyệt nạp sẵn extension
```

### 4. Quản lý Isolated Chromium Runtime (Browser Management)
Tuquet sử dụng bản Chromium thuần nguồn mở (Pure Open-Source BSD) độc lập, không phụ thuộc vào Chrome cài đặt sẵn của hệ điều hành:

```powershell
tuquet browser status                   # Kiểm tra phiên bản, đường dẫn và dung lượng disk usage
tuquet browser install                  # Tự động tải và cấu hình Chromium chuyên biệt
tuquet browser install --force          # Cài đặt lại nếu runtime bị lỗi
tuquet browser path                     # In đường dẫn tuyệt đối đến file thực thi chromium.exe
tuquet browser clean                    # Xóa runtime Chromium để giải phóng dung lượng ổ cứng
```

---

## 🏛️ SINGLE SOURCE OF TRUTH (SSOT) & CẤU TRÚC LƯU TRỮ

Toàn bộ dữ liệu, runtime và cấu hình của hệ sinh thái Tuquet được quản lý duy nhất tại thư mục canonical:

```
~/.tuquet/
├── workflows/           # Local Workflow Vault (lưu trữ các file kịch bản .json)
├── runtimes/            # Dedicated Isolated Open-Source Chromium Runtime
├── data/
│   └── tuquet.sqlite    # SQLite database nhúng (lưu trữ Jobs, Logs, Variables, Profiles)
└── history.txt          # Lịch sử câu lệnh tương tác của Tuquet Interactive Shell
```

---

## 🌐 GIAO DIỆN & DEV TOOLING ENDPOINTS

Khi Runner Daemon hoạt động (cổng mặc định `8765`), các giao diện phục vụ kiểm thử và debug sẵn sàng tại:

| Giao Diện / Endpoint | Địa Chỉ URL | Giao Thức / Mô Tả |
| :--- | :--- | :--- |
| 📑 **Swagger UI (API Docs)** | **`http://127.0.0.1:8765/swagger-ui`** | Giao diện OpenAPI v3 tương tác trực tiếp, test REST APIs. |
| 📄 **OpenAPI Spec (JSON)** | **`http://127.0.0.1:8765/api-docs/openapi.json`** | Đặc tả OpenAPI JSON v3 cho codegen hoặc Postman/Bruno sync. |
| 🎨 **Web Studio Canvas** | **`http://127.0.0.1:8765/studio/`** | Visual Workflow Canvas thiết kế kéo thả luồng tự động hóa. |
| 📡 **SSE Telemetry** | **`http://127.0.0.1:8765/api/v1/events`** | Server-Sent Events phát logs thời gian thực khi chạy jobs. |
| ⚡ **WebSocket Control** | **`ws://127.0.0.1:8765/api/v1/ws`** | Kênh WebSocket 2 chiều độ trễ thấp điều khiển luồng (Pause/Resume/Kill). |

---

## 🛑 NGUYÊN TẮC KIẾN TRÚC MÃ NGUỒN

1. **Phân Lớp Độc Lập (Decoupled Layers)**:
   - `core`: Chứa logic nghiệp vụ lõi, không phụ thuộc tầng ngoài.
   - `infrastructure`: Triển khai SQLite (`rusqlite`), File I/O, Chromium Process Management.
   - `api`: Axum HTTP, WebSocket, SSE routes và payload parsing.
   - `commands`: Command handlers trả về `Result<(), Box<dyn Error>>`, tuyệt đối không gọi `std::process::exit` để bảo vệ phiên tương tác Shell.
2. **Async Concurrency**:
   - Chạy trên `tokio` multi-thread runtime.
   - Tác vụ CPU-bound chạy qua `tokio::task::spawn_blocking`.
3. **Bảo Mật Zero-Knowledge**:
   - Lưu trữ Credentials mã hóa AES-256 kết hợp HMAC-SHA256, không lưu plain-text.
   - Browser Extension giải mã trên RAM với passphrase của người dùng.

---

## 🔗 LIÊN KẾT TÀI LIỆU CÁC REPOSITORY LIÊN QUAN (ECOSYSTEM REFERENCES)

Tuquet CLI là trung tâm điều phối, kết nối chặt chẽ với các repository chuyên biệt trong hệ sinh thái Tuquet:

| Repository / Module | Tài Liệu Chi Tiết | Vai Trò & Mối Liên Kết |
| :--- | :--- | :--- |
| **Automa Engine** | [📘 `automa/README.md`](../automa/README.md) | Chứa toàn bộ Browser Extension manifest, Background Script & Web Studio. |
| **Tuquet Runner** | [📘 `runner/README.md`](../runner/README.md) | Universal distributed execution node (`tqr`), Win32 Job Objects supervision & driver router. |
| **Tuquet Cloud** | [📘 `cloud/README.md`](../cloud/README.md) | Nền tảng Supabase Multi-Tenant, RPC enroll device và quản lý license. |
| **Tuquet Lib** | [📘 `lib/README.md`](../lib/README.md) | Monorepo thư viện chia sẻ TypeScript (`vue-ui`, `extension-runner`, `crypto`). |
| **Scoop Bucket** | [📘 `scoop-bucket/README.md`](../scoop-bucket/README.md) | Manifest cài đặt Windows Scoop chính thức cho `tuquet` và `tqr`. |
| **Claude-Agy** | [📘 `claude-agy/README.md`](../claude-agy/README.md) | Engine trung gian tích hợp Claude Code CLI và Antigravity OAuth quota. |
| **Tuquet Skills** | [📘 `skills/README.md`](../skills/README.md) | Bộ kịch bản và năng lực (Skills) cho AI Coding Agents. |
| **Ecosystem Root** | [📘 `README.md (Root)`](../README.md) | Trang tổng quan toàn bộ hệ sinh thái Tuquet. |

