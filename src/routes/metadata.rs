use crate::middlewares::api_jwt::APIClaims;
use crate::middlewares::groups::GroupMemberships;
use crate::middlewares::groups_owned;
use crate::middlewares::groups_owned::GroupOwnerships;
use crate::middlewares::jwt::Claims;
use crate::middlewares::IAMService_config::IAMService_config;
use crate::models::schema::schema::dbschema::organization_id;
use crate::models::schema::{
    Dbschema, DbschemaInsertable, Dbschema_Branch, Dbschema_BranchInsertable, Package,
    PackageInsertable, Package_Env, Package_EnvInsertable, Service, ServiceInsertable,
    Service_Envs, Service_EnvsInsertable,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use winnow::Parser;
use IAMService::apis::default_api::{identity_create_group, IdentityCreateGroupParams};
use IAMService::models::CreateGroupRequest;
#[derive(Deserialize, Serialize, JsonSchema)]
pub struct CreateDbschemaRequest {
    pub name: String,
    pub description: Option<String>,
    pub data: Option<String>,
    pub organisation_id: String,
}

#[derive(Serialize, JsonSchema)]
pub struct CreateDbschemaResponse {
    pub message: String,
    pub id: i64,
}

#[derive(Serialize, JsonSchema)]
pub struct GetDbschemaResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub updated_at: chrono::DateTime<Utc>,
    pub identifier: Option<String>,
    pub organization_id: String,
}

#[derive(Serialize, JsonSchema)]
pub struct GetDbschemaAndTablesResponse {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub updated_at: chrono::DateTime<Utc>,
    pub identifier: Option<String>,
    pub organization_id: String,
    pub tables: Vec<String>,
    pub pipeline_status: Option<String>,
    pub repo_origin: Option<String>,
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
}

#[derive(Deserialize, JsonSchema, Serialize)]
pub struct UpdateDbschemaRequest {
    pub name: String,
    pub description: Option<String>,
    pub organisation_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateDbschemaBranchRequest {
    pub branch_name: String,
    pub data: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CreateDbschemaBranchResponse {
    pub message: String,
    pub id: i64,
}

#[openapi()]
#[post("/dbschema", data = "<create_request>")]
pub async fn create_dbschema(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    create_request: Json<CreateDbschemaRequest>,
    claims: Claims,
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

    match identity_create_group(
        &iam_service_config.0,
        IdentityCreateGroupParams {
            create_group_request: CreateGroupRequest::new(group_uuid.clone()),
        },
    )
    .await
    {
        Ok(response) => {
            let dbschema_uuid = Uuid::new_v4().to_string();

            let new_dbschema = DbschemaInsertable {
                name: create_request.name.clone(),
                description: create_request.description.clone(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                data: create_request.data.clone(),
                group_id: response.identifier,
                identifier: Some(dbschema_uuid),
                organization_id: Some(create_request.organisation_id.clone()),
                repo_origin: None,
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
                version: Some("0.0.0".to_string()),
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
                    }))
                })
        }
        Err(_) => todo!(),
    }
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
#[put("/dbschema/<schema_id>", data = "<update_request>")]
pub fn update_dbschema(
    schema_id: i64,
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    update_request: Json<UpdateDbschemaRequest>,
) -> Result<Json<Dbschema>, status::Custom<String>> {
    use crate::models::schema::schema::dbschema::dsl::*;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    let updated_rows = diesel::update(dbschema.filter(id.eq(schema_id)))
        .set((
            name.eq(update_request.name.clone()),
            description.eq(update_request.description.clone()),
            organization_id.eq(update_request.organisation_id.clone()),
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
        .find(schema_id)
        .first::<Dbschema>(&mut conn)
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error retrieving updated dbschema".to_string(),
            )
        })?;

    Ok(Json(updated_dbschema))
}

#[openapi()]
#[get("/dbschemas-branch/<schema_id>?<branch>")]
pub fn get_dbschema_by_id(
    schema_id: String,
    branch: Option<String>,
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    _claims: Claims,
) -> Result<Json<GetDbschemaByIdResponse>, status::Custom<String>> {
    use crate::models::schema::schema::dbschema::dsl::*;
    use crate::models::schema::schema::dbschema_branch::dsl::*;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    let result_dbschema = dbschema
        .filter(identifier.eq(&schema_id))
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
    };

    if let Some(branch) = branch {
        let result_branch: Dbschema_Branch = dbschema_branch
            .filter(
                parent_id
                    .eq(result_dbschema.id)
                    .and(branch_name.eq(&branch)),
            )
            .first::<Dbschema_Branch>(&mut conn)
            .map_err(|_| {
                status::Custom(
                    Status::NotFound,
                    format!(
                        "Dbschema branch with parent_id {} and branch_name {} not found",
                        schema_id, branch
                    ),
                )
            })?;

        response.version = result_branch.version.clone();
        response.data = result_branch.data;
        response.branch_id = Some(result_branch.id);
    }

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

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UpdateDbschemaBranchRequest {
    pub branch_name: String,
    pub data: Option<String>,
    pub merged: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct UpdateDbschemaBranchResponse {
    pub message: String,
    pub id: i64,
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
) -> Result<Json<UpdateDbschemaBranchResponse>, status::Custom<String>> {
    use crate::models::schema::schema::dbschema::dsl::*;
    use crate::models::schema::schema::dbschema_branch::dsl as branch_dsl;

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

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UpdateServiceRequest {
    pub identifier: String,
    pub env: String,
    pub base_url: String,
    pub spec: String,
    pub dependencies: Vec<String>,
    pub tables: Vec<String>,
    pub db_schema_id: Option<String>,
    pub service_type: Option<String>,
    pub version: Option<String>,
    pub lang: Option<String>,
    pub description: String,
    pub organization_id: String,
    pub repo_origin: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct UpdateServiceResponse {
    pub message: String,
    pub service_id: i64,
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
        // Create a new group in IAM service
        let group_uuid = Uuid::new_v4().to_string();

        let group_response = identity_create_group(
            &iam_service_config.0,
            IdentityCreateGroupParams {
                create_group_request: CreateGroupRequest::new(group_uuid.clone()),
            },
        )
        .await
        .map_err(|e| {
            println!("{:?}", e);
            status::Custom(Status::InternalServerError, e.to_string())
        })?;

        // Create a new service
        let new_service = ServiceInsertable {
            identifier: service_identifier.clone(),
            group_id: group_response.identifier, // Use the group_id from IAM service response
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

#[derive(Debug, Serialize, JsonSchema)]
pub struct ServicesEnvResponse {
    pub spec: String,
    pub base_url: String,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct ServicesEnvTrimmedResponse {
    pub env_key: String,
    pub base_url: String,
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
    pub service_type: Option<String>,
    pub lang: Option<String>,
    pub description: String,
    pub organization_id: String,
    pub repo_origin: Option<String>,
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
    use crate::models::schema::schema::service::dsl::*;
    use crate::models::schema::schema::service_envs::dsl::*;

    let mut conn = rdb
        .get()
        .map_err(|_| rocket::http::Status::ServiceUnavailable)?;

    let page_number = page_number
        .as_deref()
        .unwrap_or("1")
        .parse::<i64>()
        .unwrap_or(1);
    let page_size = page_size
        .as_deref()
        .unwrap_or("10")
        .parse::<i64>()
        .unwrap_or(10);

    let offset = (page_number - 1) * page_size;

    // Query services and their associated environments for the user's groups
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
                service_type: Some(s.service_type),
                lang: s.lang,
                organization_id: s.organization_id.unwrap_or(String::from("")),
                description: s.description.unwrap_or(String::from("")),
                repo_origin: s.repo_origin,
            })
        })
        .collect::<Result<Vec<ServicesTrimmedResponse>, rocket::http::Status>>()?;

    Ok(Json(services_with_envs))
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
    use crate::models::schema::schema::service::dsl::*;
    use crate::models::schema::schema::service_envs::dsl::*;

    let mut conn = rdb
        .get()
        .map_err(|_| rocket::http::Status::ServiceUnavailable)?;

    let page_number = page_number
        .as_deref()
        .unwrap_or("1")
        .parse::<i64>()
        .unwrap_or(1);
    let page_size = page_size
        .as_deref()
        .unwrap_or("10")
        .parse::<i64>()
        .unwrap_or(10);

    let offset = (page_number - 1) * page_size;

    // Query services and their associated environments for the user's groups
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
                service_type: Some(s.service_type),
                lang: s.lang,
                organization_id: s.organization_id.unwrap_or(String::from("")),
                description: s.description.unwrap_or(String::from("")),
                repo_origin: s.repo_origin,
            })
        })
        .collect::<Result<Vec<ServicesTrimmedResponse>, rocket::http::Status>>()?;

    Ok(Json(services_with_envs))
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
    use crate::models::schema::schema::service::dsl::*;
    use crate::models::schema::schema::service_envs::dsl as service_envs_dsl;

    let mut conn = rdb
        .get()
        .map_err(|_| rocket::http::Status::ServiceUnavailable)?;

    // Query the service by ID and ensure it belongs to one of the user's groups
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

    Ok(Json(env_response))
}

#[derive(Serialize, JsonSchema, Debug)]
pub struct ServiceResponse {
    pub id: i64,
    pub identifier: String,
    pub group_id: String,
    pub db_schema_id: String,
    pub dependencies: Vec<String>,
    pub tables: Vec<String>,
    pub description: String,
    pub organization_id: String,
}

#[openapi]
#[get("/services/<service_identifier>")]
pub fn get_service_by_id(
    service_identifier: String,
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    claims: Claims,
    groups: GroupMemberships,
) -> Result<Json<ServiceResponse>, rocket::http::Status> {
    use crate::models::schema::schema::service::dsl::*;

    let mut conn = rdb
        .get()
        .map_err(|_| rocket::http::Status::ServiceUnavailable)?;

    // Extract group IDs from the `groups` parameter
    let group_ids: Vec<String> = groups.0;

    // Query the service by ID and ensure it belongs to one of the user's groups
    let service_item = service
        .filter(
            identifier
                .eq(service_identifier)
                .and(group_id.eq_any(&group_ids)),
        )
        .first::<Service>(&mut conn)
        .map_err(|_| rocket::http::Status::NotFound)?;

    let dependencies: Vec<String> =
        serde_json::from_str(&service_item.dependencies_json.unwrap()).unwrap();

    let tables: Vec<String> = serde_json::from_str(&service_item.tables_json.unwrap()).unwrap();

    let response = ServiceResponse {
        id: service_item.id,
        identifier: service_item.identifier,
        group_id: service_item.group_id,
        db_schema_id: service_item.db_schema_id.unwrap(),
        dependencies: dependencies,
        tables: tables,
        organization_id: service_item.organization_id.unwrap_or(String::from("")),
        description: service_item.description.unwrap_or(String::from("")),
    };

    Ok(Json(response))
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
}

#[derive(Serialize, JsonSchema)]
pub struct CreateOrUpdatePackageResponse {
    pub message: String,
    pub package_id: i64,
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
        // Create a new group in IAM service
        let group_uuid = Uuid::new_v4().to_string();

        let group_response = identity_create_group(
            &iam_service_config.0,
            IdentityCreateGroupParams {
                create_group_request: CreateGroupRequest::new(group_uuid.clone()),
            },
        )
        .await
        .map_err(|e| {
            println!("{:?}", e);
            status::Custom(Status::InternalServerError, e.to_string())
        })?;

        // Create a new package
        let new_package = PackageInsertable {
            identifier: package_identifier.clone(),
            package_type: package_request.package_type.clone(),
            lang: package_request.lang.clone(),
            created_at: Some(Utc::now()),
            updated_at: Utc::now(),
            group_id: Some(group_response.identifier), // Use the group_id from IAM service response
            description: Some(package_request.description.clone()),
            organization_id: Some(package_request.organization_id.clone()),
            dependencies_json: Some(serde_json::to_string(&package_request.dependencies).unwrap()),
            repo_origin: package_request.repo_origin.clone(),
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
}

#[openapi()]
#[get("/packages/<org_id>/<env>")]
pub async fn get_user_packages(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    _claims: Claims,
    env: String,
    org_id: String,
    groups: GroupMemberships,
) -> Result<Json<Vec<PackageResponse>>, status::Custom<String>> {
    use crate::models::schema::schema::package::dsl::*;
    use crate::models::schema::schema::package_env::dsl as package_env_dsl;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    // Get all group_ids the user is a member of
    let memberships: Vec<String> = groups.0;

    // Get all packages associated with those group_ids
    let results = package
        .inner_join(package_env_dsl::package_env.on(package_env_dsl::parent_id.eq(id)))
        .filter(group_id.eq_any(memberships))
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
        })
        .collect();

    Ok(Json(package_responses))
}

#[openapi()]
#[get("/dbschemas-and-tables/<org_id>/<env>")]
pub fn get_dbschemas_and_tables(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    groups: GroupMemberships,
    env: String,
    org_id: String,
    _claims: Claims,
) -> Result<Json<Vec<GetDbschemaAndTablesResponse>>, status::Custom<String>> {
    use crate::models::schema::schema::dbschema::dsl::*;
    use crate::models::schema::schema::dbschema_branch::dsl::*;
    use serde_json::Value;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    let memberships: Vec<String> = groups.0;

    let query = dbschema
        .filter(organization_id.eq(org_id))
        .filter(group_id.eq_any(memberships))
        .into_boxed();

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
                .filter(parent_id.eq(db_schema_.id).and(branch_name.eq(env.clone())))
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
            }
        })
        .collect();

    Ok(Json(response))
}

#[derive(Deserialize, JsonSchema)]
pub struct PipelineStatusUpdateRequest {
    pub env: String,
    pub status: String,      // can be running, failed, passing, dormant
    pub update_type: String, // can be schema, package, service
    pub org_id: String,      // organization ID to filter
    pub identifier: String,  // identifier to filter
}
#[openapi]
#[put("/update-pipeline-status", format = "json", data = "<status_update>")]
pub async fn update_pipeline_status(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    status_update: Json<PipelineStatusUpdateRequest>,
    _claims: APIClaims,
) -> Result<status::NoContent, status::Custom<String>> {
    use crate::models::schema::schema::dbschema::dsl as dbschema_dsl;
    use crate::models::schema::schema::dbschema_branch::dsl as dbschema_branch_dsl;
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
            .set(dbschema_branch_dsl::pipeline_status.eq(status))
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
            .set(package_env_dsl::pipeline_status.eq(status))
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
            .set(service_envs_dsl::pipeline_status.eq(status))
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

    Ok(status::NoContent)
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateOrganizationRequest {
    pub name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CreateOrganizationResponse {
    pub message: String,
    pub id: i64,
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

#[derive(Serialize, JsonSchema)]
pub struct WorkspaceSummary {
    slug: String,
    name: Option<String>,
    is_active: bool,
    is_admin: bool,
    group_id: String,
}

#[openapi()]
#[get("/get-workspaces")]
pub async fn get_workspaces(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    _claims: Claims,
    groups_owned: GroupOwnerships,
    groups: GroupMemberships,
) -> Result<Json<Vec<WorkspaceSummary>>, status::Custom<String>> {
    use crate::models::schema::schema::organization::dsl::*;

    let mut conn: diesel::r2d2::PooledConnection<ConnectionManager<PgConnection>> =
        rdb.get().map_err(|_| {
            status::Custom(
                Status::ServiceUnavailable,
                "Failed to get DB connection".to_string(),
            )
        })?;

    let ownerships: Vec<String> = groups_owned.0;
    let memberships: Vec<String> = groups.0;

    let workspaces: Vec<WorkspaceSummary> = organization
        .filter(group_id.eq_any(memberships))
        .select((slug, name, is_active, group_id))
        .load::<(String, Option<String>, bool, String)>(&mut conn)
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error retrieving workspaces".to_string(),
            )
        })?
        .into_iter()
        .map(|(_slug, _name, _is_active, _group_id)| WorkspaceSummary {
            slug: _slug,
            name: _name,
            is_active: _is_active,
            is_admin: ownerships.contains(&_group_id),
            group_id: _group_id,
        })
        .collect();

    Ok(Json(workspaces))
}

#[derive(Serialize, JsonSchema)]
pub struct WorkspaceDetail {
    name: Option<String>,
    block_positions: Option<String>,
    is_active: bool,
    is_admin: bool,
}

#[openapi()]
#[get("/get-workspace/<org_id>")]
pub async fn get_workspace(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    org_id: String,
    groups_owned: GroupOwnerships,
) -> Result<Json<WorkspaceDetail>, status::Custom<String>> {
    use crate::models::schema::schema::organization::dsl::*;

    let mut conn = rdb.get().map_err(|_| {
        status::Custom(
            Status::ServiceUnavailable,
            "Failed to get DB connection".to_string(),
        )
    })?;

    let ownerships: Vec<String> = groups_owned.0;

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
        Ok(Json(WorkspaceDetail {
            name: _name,
            block_positions: _block_positions,
            is_active: _is_active,
            is_admin: ownerships.contains(&_group_id),
        }))
    } else {
        Err(status::Custom(
            Status::NotFound,
            "Workspace not found".to_string(),
        ))
    }
}
#[openapi()]
#[get("/get-workspace-details/<org_id>")]
pub async fn get_workspace_details(
    rdb: &State<Pool<ConnectionManager<PgConnection>>>,
    groups_owned: GroupOwnerships,
    org_id: String,
) -> Result<Json<WorkspaceSummary>, status::Custom<String>> {
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
        .select((slug, name, is_active, group_id))
        .first::<(String, Option<String>, bool, String)>(&mut conn)
        .optional()
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "Error retrieving workspace".to_string(),
            )
        })?;

    match workspace {
        Some((_slug, _name, _is_active, _group_id)) => Ok(Json(WorkspaceSummary {
            slug: _slug,
            name: _name,
            is_active: _is_active,
            group_id: _group_id,
            is_admin: true,
        })),
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
        .select((group_id))
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
