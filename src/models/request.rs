use rocket_okapi::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct CreateDbschemaRequest {
    pub name: String,
    pub description: Option<String>,
    pub data: Option<String>,
    pub organisation_id: String,
    pub db_type: String,
    pub repo_origin: String,
    pub version: String,
    pub quick_links: Option<String>,
}

#[derive(Deserialize, JsonSchema, Serialize)]
pub struct UpdateDbschemaRequest {
    pub name: String,
    pub description: Option<String>,
    pub organisation_id: String,
    pub repo_origin: String,
    pub version: String,
    pub quick_links: Option<String>,
}

#[derive(Deserialize, JsonSchema, Serialize)]
pub struct UpdateDbPipelineRequest {
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateDbschemaBranchRequest {
    pub branch_name: String,
    pub data: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UpdateDbschemaBranchRequest {
    pub branch_name: String,
    pub data: Option<String>,
    pub merged: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UpdateServiceRequest {
    pub identifier: String,
    pub env: String,
    pub base_url: String,
    pub base_url_ws: Option<String>,
    pub spec: String,
    pub dependencies: Vec<String>,
    pub tables: Vec<String>,
    pub db_schema_id: Option<String>,
    pub cache_schema_id: Option<String>,
    pub message_queue_schema_id: Option<String>,
    pub service_type: Option<String>,
    pub version: Option<String>,
    pub lang: Option<String>,
    pub description: String,
    pub organization_id: String,
    pub repo_origin: Option<String>,
    pub quick_links: Option<String>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
pub struct CreateOrUpdatePackageRequest {
    pub identifier: String,
    pub package_type: String,
    pub lang: String,
    pub version: String,
    pub description: String,
    pub organization_id: String,
    pub dependencies: Vec<String>,
    pub env: String,
    pub repo_origin: Option<String>,
    pub quick_links: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct PipelineStatusUpdateRequest {
    pub env: String,
    pub status: String,      // can be running, failed, passing, dormant
    pub update_type: String, // can be schema, package, service
    pub org_id: String,      // organization ID to filter
    pub identifier: String,  // identifier to filter
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateOrganizationRequest {
    pub name: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct CreateSnapshotRequest {
    pub version: String,
    pub org_id: String,
    pub infra_repo_origin: String,
    pub quick_links: String,
}
