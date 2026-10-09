# 📦 Local Database Migrations (SQLite)

This directory manages UP and DOWN migration files for the local database engine running on the client workstation.

---

## 🏛️ Two-Tier Database Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. LOCAL CLIENT RUNTIME (specter runner / cli)                               │
│    - Technology: SQLite 3 (WAL mode)                                        │
│    - Storage: ~/.specter/automa/automa.sqlite (SSOT Pillar 2)                │
│    - Paradigm: Offline-first, single-user embedded high performance         │
│    - Management: migrations/*.sql                                           │
│    - 5 Canonical Microservice Pillars (~/.specter/):                         │
│      ├── system/  : Machine ID, identity, CLI cache, credentials            │
│      ├── automa/  : automa.sqlite (DB), workflows/, worker state            │
│      ├── browser/ : Dedicated runtimes/, profiles/, extensions/             │
│      ├── bridge/  : Mesh config (bridge.json), daemon pids/, proxies        │
│      └── faker/   : Faker schemas, template data (faker.json)               │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       │ Sync Workflows / Remote Telemetry
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 2. CENTRAL CLOUD BAAS HUB (Cloud Control Plane)                             │
│    - Technology: Supabase / PostgreSQL 15+                                  │
│    - Paradigm: Multi-tenant RBAC, RLS policies, quotas                      │
│    - Schema table prefixes: automa_* (workflows, runners...)                │
│    - Management: cloud/supabase/plugins/automa/*.sql                        │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 📂 Local Migration Catalog (SQLite)

- `0001_initial_schema.up.sql`: Initializes 10 local tables (`jobs`, `logs`, `browsers`, `workflows`, `campaigns`, `storage_tables`, `storage_table_rows`, `system_settings`, `storage_variables`, `storage_credentials`).
- `0001_initial_schema.down.sql`: Reversible rollback cleanly tearing down all 10 local tables.
