# 📦 Automa Core: Local Database Migrations (SQLite)

Thư mục này quản lý các file migration UP và DOWN cho động cơ máy trạm `automa-core` chạy cục bộ trên môi trường của người dùng.

---

## 🏛️ Phân Biệt Hai Tầng Cơ Sở Dữ Liệu Trong Hệ Sinh Thái

```
┌─────────────────────────────────────────────────────────────┐
│ 1. LOCAL CLIENT RUNTIME (tuquet-automa - apps/core)         │
│    - Công nghệ: SQLite 3 (WAL mode)                         │
│    - File lưu trữ: ~/.automa/automa.db (hoặc in-memory test)│
│    - Bản chất: Offline-first, đơn người dùng (Single-user)  │
│    - Quản lý: apps/core/migrations/*.sql                    │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           │ Sync Workflows / Remote Telemetry
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. CENTRAL CLOUD BAAS HUB (tuquet-cloud)                    │
│    - Công nghệ: Supabase / PostgreSQL                       │
│    - Bản chất: Multi-tenant RBAC, RLS Policy, Quota         │
│    - Bảng tiền tố: automa_* (automa_workflows, runners...)  │
│    - Quản lý: tuquet-cloud/supabase/modules/automa/*.sql    │
└─────────────────────────────────────────────────────────────┘
```

---

## 📂 Danh Mục Migration Local (SQLite)

- `0001_initial_schema.up.sql`: Khởi tạo 10 bảng cục bộ (`jobs`, `logs`, `browsers`, `workflows`, `campaigns`, `storage_tables`, `storage_table_rows`, `system_settings`, `storage_variables`, `storage_credentials`).
- `0001_initial_schema.down.sql`: Rollback và thu hồi sạch sẽ toàn bộ 10 bảng cục bộ.
