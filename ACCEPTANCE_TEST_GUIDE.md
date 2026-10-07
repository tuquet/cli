# Tuquet CLI: Kế hoạch & Hướng dẫn Kiểm thử Nghiệm thu Toàn diện (A - Z)

> **Tài liệu nghiệm thu hệ thống (System Acceptance Testing - UAT Runbook)**  
> **Áp dụng cho:** Máy trạm mới (Clean Workstation Environment)  
> **Phiên bản CLI:** `v1.0.0` | **Kiến trúc:** Single Source of Truth (`~/.specter/`)

---

## 1. Mục Tiêu & Nguyên Tắc Nghiệm Thu

Tài liệu này cung cấp quy trình kiểm thử nghiệm thu từng bước từ **A đến Z** trên một máy trạm hoàn toàn mới, nhằm xác minh và chứng minh tính đầy đủ, độc lập, an toàn và hiệu năng cao của bộ công cụ **Tuquet CLI**.

### 1.1. Chuẩn Kiến Trúc 5 Trụ Cột (5 Microservice Pillars)
Toàn bộ dữ liệu vận hành phải tự động quy tụ về thư mục gốc duy nhất `~/.specter/`, tuyệt đối không phân mảnh hay xả file rác ra hệ thống:
1. `system/`: Danh tính phần cứng (`.machine_id`, `.identity.json`), lịch sử CLI, token xác thực.
2. `automa/`: Định nghĩa kịch bản (`workflows/`), cơ sở dữ liệu SQLite (`automa.sqlite`), worker state.
3. `browser/`: Nhân Chromium chuyên dụng (`runtimes/`), hồ sơ ảo (`profiles/`), tiện ích (`extensions/`).
4. `bridge/`: Cấu hình mạng mesh (`bridge.json`), background tunnels & PID proxies (`pids/`).
5. `faker/`: Lược đồ dữ liệu giả lập & template nhân thân (`faker.json`).

### 1.2. Nguyên Tắc An Toàn (Zero-Leak Guardrails)
- **Fail-Safe Gate**: Mọi phiên trình duyệt gắn proxy bắt buộc phải vượt qua Pre-flight Probe; nếu proxy chết, CLI lập tức hủy phiên để chống rò rỉ IP thật của máy trạm.
- **Deterministic Hardware**: Vân tay phần cứng (Canvas noise, Audio buffer, WebGL vendor, CPU cores, RAM) được tạo ra từ PRNG Seed toán học cố định.
- **Distributed Lease Locking**: Hồ sơ Cloud được kiểm soát độc quyền qua Supabase RPC với khóa hàng (`FOR UPDATE`), ngăn chặn hai máy cùng chạy một tài khoản.

---

## 2. Ma Trận Nghiệm Thu Tính Năng (Capability Matrix)

| Giai đoạn | Nhóm tính năng | Lệnh kiểm thử chính | Tiêu chí đạt (Pass Criteria) |
|---|---|---|---|
| **Pillar 1** | Cài đặt & Khởi tạo Môi trường | `tuquet doctor`, `tuquet status` | Môi trường sạch, cấu trúc `~/.specter/` tự sinh đúng 5 domain |
| **Pillar 2** | Nhận diện & Xác thực Đám mây | `tuquet runner enroll`, `tuquet cloud whoami` | Máy nhận diện được hardware GUID và liên kết Supabase Tenant |
| **Pillar 3** | Cầu nối Mạng & Proxy Mesh | `tuquet bridge start`, `tuquet proxy probe` | Tunnel SOCKS5 `127.0.0.1:1080` mở, kiểm tra Egress IP thành công |
| **Pillar 4** | Trình duyệt C++ & Quản lý Hồ sơ | `tuquet browser install`, `tuquet profile pack` | Nhân Chromium C++ cài đặt, nén delta session `.tar.zst` < 1KB |
| **Pillar 5** | Kiểm định Vân tay Stealth Trực quan | `tuquet browser verify` | Vượt Cloudflare Turnstile, vẽ quỹ đạo Bézier chuột tự nhiên |
| **Pillar 6** | Tự động hóa Động cơ kép | `tuquet automa run <wf.json>` | Chạy workflow headless/headful, hỗ trợ Extension mode & CDP |
| **Pillar 7** | Điều phối Cloud Fleet Mesh | `tuquet runner worker --cloud-profile` | Thuê độc quyền (Acquire) -> Giải nén delta -> Chạy -> Trả lease (Release) |
| **Pillar 8** | Dữ liệu Giả & Tiện ích Agent | `tuquet faker card`, `tuquet mcp` | Sinh thẻ tín dụng hợp lệ Luhn, sẵn sàng cho Agentic AI qua MCP |

---

## 3. Quy Trình Kiểm Thử Chi Tiết Từng Bước (Step-by-Step)

### Giai đoạn 1: Cài đặt & Khởi tạo Hệ thống (Zero-Touch Provisioning)

#### Bước 1.1: Cài đặt Tuquet CLI qua Scoop (Khuyên dùng)
```powershell
# Thêm bucket Tuquet và cài đặt binary chính thức
scoop bucket add tuquet https://github.com/tuquet/scoop-bucket
scoop install tuquet

# Kiểm tra phiên bản thực thi
tuquet --version
```
- **Kỳ vọng:** Xuất ra `tuquet 1.0.0` (hoặc mới nhất).
- **Mã lỗi:** Exit Code `0`.

#### Bước 1.2: Chạy Bác sĩ Hệ thống (System Doctor)
```powershell
tuquet doctor
```
- **Kỳ vọng:** Hiển thị bảng chẩn đoán hệ thống:
  - Hệ điều hành & Kiến trúc: Windows x86_64
  - Runtimes & Dependencies: Scoop, Git, PowerShell
  - Thư mục dữ liệu chuẩn: `~/.specter/`
  - Đánh giá tổng thể: `● ALL CHECKS PASSED`

#### Bước 1.3: Khởi tạo Cấu trúc Dữ liệu Chuẩn (SSOT)
```powershell
tuquet status
```
- **Kỳ vọng:** CLI tự động tạo 5 thư mục rễ (`system`, `automa`, `browser`, `bridge`, `faker`) dưới `~/.specter/`.
- **Kiểm tra vật lý:**
```powershell
Get-ChildItem -Path "$env:USERPROFILE\.specter"
```

---

### Giai đoạn 2: Nhận Diện & Định Danh Đám Mây (Identity & Cloud Enrollment)

#### Bước 2.1: Kiểm tra Vân tay Phần cứng & Khả năng của Nút
```powershell
tuquet runner probe
```
- **Kỳ vọng:** Trả về JSON Manifest khai báo engine `chromium-extension-worker`, hỗ trợ profile sandbox, workflow graph, và CDP bridge.

#### Bước 2.2: Đăng ký Thiết bị vào Tuquet Cloud (Zero-Touch Enrollment)
```powershell
tuquet runner enroll --env dev
```
*(Hoặc dùng token đăng ký do quản trị viên cấp: `tuquet runner enroll --token <TENANT_ENROLLMENT_TOKEN>`)*
- **Kỳ vọng:**
  - Sinh mã máy độc bản: SHA-256 (`fp_xxxxxxxxxxxx`) dựa trên bo mạch/CPU.
  - Tạo file lưu trữ danh tính: `~/.specter/system/.identity.json`.
  - Hiển thị thẻ UI: `WORKSTATION ENROLLED` kèm `Device ID` và `Tenant ID`.

#### Bước 2.3: Kiểm tra Xác thực Danh tính
```powershell
tuquet cloud whoami
```
- **Kỳ vọng:** Hiển thị thông tin nút máy trạm đã kết nối thành công với Supabase Cloud.

---

### Giai đoạn 3: Mạng Lưới Bảo Mật & Tiền Kiểm Tra Proxy (Network Bridge & Safety Gate)

#### Bước 3.1: Thăm dò Kết nối Proxy Trực tiếp (Pre-flight Probe)
Kiểm tra khả năng phân giải và phát hiện rò rỉ STUN trước khi khởi chạy:
```powershell
# Kiểm tra proxy SOCKS5 nội bộ
tuquet proxy probe socks5://127.0.0.1:1080

# Hoặc kiểm tra proxy HTTP công cộng/thử nghiệm
tuquet proxy probe http://198.51.100.1:8080 --timeout 3
```
- **Kỳ vọng khi Proxy Online:**
  - Thẻ `NETWORK PROXY PRE-FLIGHT PROBE` hiển thị `● OPERATIONAL`.
  - Hiển thị độ trễ RTT (ms), IP thoát (Egress IP), Quốc gia, và Datacenter.
  - Dòng trạng thái: `WebRTC Shield: Protected (--disable-non-proxied-udp)`.
- **Kỳ vọng khi Proxy Offline:**
  - Thẻ hiển thị `● UNREACHABLE`. Trình duyệt sẽ từ chối mở để bảo vệ IP máy thật.

#### Bước 3.2: Quản lý Đường truyền Bridge Mesh
```powershell
# Kiểm tra trạng thái các tunnel
tuquet bridge status

# (Tùy chọn) Bật tunnel tới VPS dự phòng
tuquet bridge start
```

---

### Giai đoạn 4: Trình Duyệt Ẩn Danh C++ & Đóng Gói Hồ Sơ (Antidetect Engine & Packaging)

#### Bước 4.1: Tải & Cài đặt Nhân Chromium C++ Tinh chỉnh
```powershell
tuquet browser install
```
- **Kỳ vọng:** Tải bản Chromium C++ chuyên dụng (hỗ trợ C++ native spoofing) vào `~/.specter/browser/runtimes/chromium/`.

#### Bước 4.2: Tạo Hồ Sơ Trình Duyệt Ẩn Danh Cục Bộ
```powershell
tuquet profile create --name "Shopee-Seller-VN" --os windows --cores 8 --ram 16 --timezone "Asia/Ho_Chi_Minh" --locale "vi-VN"
```
- **Kỳ vọng:**
  - Sinh PRNG seed phần cứng duy nhất (Deterministic Hardware).
  - Lưu hồ sơ cấu hình vào `~/.specter/browser/profiles/<profile_id>/profile.json`.

#### Bước 4.3: Đóng Gói Hồ Sơ Delta Snapshot (`.tar.zst`)
```powershell
tuquet profile pack Shopee-Seller-VN
```
- **Kỳ vọng:**
  - Tự động lọc bỏ các file rác/cache không cần thiết (GPUCache, Code Cache, Crashpad).
  - Đóng gói dữ liệu LevelDB và Cookies thành file nén siêu nhỏ `<profile_id>.tar.zst` (< 1 KB đối với profile sạch, tỷ lệ nén > 30%).
  - Tính toán mã hash SHA-256 toàn vẹn.

#### Bước 4.4: Kiểm Tra Khôi Phục Hồ Sơ Từ Snapshot (Unpack)
```powershell
tuquet profile unpack Shopee-Seller-VN
```
- **Kỳ vọng:** Xác thực SHA-256 thành công và giải nén nguyên vẹn các bảng cookie LevelDB.

---

### Giai đoạn 5: Kiểm Định Vân Tay Trực Quan Trình Duyệt (Stealth Verification)

#### Bước 5.1: Chạy Thử Nghiệm Tương Tác Chống Phát Hiện Bot
Lệnh này mở trình duyệt trực tiếp, điều khiển chuột theo đường cong Bézier vật lý và tự động giải thử thách Cloudflare Turnstile:
```powershell
tuquet browser verify --url "https://turnstile.zerocdn.com"
```
*(Nếu muốn chạy kiểm tra ngầm không bật giao diện: thêm `--headless`)*
- **Kỳ vọng:**
  - Cửa sổ trình duyệt xuất hiện con trỏ chuột màu cam/đỏ (Visual Cursor Overlay).
  - Chuột di chuyển mượt mà, có gia tốc và giảm tốc tự nhiên (Bézier physics).
  - Turnstile Challenge được kích hoạt và vượt qua mà không bị nhận diện là automation/webdriver (`navigator.webdriver === false`).
  - Terminal in thẻ `TUQUET BROWSER STEALTH VERIFICATION: LIVE PRESENTATION`.

---

### Giai đoạn 6: Tự Động Hóa Kịch Bản Động Cơ Kép (Dual Automation Engine)

#### Bước 6.1: Khám Phá Thư Viện Kịch Bản (Workflow Vault)
```powershell
tuquet automa workflow list
```
- **Kỳ vọng:** Liệt kê các workflow có sẵn trong SQLite và Vault `~/.specter/workflows/`.

#### Bước 6.2: Thực Thi Kịch Bản Mẫu Cục Bộ (Extension Worker Mode)
Chạy workflow tìm kiếm tự động với tham số truyền vào:
```powershell
tuquet automa run fixtures/google_search.workflow.json -p keyword="Tuquet Stealth Engine" --headless
```
- **Kỳ vọng:**
  - Khởi tạo bridge socket nội bộ ngẫu nhiên.
  - Nạp extension MV3 hoặc driver tương ứng.
  - Thực thi tuần tự các node: `trigger` -> `new-tab` -> `forms` -> `finish`.
  - Terminal nhận log trực tiếp từ worker: `>> Run completed successfully.`

---

### Giai đoạn 7: Tự Động Hóa Phân Tán Qua Cloud Fleet Mesh (Distributed Orchestration)

Đây là tính năng cao cấp nhất, kết nối Workstation với Supabase Cloud.

#### Bước 7.1: Tra cứu Đội ngũ Trình duyệt Đám mây (Fleet Inventory)
```powershell
tuquet profile cloud list
```
- **Kỳ vọng:**
  - Kết nối Supabase qua Smart Proxy Auto-detect (cổng 1080/8118 nếu có).
  - Hiển thị bảng tổng hợp 13 hồ sơ Cloud: ID, Tên, Trạng thái (`IDLE` / `RUNNING`), Hardware PRNG Seed, Delta Storage, và Thiết bị đang giữ Lock (`Lease Device`).

#### Bước 7.2: Thuê Độc Quyền Hồ Sơ Đám Mây (Distributed Acquire)
```powershell
tuquet profile cloud acquire c0000000-0000-0000-0000-000000000001
```
- **Kỳ vọng:**
  - Khóa hàng cấp cơ sở dữ liệu (`FOR UPDATE`) gán cho thiết bị hiện tại.
  - Trạng thái trên Cloud chuyển từ `IDLE` sang `RUNNING`.
  - Tự động tải và giải nén delta snapshot `.tar.zst` của phiên làm việc trước.
  - Thẻ `CLOUD PROFILE LEASE ACQUIRED` thông báo thành công.

#### Bước 7.3: Giải Phóng Hồ Sơ & Đẩy Delta Lên Đám Mây (Release & Sync)
```powershell
tuquet profile cloud release c0000000-0000-0000-0000-000000000001
```
- **Kỳ vọng:**
  - Tự động nén delta LevelDB của profile thành file `.tar.zst`.
  - Đẩy metadata kích thước và mã SHA-256 lên Cloud.
  - Mở khóa hàng, trả trạng thái trên Supabase về `IDLE`.

#### Bước 7.4: Chạy Toàn Bộ Chu Trình Tự Động Khép Kín (Autonomous 6-Step Run)
Thực thi trực tiếp một workflow trên một Cloud Profile chỉ bằng một lệnh duy nhất:
```powershell
tuquet automa run fixtures/google_search.workflow.json --cloud-profile FB-Ad-Spender-01 --headless
```
- **Kỳ vọng:** CLI tự động thực hiện hoàn hảo **6 bước tự động**:
  1. `[+] 1/6`: Xác thực danh tính nút máy trạm.
  2. `[+] 2/6`: Thuê độc quyền profile `FB-Ad-Spender-01` trên Cloud.
  3. `[+] 3/6`: Tiền kiểm tra an toàn proxy (Fail-Safe Gate).
  4. `[+] 4/6`: Khởi động trình duyệt và chạy workflow kịch bản.
  5. `[+] 5/6`: Đóng gói delta session (`.tar.zst` + SHA-256 digest).
  6. `[+] 6/6`: Trả lease lock về Cloud Fleet (`status: IDLE`).
  - Thẻ kết quả: `AUTONOMOUS CLOUD WORKFLOW PIPELINE: COMPLETED`.

#### Bước 7.5: Chạy Nút Worker Tự Động Giám Sát Fleet (Cloud Runner Worker)
```powershell
# Chạy 1 vòng lặp thăm dò sức khỏe toàn bộ Fleet
tuquet runner worker --once
```
- **Kỳ vọng:** Hiển thị thẻ `AUTONOMOUS CLOUD FLEET WORKER`, báo cáo tổng số profile `IDLE` và `RUNNING` trong Tenant Pool.

---

### Giai đoạn 8: Dữ Liệu Giả Lập & Giao Diện Shell / MCP (Utilities & Agent Sidecar)

#### Bước 8.1: Sinh Thẻ Tín Dụng Giả Lập Chuẩn Thuật Toán Luhn
```powershell
tuquet faker card --nat US
tuquet faker card --nat VN
```
- **Kỳ vọng:** Xuất ra thẻ Visa/MasterCard ngẫu nhiên, ngày hết hạn tương lai, CVV, và vượt qua thuật toán kiểm tra Luhn checksum.

#### Bước 8.2: Sinh Nhân Thân Người Dùng Đầy Đủ
```powershell
tuquet faker generate --count 1 --nat VN
```
- **Kỳ vọng:** Sinh thông tin hoàn chỉnh gồm Họ tên tiếng Việt, Địa chỉ, Số điện thoại, Email, và Avatar URL.

#### Bước 8.3: Vỏ Tương Tác Toàn Năng (Interactive Shell)
```powershell
tuquet shell
```
- **Kỳ vọng:**
  - Khởi động giao diện dòng lệnh tương tác với prompt màu sắc.
  - Hỗ trợ đổi ngữ cảnh: `use browser`, `use automa`, `use runner`, `use bridge`, `use faker`.
  - Tự động gợi ý lệnh qua phím Tab và phím tắt điều hướng lịch sử.
  - Gõ `exit` để thoát.

---

## 4. Bảng Ký Duyệt Nghiệm Thu (Acceptance Sign-Off Checklist)

Người thực hiện kiểm thử đánh dấu `[x]` vào các hạng mục sau khi hoàn thành kiểm thử trên máy mới:

- [ ] **1. Môi trường & SSOT**: Thư mục `~/.specter/` tự động hình thành đủ 5 domain, không sinh file rác ngoài rễ.
- [ ] **2. Nhận diện Máy trạm**: Lệnh `tuquet runner enroll` và `tuquet cloud whoami` liên kết chuẩn xác với Supabase.
- [ ] **3. An toàn Proxy**: `tuquet proxy probe` phát hiện chính xác proxy sống/chết và bảo vệ WebRTC STUN.
- [ ] **4. Trình duyệt Stealth C++**: Nhân Chromium được cài đặt độc lập, cấu hình phần cứng deterministic hoạt động ổn định.
- [ ] **5. Đóng gói Hồ sơ Delta**: Nén `.tar.zst` loại bỏ cache, kích thước tối ưu (< 1 KB cho profile mới), SHA-256 bảo toàn.
- [ ] **6. Vượt Rào Turnstile**: `tuquet browser verify` di chuyển chuột Bézier tự nhiên, vượt qua thử thách bot.
- [ ] **7. Tự động hóa Kịch bản**: `tuquet automa run` điều phối luồng công việc mượt mà trên Extension worker và CDP.
- [ ] **8. Phân tán Đám mây (Fleet Mesh)**: Quản lý 13 hồ sơ Cloud qua `acquire` / `release` với Distributed Row Lock hoàn toàn tin cậy.
- [ ] **9. Pipeline 6 bước Khép kín**: Lệnh `tuquet automa run --cloud-profile` tự động hóa 100% từ khâu nhận lock đến trả lock.
- [ ] **10. Tiện ích Phụ trợ**: `tuquet faker`, `tuquet shell` và `tuquet doctor` hoạt động trơn tru, trải nghiệm nhất quán.

---

> **Kết luận:** Bộ công cụ **Tuquet CLI** đáp ứng đầy đủ tất cả các tiêu chí về Kiến trúc, Bảo mật, Tự động hóa và Năng lực phân tán đa thiết bị, sẵn sàng đưa vào vận hành sản xuất thương mại.
