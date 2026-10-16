# Specter Ecosystem: Embedded BFF Architecture & OpenAPI Contract Guide

## 1. Architectural Philosophy: The Local-First BFF Triad

Specter uses an **Embedded Backend for Frontend (BFF) Gateway** pattern. Frontend clients (Tauri Desktop UI, Chrome/Firefox extensions, Tuquet Web Studio) interact with system capabilities and persistence exclusively through the local Rust BFF daemon.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              CLIENT LAYER                               │
│   Tauri Desktop (WebView2)   │   Browser Extensions   │   Tuquet Studio │
│   (tauri.localhost)          │  (chrome-extension://) │   (*.tuquet.com)│
└────────────────────────────────────┬────────────────────────────────────┘
                                     │ HTTP REST / SSE / WebSocket (CORS)
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                EMBEDDED RUST BFF GATEWAY (127.0.0.1:8765)               │
│   - In-Process Tokio Daemon (Tauri) / Standalone CLI Daemon (`specter`) │
│   - Contract-First Engine: `utoipa` OpenAPI v3 -> `/swagger-ui/`        │
│   - Native Process Supervision & Zero-Zombie Job Objects                │
│   - Silent Token Delegation & Tenant RLS Management                     │
└──────────────────┬──────────────────────────────────┬───────────────────┘
                   │ Sub-millisecond R/W              │ Background Sync
                   ▼                                  ▼
┌──────────────────────────────────┐ ┌────────────────────────────────────┐
│    LOCAL-FIRST STORAGE (SSOT)    │ │        CLOUD CONTROL PLANE         │
│    ~/.specter/automa/            │ │        Supabase Fleet Engine       │
│    automa.sqlite (SQLite WAL)    │ │   (Tenant Auth, Telemetry, Fleets) │
└──────────────────────────────────┘ └────────────────────────────────────┘
```

---

## 2. Invariants & Contract Rules

### ❌ Rule 1: Frontend NEVER Queries Supabase Directly
- **Strictly Prohibited**: Importing or calling `@supabase/supabase-js`, `@supabase/ssr`, or direct Supabase PostgREST/GraphQL URLs in any frontend codebase (`desktop/src`, web extensions, web studio).
- **Rationale**: Direct frontend Supabase access leaks anon/service tokens, introduces cloud network latency to local UI actions, breaks offline-first operation, and creates dual data paths.
- **Enforcement**: All queries (`GET`) and mutations (`POST`, `PUT`, `DELETE`, `PATCH`) MUST route through `http://127.0.0.1:8765/api/v1`.

### ⚡ Rule 2: Local-First SQLite Engine & Silent Cloud Sync
- **Instant Local Execution**: Workflows, tables, variables, credentials, and browser profiles persist to SQLite (`~/.specter/automa/automa.sqlite`) in WAL mode with sub-millisecond latency.
- **Offline Autonomy**: Desktop operates at 100% capacity without internet connectivity.
- **Silent Background Sync**: The native Rust daemon (`runner::core::enrollment`, `cli::api::handlers::system`) detects network connectivity and pushes/pulls changes asynchronously to the Supabase Cloud Control Plane.

### 🔒 Rule 3: Zero-Secret Frontend & Native Token Delegation
- All Supabase JWTs, refresh tokens, machine identities, and Tenant RLS identifiers reside in `~/.specter/system/.identity.json` and Windows Credential Manager / native keyring.
- Frontend never holds cloud credentials. When invoking cloud-backed actions, Rust injects auth headers internally.

### 📜 Rule 4: Contract-First via `utoipa` OpenAPI Macro
- Every endpoint in `cli/src/api/handlers/` MUST be decorated with `#[utoipa::path(...)]`.
- Every payload and response struct MUST derive `utoipa::ToSchema`.
- Every endpoint MUST be registered in `ApiDoc` inside `cli/src/api/routes.rs`.
- Frontend TypeScript types and API hooks MUST be auto-generated from `http://127.0.0.1:8765/api-docs/openapi.json`. Handwritten fetch code or manual types are prohibited.

---

## 3. Rust Axum OpenAPI Implementation Standard

### 3.1 Handler Annotation (`utoipa::path`)

All Axum handlers in `cli/src/api/handlers/` adhere to this annotation pattern:

```rust
use axum::{extract::{State, Path, Query}, Json, response::IntoResponse};
use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, IntoParams};

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateWorkflowRequest {
    /// Human-readable workflow title
    #[schema(example = "Daily Lead Scraper")]
    pub name: String,
    /// Automation workflow JSON DAG definition
    pub nodes: serde_json::Value,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct WorkflowResponse {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

/// Create a new automation workflow
#[utoipa::path(
    post,
    path = "/api/v1/storage/workflows",
    request_body = CreateWorkflowRequest,
    responses(
        (status = 200, description = "Workflow created successfully", body = WorkflowResponse),
        (status = 400, description = "Invalid workflow DAG schema"),
        (status = 500, description = "Database write error")
    ),
    tag = "Storage"
)]
pub async fn create_storage_workflow(
    State(state): State<AppState>,
    Json(payload): Json<CreateWorkflowRequest>,
) -> Result<Json<WorkflowResponse>, AppError> {
    // 1. Write to local SQLite
    let workflow = state.db.lock().await.insert_workflow(&payload)?;
    // 2. Trigger async background cloud sync
    state.notify_sync();
    Ok(Json(workflow))
}
```

### 3.2 Registry in `cli/src/api/routes.rs`

Register all handlers and models in `ApiDoc`:

```rust
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::handlers::storage::create_storage_workflow,
        // ... other handlers
    ),
    components(
        schemas(CreateWorkflowRequest, WorkflowResponse)
    ),
    info(
        title = "Specter CLI & Core Bridge API",
        version = "1.0.0",
        description = "Embedded Backend for Frontend (BFF) Gateway, WebSocket/SSE event hub, and SQLite storage bridge for the Specter Ecosystem."
    ),
    servers(
        (url = "http://127.0.0.1:8765", description = "Local Daemon Bridge Server")
    )
)]
pub struct ApiDoc;
```

---

## 4. Automated TypeScript Code Generation Pipeline

We support two industrial code generation standards for frontend consumers: **Orval** (optimal for TanStack/React Query) and **@hey-api/openapi-ts** (optimal for lightweight typed fetch).

### 4.1 Pipeline Architecture

```
Rust Handlers (#[utoipa::path])
            │
            ▼ (Compiled In-Process)
Axum ApiDoc -> http://127.0.0.1:8765/api-docs/openapi.json
            │
            ├───────────────────────────────────────────┐
            ▼ (pnpm run gen:api)                        ▼ (pnpm run gen:api)
      Orval Generator                           @hey-api/openapi-ts
            │                                           │
            ▼                                           ▼
 desktop/src/api/generated/               desktop/src/api/client/
 ├── specter.schemas.ts                   ├── types.gen.ts
 └── specter.endpoints.ts (React Query)   └── sdk.gen.ts
```

---

### 4.2 Tool A: Orval (Recommended for React & TanStack Query)

Orval generates strongly typed React Query hooks (`useQuery`, `useMutation`), request bodies, and parameter interfaces with zero manual boilerplate.

#### Step 1: Install Dependencies
```bash
cd desktop
pnpm add @tanstack/react-query axios
pnpm add -D orval
```

#### Step 2: Configure Custom Axios/Fetch Client (`desktop/src/api/mutator.ts`)
```typescript
import axios, { AxiosRequestConfig } from 'axios';

export const AXIOS_INSTANCE = axios.create({
  baseURL: 'http://127.0.0.1:8765/api/v1',
  headers: {
    'Content-Type': 'application/json',
  },
  timeout: 30000,
});

export const customInstance = <T>(
  config: AxiosRequestConfig,
  options?: AxiosRequestConfig
): Promise<T> => {
  const source = axios.CancelToken.source();
  const promise = AXIOS_INSTANCE({
    ...config,
    ...options,
    cancelToken: source.token,
  }).then(({ data }) => data);

  // @ts-ignore
  promise.cancel = () => {
    source.cancel('Query was cancelled');
  };

  return promise;
};

export default customInstance;
```

#### Step 3: Configure `desktop/orval.config.ts`
```typescript
import { defineConfig } from 'orval';

export default defineConfig({
  specter: {
    input: {
      target: 'http://127.0.0.1:8765/api-docs/openapi.json',
      validation: false,
    },
    output: {
      mode: 'split',
      target: './src/api/generated/specter.ts',
      schemas: './src/api/generated/models',
      client: 'react-query',
      prettier: true,
      override: {
        mutator: {
          path: './src/api/mutator.ts',
          name: 'customInstance',
        },
        query: {
          useQuery: true,
          useMutation: true,
          signal: true,
        },
      },
    },
  },
});
```

#### Step 4: Add npm Script (`desktop/package.json`)
```json
{
  "scripts": {
    "gen:api": "orval --config ./orval.config.ts"
  }
}
```

#### Step 5: Consuming in React UI Components
```tsx
import React from 'react';
import { useGetWorkflows, useCreateStorageWorkflow } from '@/api/generated/specter';

export function WorkflowList() {
  // 1. Strongly typed query with automatic caching & background revalidation
  const { data: workflows, isLoading, error } = useGetWorkflows();

  // 2. Strongly typed mutation
  const createWorkflow = useCreateStorageWorkflow({
    mutation: {
      onSuccess: () => {
        console.log('Workflow persisted to local SQLite!');
      },
    },
  });

  if (isLoading) return <div>Loading local workflows...</div>;
  if (error) return <div>Error loading workflows: {error.message}</div>;

  return (
    <div>
      <button
        onClick={() =>
          createWorkflow.mutate({
            data: { name: 'Auto Form Submitter', nodes: [] },
          })
        }
      >
        Create Workflow
      </button>
      <ul>
        {workflows?.map((wf) => (
          <li key={wf.id}>{wf.name}</li>
        ))}
      </ul>
    </div>
  );
}
```

---

### 4.3 Tool B: @hey-api/openapi-ts (Modern Modular SDK)

For frontend frameworks using native `fetch` or minimal footprint:

#### Step 1: Install Dependencies
```bash
cd desktop
pnpm add @hey-api/client-fetch
pnpm add -D @hey-api/openapi-ts
```

#### Step 2: Configure `desktop/openapi-ts.config.ts`
```typescript
import { defineConfig } from '@hey-api/openapi-ts';

export default defineConfig({
  client: '@hey-api/client-fetch',
  input: 'http://127.0.0.1:8765/api-docs/openapi.json',
  output: './src/api/client',
});
```

#### Step 3: Initialize Client (`desktop/src/api/index.ts`)
```typescript
import { client } from './client/client.gen';

client.setConfig({
  baseUrl: 'http://127.0.0.1:8765/api/v1',
});

export * from './client';
```

---

## 5. SSE & Real-Time Event Stream Invalidation

The Rust BFF emits real-time events over Server-Sent Events (`/api/v1/events`). The frontend hooks into this stream to invalidate TanStack Query caches automatically without polling:

```typescript
import { useEffect } from 'react';
import { useQueryClient } from '@tanstack/react-query';

export function useBffEventSubscription() {
  const queryClient = useQueryClient();

  useEffect(() => {
    const eventSource = new EventSource('http://127.0.0.1:8765/api/v1/events');

    eventSource.onmessage = (event) => {
      try {
        const payload = JSON.parse(event.data);
        switch (payload.topic) {
          case 'job:status_changed':
          case 'job:finished':
            queryClient.invalidateQueries({ queryKey: ['/jobs'] });
            break;
          case 'storage:workflows_changed':
            queryClient.invalidateQueries({ queryKey: ['/storage/workflows'] });
            break;
          case 'browsers:state_changed':
            queryClient.invalidateQueries({ queryKey: ['/browsers'] });
            break;
          default:
            break;
        }
      } catch (err) {
        console.error('SSE decode error:', err);
      }
    };

    return () => {
      eventSource.close();
    };
  }, [queryClient]);
}
```

---

## 6. Pre-Commit & CI Contract Verification Checklist

To guarantee frontend-backend contract harmony:
1. **Rust Check**: Ensure all endpoints compile with `#[utoipa::path(...)]` and exist in `ApiDoc`.
2. **Schema Generation**: Run `specter` daemon or unit tests to generate current `openapi.json`.
3. **Type Generation**: Run `pnpm gen:api` in `desktop/`.
4. **Zero-Diff Verification**: In CI, verify `git diff --exit-code desktop/src/api/generated` passes cleanly.
