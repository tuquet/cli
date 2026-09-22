# ⚙️ Automa Core (Rust Daemon)

**Automa Core** là trái tim thực thi (Execution Engine) và máy chủ API (REST/SSE) của toàn bộ hệ sinh thái Automa ngoài trình duyệt. Kiến trúc tuân thủ nghiêm ngặt **Clean Architecture** và nguyên tắc **SOLID** bằng ngôn ngữ **Rust**.

---

## 🛑 QUY TẮC KIẾN TRÚC (BẮT BUỘC)

### 1. Phân Lớp Độc Lập (Decoupled Layers)
- **Domain/Core:** KHÔNG ĐƯỢC PHÉP import các module `infrastructure` (DB, External API) hay `api` (Axum). Mọi giao tiếp ra ngoài **BẮT BUỘC** thông qua Trait Interfaces (Dependency Inversion).
- **Infrastructure:** Triển khai các Trait của Domain. Chịu trách nhiệm trực tiếp gọi SQLite (`rusqlite`), thao tác file IO, và gọi tiến trình con (Chromium).
- **API Layer:** Chỉ làm nhiệm vụ điều hướng HTTP (Axum) và parse payload. Phải uỷ quyền logic nghiệp vụ cho Domain.

### 2. Xử Lý Bất Đồng Bộ (Async Concurrency)
- **Runtime:** Toàn bộ hệ thống chạy trên `tokio` async runtime.
- **CPU-bound Tasks:** Các tác vụ nặng (như mã hoá AES, parse file JSON khổng lồ) **BẮT BUỘC** chạy qua `tokio::task::spawn_blocking` để tránh block luồng Async chính.
- **Shared State:** State dùng chung qua Axum **BẮT BUỘC** gói trong `Arc<T>`. Dữ liệu thay đổi cần khoá bằng `tokio::sync::RwLock` hoặc `tokio::sync::Mutex` (KHÔNG dùng thư viện chuẩn `std::sync`).

### 3. Xử Lý Lỗi (Error Handling)
- **Cấm Crash:** **TUYỆT ĐỐI KHÔNG** sử dụng `.unwrap()` hay `.expect()` trong code production.
- **Domain Errors:** **BẮT BUỘC** định nghĩa cấu trúc lỗi tập trung thông qua thư viện `thiserror` và lan truyền (propagate) bằng toán tử `?`.

---

## 🗄️ CƠ SỞ DỮ LIỆU (SQLITE)

Hệ thống sử dụng SQLite để ghi nhận trạng thái Job, Logs, Browser Profiles và Storage Tables. Vị trí mặc định: `~/.automa/core/automa.sqlite`.

- **Mô hình WAL:** Cấu hình **BẮT BUỘC** kích hoạt `journal_mode = WAL` để hỗ trợ đa luồng ghi log song song không bị khoá.
- **Bảng `jobs`:** Lưu metadata của luồng thực thi (id, name, options, status, kết quả outputs).
- **Bảng `logs`:** Lưu chi tiết từng step execution, tham chiếu qua khóa ngoại `job_id` (Cấu hình `ON DELETE CASCADE`).
- **Dọn Dẹp (Cleanup):** Daemon **BẮT BUỘC** tự động purge các Job và Logs vượt quá giới hạn (giữ 100 jobs gần nhất) vào thời điểm khởi động server để chống rò rỉ dung lượng.

---

## 🔐 BẢO MẬT VÀ MÃ HOÁ (CRYPTOGRAPHY)

Quản lý thông tin nhạy cảm (Credentials, Tokens, Passwords) trong Global Storage.

- **Không Lưu Bảng Rõ:** **TUYỆT ĐỐI KHÔNG** lưu plain-text lên ổ cứng.
- **Chuẩn Mã Hoá:** Tuân thủ 100% chuẩn AES-256-GCM hoặc AES kết hợp HMAC-SHA256 (tương thích ngược với `crypto-js` của Extension).
- **Zero-knowledge Runtime:** Rust Daemon **TUYỆT ĐỐI KHÔNG** tự động giải mã Credentials khi chạy luồng. Dữ liệu mã hoá được tiêm trực tiếp (inject) nguyên bản vào Web Storage; Browser Extension tự chịu trách nhiệm giải mã tại bộ nhớ (RAM) bằng passphrase của người dùng.
- **CLI Commands:** Yêu cầu người dùng nhập Passphrase qua cờ `--passphrase`, biến môi trường `AUTOMA_PASSPHRASE` hoặc STDIN prompt ẩn khi sử dụng lệnh `encrypt-secret`.

---

## 🛠️ VÒNG ĐỜI JOB & CHIẾN DỊCH (EXECUTION ENGINE)

- **Sanitization Bắt Buộc:** Mọi file JSON import từ ngoài **BẮT BUỘC** đi qua bộ phận tiền xử lý (Auto-Sanitization) để tiêm NanoID, default types và khôi phục edge handles trước khi parse.
- **Zombie Process Prevention:** Quản lý Chromium browser **BẮT BUỘC** sử dụng *Registry Pattern*. Mọi process đều phải đăng ký vào một `Set` toàn cục và bị triệt tiêu (`kill`) sạch sẽ khi nhận tín hiệu graceful shutdown (Ctrl+C).
- **Cross-Validation Linter:** CLI **PHẢI** kiểm tra chéo (Cross-reference) tính toàn vẹn của File liên kết trước khi thực thi. Nếu `execute-workflow` trỏ tới file không tồn tại, báo lỗi `[Missing Workflow Reference]`. Cảnh báo (Warnings) được VS Code thu thập qua Diagnostics để hiển thị trực tiếp lên UI.

---

## 🌐 DEVELOPMENT & API ENDPOINTS (DEV TOOLING)

Khi Automa Core Daemon chạy ngầm (`pnpm run dev:all` hoặc `cargo run`), các cổng giao tiếp và giao diện chẩn đoán được cung cấp tại:

| Giao Diện / Endpoint | Địa Chỉ URL | Giao Thức / Mô Tả |
| :--- | :--- | :--- |
| 📑 **Swagger UI (API Docs)** | **`http://127.0.0.1:8765/swagger-ui`** | Giao diện OpenAPI v3 tương tác trực tiếp, test và khám phá toàn bộ REST APIs. |
| 📄 **OpenAPI Spec (JSON)** | **`http://127.0.0.1:8765/api-docs/openapi.json`** | Bản đặc tả OpenAPI JSON v3 phục vụ codegen và Bruno/Postman sync. |
| 🎨 **Web Studio Canvas** | **`http://127.0.0.1:8765/studio/`** | Visual Workflow Canvas standalone nhúng sẵn từ build `dist/studio`. |
| 📡 **SSE Telemetry** | **`http://127.0.0.1:8765/api/v1/events`** | Server-Sent Events luồng đơn truyền tải real-time logs và tiến trình jobs. |
| ⚡ **WebSocket Control** | **`ws://127.0.0.1:8765/api/v1/ws`** | Kênh WS 2 chiều độ trễ thấp (Pause/Resume/Kill job và live breakpoint debug). |

---

## 📚 BẢNG THUẬT NGỮ CỐT LÕI (CORE CODEBASE TERMINOLOGY)

| Thuật Ngữ Chuẩn (Canonical Term) | Thành Phần Code Đại Diện | Mô Tả Kỹ Thuật Ngắn Gọn |
| :--- | :--- | :--- |
| **Daemon Engine** | `AppState`, `run_server` | Tiến trình dịch vụ Rust chạy ngầm (Axum REST/SSE tại cổng `8765`), là trái tim điều phối execution, quản lý trình duyệt và SQLite DB. |
| **Job** | `SubmitJobPayload`, `JobRunner` | Đơn vị tác vụ thực thi 1 workflow có vòng đời (`queued` ➔ `running` ➔ `completed`/`error`), cấp phát cancel token và stream logs qua SSE. |
| **Ephemeral Profile** | `TempDir`, `TempProfile` | Hồ sơ trình duyệt tạm thời lưu tại `%LOCALAPPDATA%\Temp\automa_browser_<id>_<timestamp>`, tự sinh khi chạy Job và tự xoá sạch sau khi kết thúc. |
| **Persistent Profile** | `BrowserProfile`, `{data_dir}/browsers/{id}` | Hồ sơ trình duyệt lưu cố định, bảo lưu cookies, local storage và đăng nhập lâu dài cho các browser profile do người dùng tạo. |
| **Browser Worker** | `ensure_browser_worker`, `BrowserManager` | Cửa sổ Chromium chạy nền cài sẵn extension Automa để thực thi kịch bản headless thông qua kết nối SSE reader loop nội bộ. |
| **Auto-Sanitization** | `sanitize_workflow` | Tiền xử lý JSON workflow để tự động tiêm nanoid hợp lệ, gán `BlockBasic` và sửa edge handles lỗi trước khi nạp vào engine. |
| **Registry Pattern** | `static ref INSTANCES: Set<...>` | Mẫu quản lý tập trung toàn bộ tiến trình Chromium con để đảm bảo dọn dẹp sạch sẽ (`destroyAll`) ngăn chặn triệt để zombie processes. |
| **AutomaDb** | `AutomaDb`, `SqliteJobRepository` | Cơ sở dữ liệu SQLite nhúng (`journal_mode = WAL`) lưu trữ metadata jobs, step logs, app settings và browser profiles. |
| **Storage Workspace** | `apps/vault/` | Thư mục cục bộ phục vụ Git Version Control và Export/Import thủ công (tuân thủ nguyên tắc Zero Folder Scanning; 100% kịch bản runtime được quản lý tập trung qua SQLite Database API). |


