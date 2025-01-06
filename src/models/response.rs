use chrono::{DateTime, Utc};
use rocket_okapi::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, JsonSchema)]
pub struct CreateDbschemaResponse {
    pub message: String,
    pub id: i64,
    pub identifier: String,
}

#[derive(Serialize, JsonSchema)]
pub struct GetDbschemaResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub identifier: Option<String>,
    pub organization_id: String,
}

#[derive(Serialize, JsonSchema)]
pub struct GetDbschemaAndTablesResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub identifier: Option<String>,
    pub db_type: Option<String>,
    pub organization_id: String,
    pub tables: Vec<String>,
    pub pipeline_status: Option<String>,
    pub repo_origin: Option<String>,
    pub quick_links: Option<String>,
}

#[derive(Serialize, JsonSchema)]
pub struct GetDbschemaByIdResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub updated_at: chrono::DateTime<Utc>,
    pub data: Option<String>,
    pub branch_id: Option<i64>,
    pub org_id: Option<String>,
    pub group_id: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CreateDbschemaBranchResponse {
    pub message: String,
    pub id: i64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct UpdateDbschemaBranchResponse {
    pub message: String,
    pub id: i64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct UpdateServiceResponse {
    pub message: String,
    pub service_id: i64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ServicesEnvResponse {
    pub spec: String,
    pub base_url: String,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct ServicesEnvTrimmedResponse {
    pub env_key: String,
    pub base_url: String,
    pub base_url_ws: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub version: Option<String>,
    pub pipeline_status: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ServicesTrimmedResponse {
    pub identifier: String,
    pub envs: Vec<ServicesEnvTrimmedResponse>,
    pub tables: Vec<String>,
    pub dependencies: Vec<String>,
    pub db_schema_id: Option<String>,
    pub cache_schema_id: Option<String>,
    pub message_queue_schema_id: Option<String>,
    pub service_type: Option<String>,
    pub lang: Option<String>,
    pub description: String,
    pub organization_id: String,
    pub repo_origin: Option<String>,
    pub quick_links: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct APISessionDetailsResponse {
    pub sub: String,
    pub exp: usize,
    pub scopes: Vec<String>,
    pub group_id: i64,
    pub org_id: String,
}

#[derive(Serialize, JsonSchema, Debug)]
pub struct ServiceResponse {
    pub id: i64,
    pub identifier: String,
    pub group_id: Option<String>,
    pub db_schema_id: String,
    pub dependencies: Vec<String>,
    pub tables: Vec<String>,
    pub description: String,
    pub organization_id: String,
    pub repo_origin: Option<String>,
}

#[derive(Serialize, JsonSchema)]
pub struct CreateOrUpdatePackageResponse {
    pub message: String,
    pub package_id: i64,
}

#[derive(Serialize, JsonSchema)]
pub struct PackageResponse {
    pub identifier: String,
    pub package_type: String,
    pub lang: String,
    pub version: String,
    pub updated_at: DateTime<Utc>,
    pub description: String,
    pub organization_id: String,
    pub dependencies: Vec<String>,
    pub pipeline_status: Option<String>,
    pub repo_origin: Option<String>,
    pub quick_links: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CreateOrganizationResponse {
    pub message: String,
    pub id: i64,
}

#[derive(Serialize, JsonSchema)]
pub struct WorkspaceSummaryResponse {
    pub slug: String,
    pub name: Option<String>,
    pub is_active: bool,
    pub is_admin: bool,
    pub group_id: String,
    pub infra_repo_origin: Option<String>,
    pub quick_links: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VersionResponse {
    pub version: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct SnapshotsResponse {
    pub version: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, JsonSchema)]
pub struct WorkspaceDetailResponse {
    pub name: Option<String>,
    pub block_positions: Option<String>,
    pub is_active: bool,
    pub is_admin: bool,
}
