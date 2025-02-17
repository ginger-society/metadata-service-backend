use crate::middlewares::groups::GroupMemberships;
use crate::middlewares::groups_owned::GroupOwnerships;
use crate::middlewares::IAMService_config::IAMService_config;
use crate::middlewares::NotificationService_api_config::NotificationService_api_config;
use crate::models::schema::{
    Dbschema, DbschemaInsertable, Dbschema_Branch, Dbschema_BranchInsertable, Package,
    PackageInsertable, Package_Env, Package_EnvInsertable, Service, ServiceInsertable,
    Service_Envs, Service_EnvsInsertable, Snapshots, SnapshotsInsertable, Templates,
};
use ginger_shared_rs::rocket_models::RealtimeMessage;
use ginger_shared_rs::rocket_utils::{APIClaims, Claims};

use crate::models::request::{
    CreateDbschemaBranchRequest, CreateDbschemaRequest, CreateOrUpdatePackageRequest,
    CreateOrganizationRequest, CreateSnapshotRequest, PipelineStatusUpdateRequest,
    UpdateDbPipelineRequest, UpdateDbschemaBranchRequest, UpdateDbschemaRequest,
    UpdateServiceRequest,
};
use crate::models::response::{
    APISessionDetailsResponse, CreateDbschemaBranchResponse, CreateDbschemaResponse,
    CreateOrUpdatePackageResponse, CreateOrganizationResponse, GetDbschemaAndTablesResponse,
    GetDbschemaByIdResponse, GetDbschemaResponse, PackageResponse, ServiceResponse,
    ServicesEnvResponse, ServicesEnvTrimmedResponse, ServicesTrimmedResponse, SnapshotsResponse,
    UpdateDbschemaBranchResponse, UpdateServiceResponse, VersionResponse, WorkspaceDetailResponse,
    WorkspaceSummaryResponse,
};

use chrono::Utc;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;
use IAMService::apis::default_api::{identity_create_group, IdentityCreateGroupParams};
use IAMService::models::CreateGroupRequest;
use NotificationService::apis::default_api::{
    publish_message_to_group, PublishMessageToGroupParams,
};
use NotificationService::models::PublishRequest;

#[openapi()]
#[post("/dbschema", data = "<create_request>")]
pub async fn create_dbschema(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    create_request: Json<CreateDbschemaRequest>,
    claims: APIClaims,
    iam_service_config: IAMService_config,
) -> Result<status::Created<Json<CreateDbschemaResponse>>, status::Custom<String>> {
    use crate::models::schema::schema::dbschema::dsl::*;
    use crate::models::schema::schema::dbschema_branch::dsl::*;
    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    let group_uuid = Uuid::new_v4().to_string();

    let dbschema_uuid = Uuid::new_v4().to_string();

    let new_dbschema = DbschemaInsertable {
        name: create_request.name.clone(),
        description: create_request.description.clone(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        data: create_request.data.clone(),
        group_id: None,
        identifier: Some(dbschema_uuid),
        organization_id: Some(create_request.organisation_id.clone()),
        repo_origin: Some(create_request.repo_origin.clone()),
        db_type: create_request.db_type.clone(),
        quick_links: create_request.quick_links.clone(),
    };

    let created_dbschema: Dbschema = diesel::insert_into(dbschema)
        .values(&new_dbschema)
        .get_result::<Dbschema>(&mut conn)
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error inserting new dbschema".to_string(),
            )
        })?;

    // Insert new branch with name "main"
    let new_branch = Dbschema_BranchInsertable {
        data: None,
        branch_name: "stage".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        parent_id: created_dbschema.id,
        version: Some(create_request.version.clone()),
        pipeline_status: None,
    };

    diesel::insert_into(dbschema_branch)
        .values(&new_branch)
        .execute(&mut conn)
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error inserting new branch".to_string(),
            )
        })
        .map(|created_branch| {
            status::Created::new("/dbschema").body(Json(CreateDbschemaResponse {
                message: "Dbschema created successfully".to_string(),
                id: created_dbschema.id,
                identifier: created_dbschema.identifier.unwrap(),
            }))
        })
}

#[openapi()]
#[get("/dbschemas?<search>&<page_number>&<page_size>")]
pub fn get_dbschemas(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    groups: GroupMemberships,
    _claims: Claims,
    search: Option<String>,
    page_number: Option<i64>,
    page_size: Option<i64>,
) -> Result<Json<Vec<GetDbschemaResponse>>, status::Custom<String>> {
    use crate::models::schema::schema::dbschema::dsl::*;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    let memberships: Vec<String> = groups.0;

    let mut query = dbschema.filter(group_id.eq_any(memberships)).into_boxed();

    if let Some(search_term) = search {
        query = query.filter(name.like(format!("%{}%", search_term)));
    }

    // Pagination logic
    let page_number = page_number.unwrap_or(1);
    let page_size = page_size.unwrap_or(10);
    let offset = (page_number - 1) * page_size;

    query = query.offset(offset).limit(page_size);

    let results = query.load::<Dbschema>(&mut conn).map_err(|_| {
        status::Custom(
            Status::InternalServerError,
            "Error retrieving dbschemas".to_string(),
        )
    })?;

    let response = results
        .into_iter()
        .map(|db_schema_| GetDbschemaResponse {
            id: db_schema_.id,
            name: db_schema_.name,
            description: db_schema_.description,
            updated_at: db_schema_.updated_at,
            identifier: db_schema_.identifier,
            organization_id: db_schema_.organization_id.unwrap(),
        })
        .collect();

    Ok(Json(response))
}

#[openapi()]
#[put("/dbschema/<schema_id>/<branch_name>", data = "<update_request>")]
pub fn update_dbschema(
    schema_id: String,
    branch_name: String,
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    update_request: Json<UpdateDbschemaRequest>,
    _claims: APIClaims,
) -> Result<Json<Dbschema>, status::Custom<String>> {
    use crate::models::schema::schema::dbschema::dsl::*;
    use crate::models::schema::schema::dbschema_branch::dsl as dbschema_branch_dsl;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    let updated_rows = diesel::update(dbschema.filter(identifier.eq(schema_id.clone())))
        .set((
            name.eq(update_request.name.clone()),
            description.eq(update_request.description.clone()),
            repo_origin.eq(update_request.repo_origin.clone()),
            organization_id.eq(update_request.organisation_id.clone()),
            quick_links.eq(update_request.quick_links.clone()),
        ))
        .execute(&mut conn)
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Failed to update dbschema".to_string(),
            )
        })?;

    if updated_rows == 0 {
        return Err(status::Custom(
            Status::NotFound,
            "Dbschema not found".to_string(),
        ));
    }

    let updated_dbschema = dbschema
        .filter(identifier.eq(schema_id))
        .first::<Dbschema>(&mut conn)
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error retrieving updated dbschema".to_string(),
            )
        })?;

    diesel::update(
        dbschema_branch_dsl::dbschema_branch
            .filter(dbschema_branch_dsl::parent_id.eq(updated_dbschema.id))
            .filter(dbschema_branch_dsl::branch_name.eq(&branch_name)),
    )
    .set(dbschema_branch_dsl::version.eq(update_request.version.clone()))
    .execute(&mut conn)
    .map_err(|_| {
        status::Custom(
            Status::InternalServerError,
            "Failed to update schema version".to_string(),
        )
    })?;

    Ok(Json(updated_dbschema))
}
fn fetch_dbschema_by_id(
    schema_id: &str,
    branch: Option<&String>,
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
) -> Result<GetDbschemaByIdResponse, status::Custom<String>> {
    use crate::models::schema::schema::dbschema::dsl::*;
    use crate::models::schema::schema::dbschema_branch::dsl::*;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    let result_dbschema = dbschema
        .filter(identifier.eq(schema_id))
        .first::<Dbschema>(&mut conn)
        .map_err(|_| {
            status::Custom(
                Status::NotFound,
                format!("Dbschema with id {} not found", schema_id),
            )
        })?;

    let mut response = GetDbschemaByIdResponse {
        id: result_dbschema.id,
        name: result_dbschema.name.clone(),
        description: result_dbschema.description.clone(),
        updated_at: result_dbschema.updated_at,
        org_id: result_dbschema.organization_id,
        data: None,
        branch_id: None,
        version: None,
        group_id: result_dbschema.group_id
    };

    if let Some(branch_name_val) = branch {
        let result_branch: Dbschema_Branch = dbschema_branch
            .filter(
                parent_id
                    .eq(result_dbschema.id)
                    .and(branch_name.eq(branch_name_val)),
            )
            .first::<Dbschema_Branch>(&mut conn)
            .map_err(|_| {
                status::Custom(
                    Status::NotFound,
                    format!(
                        "Dbschema branch with parent_id {} and branch_name {} not found",
                        schema_id, branch_name_val
                    ),
                )
            })?;

        response.version = result_branch.version.clone();
        response.data = result_branch.data;
        response.branch_id = Some(result_branch.id);
    }

    Ok(response)
}

#[openapi()]
#[get("/dbschemas-branch/<schema_id>?<branch>")]
pub fn get_dbschema_by_id(
    schema_id: String,
    branch: Option<String>,
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    _claims: APIClaims,
) -> Result<Json<GetDbschemaByIdResponse>, status::Custom<String>> {
    let response = fetch_dbschema_by_id(&schema_id, branch.as_ref(), rdb)?;
    Ok(Json(response))
}

#[openapi()]
#[get("/public/dbschemas-branch/<schema_id>?<branch>")]
pub fn get_dbschema_by_id_public(
    schema_id: String,
    branch: Option<String>,
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
) -> Result<Json<GetDbschemaByIdResponse>, status::Custom<String>> {
    let response = fetch_dbschema_by_id(&schema_id, branch.as_ref(), rdb)?;
    Ok(Json(response))
}

#[openapi()]
#[get("/user-land/dbschemas-branch/<schema_id>?<branch>")]
pub fn get_dbschema_by_id_userland(
    schema_id: String,
    branch: Option<String>,
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    _claims: Claims,
) -> Result<Json<GetDbschemaByIdResponse>, status::Custom<String>> {
    let response = fetch_dbschema_by_id(&schema_id, branch.as_ref(), rdb)?;
    Ok(Json(response))
}

#[openapi()]
#[post("/dbschemas/<schema_id>/branches", data = "<branch_request>")]
pub fn create_dbschema_branch(
    schema_id: i64,
    branch_request: Json<CreateDbschemaBranchRequest>,
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    _claims: Claims,
) -> Result<Json<CreateDbschemaBranchResponse>, status::Custom<String>> {
    use crate::models::schema::schema::dbschema::dsl::*;
    use crate::models::schema::schema::dbschema_branch::dsl::*;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    // Check if the Dbschema exists
    let _ = dbschema
        .find(schema_id)
        .first::<Dbschema>(&mut conn)
        .map_err(|_| {
            status::Custom(
                Status::NotFound,
                format!("Dbschema with id {} not found", schema_id),
            )
        })?;

    // Create the new Dbschema_Branch
    let new_branch = Dbschema_BranchInsertable {
        parent_id: schema_id,
        branch_name: branch_request.branch_name.clone(),
        data: branch_request.data.clone(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        version: None,
        pipeline_status: None,
    };

    let inserted_branch: Dbschema_Branch = diesel::insert_into(dbschema_branch)
        .values(&new_branch)
        .get_result::<Dbschema_Branch>(&mut conn)
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Failed to insert new Dbschema_Branch".to_string(),
            )
        })?;

    let response = CreateDbschemaBranchResponse {
        message: "Dbschema branch created successfully".to_string(),
        id: inserted_branch.id,
    };

    Ok(Json(response))
}

#[openapi()]
#[put(
    "/dbschemas/<schema_id>/branches/<branch_id>",
    data = "<branch_request>"
)]
pub fn update_dbschema_branch(
    schema_id: String,
    branch_id: i64,
    branch_request: Json<UpdateDbschemaBranchRequest>,
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    _claims: Claims,
    groups: GroupMemberships
) -> Result<Json<UpdateDbschemaBranchResponse>, status::Custom<String>> {
    use crate::models::schema::schema::dbschema::dsl::*;
    use crate::models::schema::schema::dbschema_branch::dsl as branch_dsl;
    let memberships: Vec<String> = groups.0;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    // Check if the Dbschema and Dbschema_Branch exist
    let db_schema_retrived = dbschema
        .filter(identifier.eq(&schema_id))
        .first::<Dbschema>(&mut conn)
        .map_err(|_| {
            status::Custom(
                Status::NotFound,
                format!("Dbschema with id {} not found", schema_id),
            )
        })?;

    // Check permission
    if let Some(grp_id) = &db_schema_retrived.group_id {
        if !memberships.contains(grp_id) {
            return Err(status::Custom(
                Status::Forbidden,
                "Permission Denied: You are not authorized to update this schema branch.".to_string(),
            ));
        }
    } else {
        return Err(status::Custom(
            Status::Forbidden,
            "Permission Denied: Dbschema group ID is missing.".to_string(),
        ));
    }

    let _ = branch_dsl::dbschema_branch
        .filter(
            branch_dsl::id
                .eq(branch_id)
                .and(branch_dsl::parent_id.eq(db_schema_retrived.id)),
        )
        .first::<Dbschema_Branch>(&mut conn)
        .map_err(|_| {
            status::Custom(
                Status::NotFound,
                format!(
                    "Dbschema branch with id {} not found for schema {}",
                    branch_id, schema_id
                ),
            )
        })?;

    // Perform the update
    let updated_rows =
        diesel::update(branch_dsl::dbschema_branch.filter(branch_dsl::id.eq(branch_id)))
            .set((
                branch_dsl::branch_name.eq(branch_request.branch_name.clone()),
                branch_dsl::data.eq(branch_request.data.clone()),
                branch_dsl::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .map_err(|_| {
                status::Custom(
                    Status::InternalServerError,
                    "Failed to update dbschema branch".to_string(),
                )
            })?;

    if updated_rows == 0 {
        return Err(status::Custom(
            Status::NotFound,
            "Dbschema branch not found".to_string(),
        ));
    }

    let response = UpdateDbschemaBranchResponse {
        message: "Dbschema branch updated successfully".to_string(),
        id: branch_id,
    };

    Ok(Json(response))
}

#[openapi()]
#[put("/services", data = "<service_request>")]
pub async fn update_or_create_service(
    service_request: Json<UpdateServiceRequest>,
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    iam_service_config: IAMService_config,
    _claims: APIClaims,
) -> Result<Json<UpdateServiceResponse>, status::Custom<String>> {
    use crate::models::schema::schema::service::dsl::*;
    use crate::models::schema::schema::service_envs::dsl as service_env_dsl;
    println!("{:?}", service_request);

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    let service_identifier = &service_request.identifier;

    // Check if the service exists
    let existing_service = service
        .filter(identifier.eq(service_identifier))
        .first::<Service>(&mut conn)
        .optional()
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error retrieving service".to_string(),
            )
        })?;

    println!("{:?}", &service_request.service_type.clone().unwrap());
    println!("{:?}", existing_service);
    let service_id = if let Some(s) = existing_service {
        diesel::update(service.filter(id.eq(s.id)))
            .set((
                db_schema_id.eq(&service_request.db_schema_id),
                description.eq(&service_request.description),
                organization_id.eq(&service_request.organization_id),
                service_type.eq(&service_request.service_type.clone().unwrap()),
                lang.eq(&service_request.lang.clone().unwrap()),
                dependencies_json.eq(Some(
                    serde_json::to_string(&service_request.dependencies).unwrap(),
                )),
                tables_json.eq(Some(
                    serde_json::to_string(&service_request.tables).unwrap(),
                )),
                repo_origin.eq(&service_request.repo_origin),
                quick_links.eq(&service_request.quick_links),
                cache_schema_id.eq(&service_request.cache_schema_id),
                message_queue_schema_id.eq(&service_request.message_queue_schema_id),
            ))
            .execute(&mut conn)
            .map_err(|_| {
                status::Custom(
                    Status::InternalServerError,
                    "Error updating service db_schema_id".to_string(),
                )
            })?;

        s.id
    } else {
        let new_service = ServiceInsertable {
            identifier: service_identifier.clone(),
            group_id: None, // Use the group_id from IAM service response
            db_schema_id: service_request.db_schema_id.clone(),
            service_type: service_request
                .service_type
                .clone()
                .expect("Missing service type"),
            tables_json: Some(serde_json::to_string(&service_request.tables).unwrap()),
            dependencies_json: Some(serde_json::to_string(&service_request.dependencies).unwrap()),
            lang: service_request.lang.clone(),
            organization_id: Some(service_request.organization_id.clone()),
            description: Some(service_request.description.clone()),
            repo_origin: service_request.repo_origin.clone(),
            cache_schema_id: service_request.cache_schema_id.clone(),
            message_queue_schema_id: service_request.message_queue_schema_id.clone(),
            quick_links: service_request.quick_links.clone(),
        };

        diesel::insert_into(service)
            .values(&new_service)
            .returning(id)
            .get_result::<i64>(&mut conn)
            .map_err(|_| {
                status::Custom(
                    Status::InternalServerError,
                    "Error creating service".to_string(),
                )
            })?
    };

    // Check if the service environment exists
    let existing_service_env = service_env_dsl::service_envs
        .filter(
            service_env_dsl::parent_id
                .eq(service_id)
                .and(service_env_dsl::env.eq(&service_request.env)),
        )
        .first::<Service_Envs>(&mut conn)
        .optional()
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error retrieving service environment".to_string(),
            )
        })?;

    if let Some(service_env) = existing_service_env {
        // Update the existing service environment
        diesel::update(
            service_env_dsl::service_envs.filter(service_env_dsl::id.eq(service_env.id)),
        )
        .set((
            service_env_dsl::base_url.eq(&service_request.base_url),
            service_env_dsl::base_url_ws.eq(&service_request.base_url_ws),
            service_env_dsl::spec.eq(&service_request.spec),
            service_env_dsl::updated_at.eq(Utc::now()),
            service_env_dsl::version.eq(service_request
                .version
                .clone()
                .unwrap_or("0.0.0".to_string())),
        ))
        .execute(&mut conn)
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error updating service environment".to_string(),
            )
        })?;
    } else {
        // Create a new service environment
        let new_service_env = Service_EnvsInsertable {
            parent_id: service_id,
            env: service_request.env.clone(),
            base_url: service_request.base_url.clone(),
            base_url_ws: service_request.base_url_ws.clone(),
            spec: service_request.spec.clone(),
            updated_at: Some(Utc::now()),
            version: service_request.version.clone().expect("Version is missing"),
            pipeline_status: None,
        };

        diesel::insert_into(service_env_dsl::service_envs)
            .values(&new_service_env)
            .execute(&mut conn)
            .map_err(|_| {
                status::Custom(
                    Status::InternalServerError,
                    "Error creating service environment".to_string(),
                )
            })?;
    }

    let response = UpdateServiceResponse {
        message: "Service and environment updated successfully".to_string(),
        service_id,
    };

    Ok(Json(response))
}

fn fetch_services_and_envs(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    org_id: &str,
    page_number: Option<&String>,
    page_size: Option<&String>,
) -> Result<Vec<ServicesTrimmedResponse>, rocket::http::Status> {
    use crate::models::schema::schema::service::dsl::*;
    use crate::models::schema::schema::service_envs::dsl::*;

    let mut conn = rdb
        .get()
        .map_err(|_| rocket::http::Status::ServiceUnavailable)?;

    let page_number = page_number
        .as_deref()
        .unwrap_or(&"1".to_string())
        .parse::<i64>()
        .unwrap_or(1);
    let page_size = page_size
        .as_deref()
        .unwrap_or(&"10".to_string())
        .parse::<i64>()
        .unwrap_or(10);

    let offset = (page_number - 1) * page_size;

    // Query services and their associated environments
    let services_with_envs = service
        .filter(organization_id.eq(org_id))
        .offset(offset)
        .limit(page_size)
        .load::<Service>(&mut conn)
        .map_err(|_| rocket::http::Status::InternalServerError)?
        .into_iter()
        .map(|s| {
            let envs = service_envs
                .filter(parent_id.eq(s.id))
                .load::<Service_Envs>(&mut conn)
                .map_err(|_| rocket::http::Status::InternalServerError)?;

            let env_responses: Vec<ServicesEnvTrimmedResponse> = envs
                .into_iter()
                .map(|e| ServicesEnvTrimmedResponse {
                    env_key: e.env,
                    base_url: e.base_url,
                    base_url_ws: e.base_url_ws,
                    updated_at: e.updated_at,
                    version: Some(e.version),
                    pipeline_status: e.pipeline_status,
                })
                .collect();

            // Transform `Service` into `ServicesResponse`
            Ok(ServicesTrimmedResponse {
                identifier: s.identifier,
                envs: env_responses,
                tables: serde_json::from_str(&s.tables_json.unwrap()).unwrap(),
                dependencies: serde_json::from_str(&s.dependencies_json.unwrap()).unwrap(),
                db_schema_id: s.db_schema_id,
                cache_schema_id: s.cache_schema_id,
                message_queue_schema_id: s.message_queue_schema_id,
                service_type: Some(s.service_type),
                lang: s.lang,
                organization_id: s.organization_id.unwrap_or(String::from("")),
                description: s.description.unwrap_or(String::from("")),
                repo_origin: s.repo_origin,
                quick_links: s.quick_links,
            })
        })
        .collect::<Result<Vec<ServicesTrimmedResponse>, rocket::http::Status>>()?;

    Ok(services_with_envs)
}

#[openapi]
#[get("/user-land/services-and-envs/<org_id>?<page_number>&<page_size>")]
pub fn get_services_and_envs_user_land(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    _claims: Claims,
    org_id: String,
    page_number: Option<String>,
    page_size: Option<String>,
) -> Result<Json<Vec<ServicesTrimmedResponse>>, rocket::http::Status> {
    let response = fetch_services_and_envs(rdb, &org_id, page_number.as_ref(), page_size.as_ref())?;
    Ok(Json(response))
}

#[openapi]
#[get("/public/services-and-envs/<org_id>?<page_number>&<page_size>")]
pub fn get_services_and_envs_public(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    org_id: String,
    page_number: Option<String>,
    page_size: Option<String>,
) -> Result<Json<Vec<ServicesTrimmedResponse>>, rocket::http::Status> {
    let response = fetch_services_and_envs(rdb, &org_id, page_number.as_ref(), page_size.as_ref())?;
    Ok(Json(response))
}

#[openapi]
#[get("/services-and-envs/<org_id>?<page_number>&<page_size>")]
pub fn get_services_and_envs(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    _claims: APIClaims,
    org_id: String,
    page_number: Option<String>,
    page_size: Option<String>,
) -> Result<Json<Vec<ServicesTrimmedResponse>>, rocket::http::Status> {
    let response = fetch_services_and_envs(rdb, &org_id, page_number.as_ref(), page_size.as_ref())?;
    Ok(Json(response))
}

#[openapi]
#[get("/get-current-workspace")]
pub fn get_current_workspace(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    claims: APIClaims,
) -> Result<Json<APISessionDetailsResponse>, rocket::http::Status> {
    use crate::models::schema::schema::organization::dsl::*;
    let mut conn = rdb
        .get()
        .map_err(|_| rocket::http::Status::InternalServerError)?;

    let result = organization
        .filter(group_id.eq(claims.sub.clone()))
        .first::<Organization>(&mut conn)
        .optional()
        .map_err(|_| rocket::http::Status::InternalServerError)?;

    if let Some(org) = result {
        let session_details = APISessionDetailsResponse {
            sub: claims.sub,
            exp: claims.exp,
            scopes: claims.scopes,
            group_id: claims.group_id,
            org_id: org.slug, // Assuming org_id maps to group_id in Organization
        };
        Ok(Json(session_details))
    } else {
        Err(rocket::http::Status::NotFound)
    }
}
fn fetch_service_and_env_by_id(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    org_id: &str,
    service_identifier: &str,
    env: &str,
) -> Result<ServicesEnvResponse, rocket::http::Status> {
    use crate::models::schema::schema::service::dsl::*;
    use crate::models::schema::schema::service_envs::dsl as service_envs_dsl;

    let mut conn = rdb
        .get()
        .map_err(|_| rocket::http::Status::ServiceUnavailable)?;

    // Query the service by ID
    let service_item = service
        .filter(identifier.eq(service_identifier))
        .filter(organization_id.eq(org_id))
        .first::<Service>(&mut conn)
        .map_err(|_| rocket::http::Status::NotFound)?;

    // Query the specific environment for the service
    let env_item = service_envs_dsl::service_envs
        .filter(service_envs_dsl::parent_id.eq(service_item.id))
        .filter(service_envs_dsl::env.eq(env))
        .first::<Service_Envs>(&mut conn)
        .map_err(|_| rocket::http::Status::NotFound)?;

    // Transform `ServiceEnvs` into `ServicesEnvResponse`
    let env_response = ServicesEnvResponse {
        spec: env_item.spec,
        base_url: env_item.base_url,
    };

    Ok(env_response)
}

#[openapi]
#[get("/services-and-envs/<org_id>/<service_identifier>/<env>")]
pub fn get_service_and_env_by_id(
    org_id: String,
    service_identifier: String,
    env: String,
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    claims: APIClaims,
) -> Result<Json<ServicesEnvResponse>, rocket::http::Status> {
    let env_response = fetch_service_and_env_by_id(rdb, &org_id, &service_identifier, &env)?;
    Ok(Json(env_response))
}

#[openapi]
#[get("/public/services-and-envs/<org_id>/<service_identifier>/<env>")]
pub fn get_service_and_env_by_id_public(
    org_id: String,
    service_identifier: String,
    env: String,
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
) -> Result<Json<ServicesEnvResponse>, rocket::http::Status> {
    let env_response = fetch_service_and_env_by_id(rdb, &org_id, &service_identifier, &env)?;
    Ok(Json(env_response))
}

#[openapi]
#[get("/user-land/services-and-envs/<org_id>/<service_identifier>/<env>")]
pub fn get_service_and_env_by_id_user_land(
    org_id: String,
    service_identifier: String,
    env: String,
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    claims: Claims,
) -> Result<Json<ServicesEnvResponse>, rocket::http::Status> {
    let env_response = fetch_service_and_env_by_id(rdb, &org_id, &service_identifier, &env)?;
    Ok(Json(env_response))
}

#[openapi]
#[get("/services/<service_identifier>/<org_id>")]
pub fn get_service_by_id(
    service_identifier: String,
    org_id: String,
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    claims: APIClaims,
) -> Result<Json<ServiceResponse>, rocket::http::Status> {
    use crate::models::schema::schema::service::dsl::*;

    let mut conn = rdb
        .get()
        .map_err(|_| rocket::http::Status::ServiceUnavailable)?;

    // Query the service by ID and ensure it belongs to one of the user's groups
    let service_item = service
        .filter(organization_id.eq(org_id.clone()))
        .filter(identifier.eq(service_identifier))
        .first::<Service>(&mut conn)
        .map_err(|_| rocket::http::Status::NotFound)?;

    let dependencies: Vec<String> =
        serde_json::from_str(&service_item.dependencies_json.unwrap()).unwrap();

    let tables: Vec<String> = serde_json::from_str(&service_item.tables_json.unwrap()).unwrap();

    let response = ServiceResponse {
        id: service_item.id,
        identifier: service_item.identifier,
        group_id: service_item.group_id,
        db_schema_id: service_item.db_schema_id.unwrap_or(String::from("")),
        dependencies: dependencies,
        tables: tables,
        organization_id: service_item.organization_id.unwrap_or(String::from("")),
        description: service_item.description.unwrap_or(String::from("")),
        repo_origin: service_item.repo_origin,
    };

    Ok(Json(response))
}

#[openapi()]
#[post("/create_or_update_package", data = "<package_request>")]
pub async fn create_or_update_package(
    package_request: Json<CreateOrUpdatePackageRequest>,
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    iam_service_config: IAMService_config,
    _claims: APIClaims,
) -> Result<status::Created<Json<CreateOrUpdatePackageResponse>>, status::Custom<String>> {
    use crate::models::schema::schema::package::dsl::*;
    use crate::models::schema::schema::package_env::dsl as package_env_dsl;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    let package_identifier = &package_request.identifier;

    // Check if the package exists
    let existing_package = package
        .filter(identifier.eq(package_identifier))
        .first::<Package>(&mut conn)
        .optional()
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error retrieving package".to_string(),
            )
        })?;

    let package_id = if let Some(p) = existing_package {
        diesel::update(package.filter(id.eq(p.id)))
            .set((
                package_type.eq(&package_request.package_type),
                lang.eq(&package_request.lang),
                // version.eq(&package_request.version),
                updated_at.eq(Utc::now()),
                dependencies_json.eq(Some(
                    serde_json::to_string(&package_request.dependencies).unwrap(),
                )),
                description.eq(&package_request.description),
                repo_origin.eq(&package_request.repo_origin),
                quick_links.eq(&package_request.quick_links),
            ))
            .execute(&mut conn)
            .map_err(|_| {
                status::Custom(
                    Status::InternalServerError,
                    "Error updating package".to_string(),
                )
            })?;

        p.id
    } else {
        let new_package = PackageInsertable {
            identifier: package_identifier.clone(),
            package_type: package_request.package_type.clone(),
            lang: package_request.lang.clone(),
            created_at: Some(Utc::now()),
            updated_at: Utc::now(),
            group_id: None, // Use the group_id from IAM service response
            description: Some(package_request.description.clone()),
            organization_id: Some(package_request.organization_id.clone()),
            dependencies_json: Some(serde_json::to_string(&package_request.dependencies).unwrap()),
            repo_origin: package_request.repo_origin.clone(),
            quick_links: package_request.quick_links.clone(),
        };

        diesel::insert_into(package)
            .values(&new_package)
            .returning(id)
            .get_result::<i64>(&mut conn)
            .map_err(|_| {
                status::Custom(
                    Status::InternalServerError,
                    "Error creating package".to_string(),
                )
            })?
    };

    // Update or create the environment in the Package_Env table
    let existing_env = package_env_dsl::package_env
        .filter(package_env_dsl::parent_id.eq(package_id))
        .filter(package_env_dsl::env.eq(&package_request.env))
        .first::<Package_Env>(&mut conn)
        .optional()
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error retrieving package environment".to_string(),
            )
        })?;

    if let Some(_) = existing_env {
        // Update the existing environment
        diesel::update(
            package_env_dsl::package_env
                .filter(package_env_dsl::parent_id.eq(package_id))
                .filter(package_env_dsl::env.eq(&package_request.env)),
        )
        .set(package_env_dsl::version.eq(&package_request.version))
        .execute(&mut conn)
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error updating package environment".to_string(),
            )
        })?;
    } else {
        // Create a new environment
        let new_env = Package_EnvInsertable {
            parent_id: package_id,
            env: package_request.env.clone(),
            version: package_request.version.clone(),
            pipeline_status: None,
        };

        diesel::insert_into(package_env_dsl::package_env)
            .values(&new_env)
            .execute(&mut conn)
            .map_err(|_| {
                status::Custom(
                    Status::InternalServerError,
                    "Error creating package environment".to_string(),
                )
            })?;
    }

    let response = CreateOrUpdatePackageResponse {
        message: "Package created or updated successfully".to_string(),
        package_id,
    };

    Ok(status::Created::new("/package").body(Json(response)))
}

async fn fetch_user_packages(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    env: &str,
    org_id: &str,
) -> Result<Vec<PackageResponse>, status::Custom<String>> {
    use crate::models::schema::schema::package::dsl::*;
    use crate::models::schema::schema::package_env::dsl as package_env_dsl;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    // Get all packages associated with the group_ids
    let results = package
        .inner_join(package_env_dsl::package_env.on(package_env_dsl::parent_id.eq(id)))
        .filter(package_env_dsl::env.eq(env))
        .filter(organization_id.eq(org_id))
        .select((
            package::all_columns(),
            package_env_dsl::version,
            package_env_dsl::pipeline_status,
        ))
        .load::<(Package, String, Option<String>)>(&mut conn)
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error retrieving packages".to_string(),
            )
        })?;

    let package_responses: Vec<PackageResponse> = results
        .into_iter()
        .map(|(p, version, pipeline_status)| PackageResponse {
            identifier: p.identifier,
            package_type: p.package_type,
            lang: p.lang,
            updated_at: p.updated_at,
            description: p.description.unwrap_or(String::from("")),
            organization_id: p.organization_id.unwrap_or(String::from("")),
            dependencies: serde_json::from_str(&p.dependencies_json.unwrap_or(String::from("[]")))
                .unwrap(),
            version, // Include the version from the package_env table
            pipeline_status,
            repo_origin: p.repo_origin,
            quick_links: p.quick_links,
        })
        .collect();

    Ok(package_responses)
}

#[openapi()]
#[get("/packages/<org_id>/<env>")]
pub async fn get_user_packages(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    _claims: APIClaims,
    env: String,
    org_id: String,
) -> Result<Json<Vec<PackageResponse>>, status::Custom<String>> {
    let package_responses = fetch_user_packages(rdb, &env, &org_id).await?;
    Ok(Json(package_responses))
}

#[openapi()]
#[get("/user-land/packages/<org_id>/<env>")]
pub async fn get_user_packages_user_land(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    _claims: Claims,
    env: String,
    org_id: String,
) -> Result<Json<Vec<PackageResponse>>, status::Custom<String>> {
    let package_responses = fetch_user_packages(rdb, &env, &org_id).await?;
    Ok(Json(package_responses))
}

#[openapi()]
#[get("/public/packages/<org_id>/<env>")]
pub async fn get_user_packages_public(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    env: String,
    org_id: String,
) -> Result<Json<Vec<PackageResponse>>, status::Custom<String>> {
    let package_responses = fetch_user_packages(rdb, &env, &org_id).await?;
    Ok(Json(package_responses))
}
async fn fetch_dbschemas_and_tables(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    env: &str,
    org_id: &str,
) -> Result<Vec<GetDbschemaAndTablesResponse>, status::Custom<String>> {
    use crate::models::schema::schema::dbschema::dsl::*;
    use crate::models::schema::schema::dbschema_branch::dsl::*;
    use serde_json::Value;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    let query = dbschema.filter(organization_id.eq(org_id)).into_boxed();

    let results = query.load::<Dbschema>(&mut conn).map_err(|_| {
        status::Custom(
            Status::InternalServerError,
            "Error retrieving dbschemas".to_string(),
        )
    })?;

    let response = results
        .into_iter()
        .map(|db_schema_| {
            // Attempt to get the main branch data
            let branch = dbschema_branch
                .filter(parent_id.eq(db_schema_.id).and(branch_name.eq(env)))
                .first::<Dbschema_Branch>(&mut conn)
                .ok();

            let branch_data = branch.clone().unwrap().data;

            // Use the main branch data if available, otherwise fallback to the db_schema_ data
            let data_to_use = branch_data.as_deref().or(db_schema_.data.as_deref());

            let tables: Vec<String> = match data_to_use {
                Some(data_str) => match serde_json::from_str::<Value>(data_str) {
                    Ok(Value::Array(array)) => array
                        .into_iter()
                        .filter_map(|element| {
                            element
                                .get("data")
                                .and_then(|d| d.get("name"))
                                .and_then(|n| n.as_str().map(|s| s.to_string()))
                        })
                        .collect(),
                    _ => Vec::new(),
                },
                None => Vec::new(),
            };

            GetDbschemaAndTablesResponse {
                id: db_schema_.id,
                name: db_schema_.name,
                description: db_schema_.description,
                version: branch.clone().unwrap().version,
                updated_at: db_schema_.updated_at,
                identifier: db_schema_.identifier,
                organization_id: db_schema_.organization_id.unwrap(),
                tables,
                pipeline_status: branch.clone().unwrap().pipeline_status,
                repo_origin: db_schema_.repo_origin,
                db_type: Some(db_schema_.db_type),
                quick_links: db_schema_.quick_links,
            }
        })
        .collect();

    Ok(response)
}

#[openapi()]
#[get("/dbschemas-and-tables/<org_id>/<env>")]
pub async fn get_dbschemas_and_tables(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    env: String,
    org_id: String,
    _claims: APIClaims,
) -> Result<Json<Vec<GetDbschemaAndTablesResponse>>, status::Custom<String>> {
    let dbschemas = fetch_dbschemas_and_tables(rdb, &env, &org_id).await?;
    Ok(Json(dbschemas))
}

#[openapi()]
#[get("/user-land/dbschemas-and-tables/<org_id>/<env>")]
pub async fn get_dbschemas_and_tables_user_land(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    env: String,
    org_id: String,
    _claims: Claims,
) -> Result<Json<Vec<GetDbschemaAndTablesResponse>>, status::Custom<String>> {
    let dbschemas = fetch_dbschemas_and_tables(rdb, &env, &org_id).await?;
    Ok(Json(dbschemas))
}

#[openapi()]
#[get("/public/dbschemas-and-tables/<org_id>/<env>")]
pub async fn get_dbschemas_and_tables_public(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    env: String,
    org_id: String,
) -> Result<Json<Vec<GetDbschemaAndTablesResponse>>, status::Custom<String>> {
    let dbschemas = fetch_dbschemas_and_tables(rdb, &env, &org_id).await?;
    Ok(Json(dbschemas))
}

#[openapi]
#[put("/update-pipeline-status", format = "json", data = "<status_update>")]
pub async fn update_pipeline_status(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    status_update: Json<PipelineStatusUpdateRequest>,
    _claims: APIClaims,
    notification_config: NotificationService_api_config,
) -> Result<status::NoContent, status::Custom<String>> {
    use crate::models::schema::schema::dbschema::dsl as dbschema_dsl;
    use crate::models::schema::schema::dbschema_branch::dsl as dbschema_branch_dsl;
    use crate::models::schema::schema::organization::dsl as org_dsl;
    use crate::models::schema::schema::package::dsl as package_dsl;
    use crate::models::schema::schema::package_env::dsl as package_env_dsl;
    use crate::models::schema::schema::service::dsl as service_dsl;
    use crate::models::schema::schema::service_envs::dsl as service_envs_dsl;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    let update_type = status_update.update_type.clone();
    let env = status_update.env.clone();
    let status = status_update.status.clone();
    let org_id = status_update.org_id.clone();
    let identifier = status_update.identifier.clone();

    match update_type.as_str() {
        "schema" => {
            // Retrieve the parent ID from the dbschema table
            let parent_id = dbschema_dsl::dbschema
                .filter(dbschema_dsl::identifier.eq(&identifier))
                .filter(dbschema_dsl::organization_id.eq(&org_id))
                .select(dbschema_dsl::id)
                .first::<i64>(&mut conn)
                .map_err(|_| status::Custom(Status::NotFound, "Schema not found".to_string()))?;

            // Update the pipeline status in the dbschema_branch table
            diesel::update(
                dbschema_branch_dsl::dbschema_branch
                    .filter(dbschema_branch_dsl::parent_id.eq(parent_id))
                    .filter(dbschema_branch_dsl::branch_name.eq(&env)),
            )
            .set(dbschema_branch_dsl::pipeline_status.eq(status.clone()))
            .execute(&mut conn)
            .map_err(|_| {
                status::Custom(
                    Status::InternalServerError,
                    "Failed to update schema pipeline status".to_string(),
                )
            })?;
        }
        "package" => {
            // Retrieve the parent ID from the package table
            let parent_id = package_dsl::package
                .filter(package_dsl::identifier.eq(&identifier))
                .filter(package_dsl::organization_id.eq(&org_id))
                .select(package_dsl::id)
                .first::<i64>(&mut conn)
                .map_err(|_| status::Custom(Status::NotFound, "Package not found".to_string()))?;

            // Update the pipeline status in the package_env table
            diesel::update(
                package_env_dsl::package_env
                    .filter(package_env_dsl::parent_id.eq(parent_id))
                    .filter(package_env_dsl::env.eq(&env)),
            )
            .set(package_env_dsl::pipeline_status.eq(status.clone()))
            .execute(&mut conn)
            .map_err(|_| {
                status::Custom(
                    Status::InternalServerError,
                    "Failed to update package pipeline status".to_string(),
                )
            })?;
        }
        "service" => {
            // Retrieve the parent ID from the service table
            let parent_id = service_dsl::service
                .filter(service_dsl::identifier.eq(&identifier))
                .filter(service_dsl::organization_id.eq(&org_id))
                .select(service_dsl::id)
                .first::<i64>(&mut conn)
                .map_err(|e| {
                    println!("{:?}", e);
                    status::Custom(Status::NotFound, "Service not found".to_string())
                })?;

            // Update the pipeline status in the service_envs table
            diesel::update(
                service_envs_dsl::service_envs
                    .filter(service_envs_dsl::parent_id.eq(parent_id))
                    .filter(service_envs_dsl::env.eq(&env)),
            )
            .set(service_envs_dsl::pipeline_status.eq(status.clone()))
            .execute(&mut conn)
            .map_err(|_| {
                status::Custom(
                    Status::InternalServerError,
                    "Failed to update service pipeline status".to_string(),
                )
            })?;
        }
        _ => {
            return Err(status::Custom(
                Status::BadRequest,
                "Invalid update_type provided".to_string(),
            ));
        }
    }

    let group_id = org_dsl::organization
        .filter(org_dsl::slug.eq(&org_id))
        .select(org_dsl::group_id)
        .first::<String>(&mut conn)
        .map_err(|_| status::Custom(Status::NotFound, "Organization not found".to_string()))?;

    let msg = RealtimeMessage {
        topic: "pipeline-update".to_string(),
        payload: serde_json::to_string(
            &json!({"org_id" : org_id , "identifier" : identifier, "status" : status.clone() }),
        )
        .unwrap(),
    };

    match publish_message_to_group(
        &notification_config.0,
        PublishMessageToGroupParams {
            group_id,
            publish_request: PublishRequest {
                message: msg.to_string(),
            },
        },
    )
    .await
    {
        Ok(_) => {}
        Err(e) => {
            return Err(status::Custom(
                Status::InternalServerError,
                format!("Failed to publish message: {:?}", e),
            ));
        }
    }

    Ok(status::NoContent)
}

use crate::models::schema::Organization;
use crate::models::schema::OrganizationInsertable;

use regex::Regex;

fn to_slug(input: &str) -> String {
    // Lowercase the input string
    let mut slug = input.to_lowercase();

    // Remove everything except alphabets a-z and spaces
    let re = Regex::new(r"[^a-z\s]").unwrap();
    slug = re.replace_all(&slug, " ").to_string();

    // Trim the string
    slug = slug.trim().to_string();

    // Replace single and multiple consecutive spaces with a hyphen
    let re_spaces = Regex::new(r"\s+").unwrap();
    slug = re_spaces.replace_all(&slug, "-").to_string();

    slug
}

#[openapi()]
#[post("/organization", data = "<create_request>")]
pub async fn create_organization(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    create_request: Json<CreateOrganizationRequest>,
    claims: Claims,
    iam_service_config: IAMService_config,
) -> Result<status::Created<Json<CreateOrganizationResponse>>, status::Custom<String>> {
    use crate::models::schema::schema::organization::dsl::*;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    let org_slug = to_slug(&create_request.name);

    // Check if organization with the slug already exists
    let existing_org = organization
        .filter(slug.eq(&org_slug))
        .first::<Organization>(&mut conn)
        .optional()
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error checking existing organization".to_string(),
            )
        })?;

    if let Some(_) = existing_org {
        return Err(status::Custom(
            Status::Conflict,
            "Workspace ID is already taken".to_string(),
        ));
    }

    let group_uuid = Uuid::new_v4().to_string();

    match identity_create_group(
        &iam_service_config.0,
        IdentityCreateGroupParams {
            create_group_request: CreateGroupRequest::new(group_uuid.clone()),
        },
    )
    .await
    {
        Ok(response) => {
            let new_organization = OrganizationInsertable {
                slug: org_slug,
                group_id: response.identifier,
                name: Some(create_request.name.clone()),
                is_active: true,
                blocks_positions: None,
                is_public: false,
                infra_repo_origin: None,
                quick_links: None,
                version: None,
            };

            let created_organization: Organization = diesel::insert_into(organization)
                .values(&new_organization)
                .get_result::<Organization>(&mut conn)
                .map_err(|_| {
                    status::Custom(
                        Status::InternalServerError,
                        "Error inserting new organization".to_string(),
                    )
                })?;

            Ok(
                status::Created::new("/organization").body(Json(CreateOrganizationResponse {
                    message: "Organization created successfully".to_string(),
                    id: created_organization.id,
                })),
            )
        }
        Err(e) => {
            println!("{:?}", e);
            Err(status::Custom(
                Status::InternalServerError,
                "Failed to create group in IAM service".to_string(),
            ))
        }
    }
}

#[openapi()]
#[post("/update-block-positions/<org_id>", data = "<block_positions>")]
pub async fn update_block_positions(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    org_id: String,
    block_positions: String,
) -> Result<status::Accepted<String>, status::Custom<String>> {
    use crate::models::schema::schema::organization::dsl::*;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    // Check if the organization exists
    let existing_org = organization
        .filter(slug.eq(org_id.clone()))
        .first::<Organization>(&mut conn)
        .optional()
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error checking organization existence".to_string(),
            )
        })?;

    if let Some(mut org) = existing_org {
        // Update block_positions
        org.blocks_positions = Some(block_positions);

        diesel::update(organization.filter(slug.eq(org_id)))
            .set(blocks_positions.eq(org.blocks_positions))
            .execute(&mut conn)
            .map_err(|_| {
                status::Custom(
                    Status::InternalServerError,
                    "Error updating block positions".to_string(),
                )
            })?;

        Ok(status::Accepted(
            "Block positions updated successfully".to_string(),
        ))
    } else {
        Err(status::Custom(
            Status::NotFound,
            "Organization not found".to_string(),
        ))
    }
}

#[openapi()]
#[get("/get-workspaces")]
pub async fn get_workspaces(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    _claims: Claims,
    groups_owned: GroupOwnerships,
    groups: GroupMemberships,
) -> Result<Json<Vec<WorkspaceSummaryResponse>>, status::Custom<String>> {
    use crate::models::schema::schema::organization::dsl::*;
    println!("Handler invoked");
    let mut conn: diesel::r2d2::PooledConnection<ConnectionManager<PgConnection>> =
        rdb.get().map_err(|_| {
            status::Custom(
                Status::ServiceUnavailable,
                "Failed to get DB connection".to_string(),
            )
        })?;

    let ownerships: Vec<String> = groups_owned.0;
    let memberships: Vec<String> = groups.0;

    let workspaces: Vec<WorkspaceSummaryResponse> = organization
        .filter(group_id.eq_any(memberships))
        .select((
            slug,
            name,
            is_active,
            group_id,
            infra_repo_origin,
            quick_links,
            version,
        ))
        .load::<(
            String,
            Option<String>,
            bool,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
        )>(&mut conn)
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error retrieving workspaces".to_string(),
            )
        })?
        .into_iter()
        .map(
            |(_slug, _name, _is_active, _group_id, _infra_repo_origin, _quick_links, _version)| {
                WorkspaceSummaryResponse {
                    slug: _slug,
                    name: _name,
                    is_active: _is_active,
                    is_admin: ownerships.contains(&_group_id),
                    group_id: _group_id,
                    infra_repo_origin: _infra_repo_origin,
                    quick_links: _quick_links,
                    version: _version,
                }
            },
        )
        .collect();

    Ok(Json(workspaces))
}

async fn fetch_workspace(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    org_id: &str,
    is_admin: Option<&Vec<String>>,
) -> Result<WorkspaceDetailResponse, status::Custom<String>> {
    use crate::models::schema::schema::organization::dsl::*;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    let workspace = organization
        .filter(slug.eq(org_id))
        .select((name, blocks_positions, is_active, group_id))
        .first::<(Option<String>, Option<String>, bool, String)>(&mut conn)
        .optional()
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error retrieving workspace".to_string(),
            )
        })?;

    if let Some((_name, _block_positions, _is_active, _group_id)) = workspace {
        let is_admin_flag = is_admin.map_or(false, |ownerships| ownerships.contains(&_group_id));
        Ok(WorkspaceDetailResponse {
            name: _name,
            block_positions: _block_positions,
            is_active: _is_active,
            is_admin: is_admin_flag,
        })
    } else {
        Err(status::Custom(
            Status::NotFound,
            "Workspace not found".to_string(),
        ))
    }
}

#[openapi()]
#[get("/get-workspace/<org_id>")]
pub async fn get_workspace(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    org_id: String,
    groups_owned: GroupOwnerships,
) -> Result<Json<WorkspaceDetailResponse>, status::Custom<String>> {
    let workspace_detail = fetch_workspace(rdb, &org_id, Some(&groups_owned.0)).await?;
    Ok(Json(workspace_detail))
}

#[openapi()]
#[get("/public/get-workspace/<org_id>")]
pub async fn get_workspace_public(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    org_id: String,
) -> Result<Json<WorkspaceDetailResponse>, status::Custom<String>> {
    let workspace_detail = fetch_workspace(rdb, &org_id, None).await?;
    Ok(Json(workspace_detail))
}

#[openapi()]
#[get("/get-workspace-details/<org_id>")]
pub async fn get_workspace_details(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    groups_owned: GroupOwnerships,
    org_id: String,
) -> Result<Json<WorkspaceSummaryResponse>, status::Custom<String>> {
    use crate::models::schema::schema::organization::dsl::*;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    let ownerships: Vec<String> = groups_owned.0;

    let workspace = organization
        .filter(slug.eq(&org_id))
        .filter(group_id.eq_any(&ownerships))
        .select((
            slug,
            name,
            is_active,
            group_id,
            infra_repo_origin,
            quick_links,
            version,
        ))
        .first::<(
            String,
            Option<String>,
            bool,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
        )>(&mut conn)
        .optional()
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error retrieving workspace".to_string(),
            )
        })?;

    match workspace {
        Some((_slug, _name, _is_active, _group_id, _infra_repo_origin, _quick_links, _version)) => {
            Ok(Json(WorkspaceSummaryResponse {
                slug: _slug,
                name: _name,
                is_active: _is_active,
                group_id: _group_id,
                is_admin: true,
                infra_repo_origin: _infra_repo_origin,
                quick_links: _quick_links,
                version: _version,
            }))
        }
        None => Err(status::Custom(
            Status::NotFound,
            "Workspace not found".to_string(),
        )),
    }
}

use crate::routes::MessageResponse;

#[openapi()]
#[delete("/manage-workspace/<org_id>/delete")]
pub async fn delete_workspace(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    org_id: String,
    groups_owned: GroupOwnerships,
) -> Result<Json<MessageResponse>, rocket::http::Status> {
    use crate::models::schema::schema::organization::dsl::*;

    let mut conn = rdb.get().map_err(|_| Status::ServiceUnavailable)?;

    let ownerships: Vec<String> = groups_owned.0;

    // Fetch the workspace to ensure it exists and check if the user has permission to delete it
    let workspace = organization
        .filter(slug.eq(org_id.clone()))
        .select(group_id)
        .first::<String>(&mut conn)
        .optional()
        .map_err(|_| Status::InternalServerError)?;

    if let Some(_group_id) = workspace {
        // Check if the user owns the group to which the workspace belongs
        if ownerships.contains(&_group_id) {
            // Proceed to delete the workspace
            diesel::delete(organization.filter(slug.eq(org_id)))
                .execute(&mut conn)
                .map_err(|_| Status::InternalServerError)?;

            Ok(Json(MessageResponse {
                message: "Workspace successfully deleted".to_string(),
            }))
        } else {
            Err(Status::Forbidden)
        }
    } else {
        Err(Status::NotFound)
    }
}

#[openapi()]
#[get("/version/<org_id>/<package_name>")]
pub async fn get_package_version_plain_text(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    org_id: String,
    package_name: String,
) -> Result<String, status::Custom<String>> {
    use crate::models::schema::schema::package::dsl::*;
    use crate::models::schema::schema::package_env::dsl as package_env_dsl;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    // Fetch the latest version of the package for the given org_id and package_name
    let version_result = package
        .inner_join(package_env_dsl::package_env.on(package_env_dsl::parent_id.eq(id)))
        .filter(organization_id.eq(org_id))
        .filter(identifier.eq(package_name))
        .order_by(package_env_dsl::version.desc()) // Order by version descending to get the latest version
        .select(package_env_dsl::version)
        .first::<String>(&mut conn)
        .map_err(|_| {
            status::Custom(Status::NotFound, "Package or version not found".to_string())
        })?;

    // Return the version as plain text
    Ok(version_result)
}

#[openapi()]
#[get("/version-details/<org_id>/<package_name>")]
pub async fn get_package_version(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    org_id: String,
    package_name: String,
) -> Result<Json<VersionResponse>, status::Custom<String>> {
    use crate::models::schema::schema::package::dsl::*;
    use crate::models::schema::schema::package_env::dsl as package_env_dsl;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    // Fetch the latest version of the package for the given org_id and package_name
    let version_result = package
        .inner_join(package_env_dsl::package_env.on(package_env_dsl::parent_id.eq(id)))
        .filter(organization_id.eq(org_id))
        .filter(identifier.eq(package_name))
        .order_by(package_env_dsl::version.desc()) // Order by version descending to get the latest version
        .select(package_env_dsl::version)
        .first::<String>(&mut conn)
        .map_err(|_| {
            status::Custom(Status::NotFound, "Package or version not found".to_string())
        })?;

    // Return the version as plain text
    Ok(Json(VersionResponse {
        version: version_result,
    }))
}

#[openapi()]
#[put(
    "/update-db-pipeline/<org_id>/<schema_name>/<branch_name>",
    data = "<update_db_pipeline_request>"
)]
pub fn update_db_pipeline(
    org_id: String,
    schema_name: String,
    branch_name: String,
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    update_db_pipeline_request: Json<UpdateDbPipelineRequest>,
    _claims: APIClaims,
) -> Result<Json<Dbschema>, status::Custom<String>> {
    use crate::models::schema::schema::dbschema::dsl::*;
    use crate::models::schema::schema::dbschema_branch::dsl as dbschema_branch_dsl;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    let updated_dbschema = dbschema
        .filter(name.eq(schema_name.clone()))
        .filter(organization_id.eq(org_id.clone()))
        .first::<Dbschema>(&mut conn)
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error retrieving updated dbschema".to_string(),
            )
        })?;

    diesel::update(
        dbschema_branch_dsl::dbschema_branch
            .filter(dbschema_branch_dsl::parent_id.eq(updated_dbschema.id))
            .filter(dbschema_branch_dsl::branch_name.eq(&branch_name)),
    )
    .set(dbschema_branch_dsl::pipeline_status.eq(update_db_pipeline_request.status.clone()))
    .execute(&mut conn)
    .map_err(|_| {
        status::Custom(
            Status::InternalServerError,
            "Failed to update schema pipeline".to_string(),
        )
    })?;

    Ok(Json(updated_dbschema))
}

#[openapi]
#[get("/templates")]
pub fn get_all_templates(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    claims: APIClaims,
) -> Result<Json<Vec<Templates>>, rocket::http::Status> {
    use crate::models::schema::schema::templates::dsl::*;

    let mut conn = rdb
        .get()
        .map_err(|_| rocket::http::Status::ServiceUnavailable)?;

    // Query all templates
    let template_list = templates
        .load::<Templates>(&mut conn)
        .map_err(|_| rocket::http::Status::InternalServerError)?;

    Ok(Json(template_list))
}

#[openapi]
#[post("/create-snapshot", data = "<create_snapshot_request>")]
pub async fn create_snapshot(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    claims: APIClaims,
    create_snapshot_request: Json<CreateSnapshotRequest>,
) -> Result<Json<MessageResponse>, status::Custom<String>> {
    use crate::models::schema::schema::organization::dsl as org_dsl;
    use crate::models::schema::schema::snapshots::dsl::*;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Database unavailable".to_string(),
        )
    })?;

    let new_snapshot = SnapshotsInsertable {
        version: create_snapshot_request.version.clone(),
        created_at: Utc::now(), // Ensure NaiveDateTime is used
        updated_at: Utc::now(),
        organization_id: create_snapshot_request.org_id.clone(),
    };

    diesel::insert_into(snapshots)
        .values(&new_snapshot)
        .execute(&mut conn)
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Failed to create snapshot".to_string(),
            )
        })?;

    let updated_rows = diesel::update(
        org_dsl::organization.filter(org_dsl::slug.eq(create_snapshot_request.org_id.clone())),
    )
    .set((
        org_dsl::infra_repo_origin.eq(create_snapshot_request.infra_repo_origin.clone()),
        org_dsl::quick_links.eq(create_snapshot_request.quick_links.clone()),
        org_dsl::version.eq(create_snapshot_request.version.clone()),
    )) // Assuming this field is in your organization table
    .execute(&mut conn)
    .map_err(|_| {
        status::Custom(
            Status::InternalServerError,
            "Failed to update organization".to_string(),
        )
    })?;

    if updated_rows == 0 {
        return Err(status::Custom(
            Status::NotFound,
            "Organization not found".to_string(),
        ));
    }

    Ok(Json(MessageResponse {
        message: "Snapshot record created and organization updated".to_string(),
    }))
}

#[openapi]
#[get("/get-snapshots/<org_id>")]
pub async fn get_snapshots(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    org_id: String,
) -> Result<Json<Vec<SnapshotsResponse>>, rocket::http::Status> {
    use crate::models::schema::schema::snapshots::dsl::*;

    let mut conn = rdb
        .get()
        .map_err(|_| rocket::http::Status::ServiceUnavailable)?;

    let db_snapshots = snapshots
        .filter(organization_id.eq(org_id))
        .load::<Snapshots>(&mut conn)
        .map_err(|_| rocket::http::Status::InternalServerError)?;

    // Map only the version and created_at fields
    let response_snapshots: Vec<SnapshotsResponse> = db_snapshots
        .into_iter()
        .map(|snapshot| SnapshotsResponse {
            version: snapshot.version,
            created_at: snapshot.created_at,
        })
        .collect();

    Ok(Json(response_snapshots))
}
