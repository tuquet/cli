pub mod fingerprint;
pub mod credential_store;
pub mod client;
pub mod supabase_auth;
pub mod tenancy;

pub use fingerprint::MachineFingerprint;
pub use credential_store::{
    CredentialStore, DeviceCredentials, TenantInfo, CloudSyncResult, CloudStatusInfo,
    DEFAULT_SUPABASE_ANON_KEY,
};
pub use client::CloudApiClient;
pub use supabase_auth::{SupabaseAuthClient, AuthSession, AuthUser};
pub use tenancy::TenancyManager;

