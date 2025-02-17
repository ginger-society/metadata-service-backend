#[macro_use]
extern crate rocket;
use rocket::Rocket;

use crate::routes::metadata;
use db::redis::create_redis_pool;
use dotenv::dotenv;
use rocket::Build;
use rocket_okapi::openapi_get_routes;
use rocket_okapi::swagger_ui::{make_swagger_ui, SwaggerUIConfig};
use rocket_prometheus::PrometheusMetrics;
use std::env;
mod db;
mod fairings;
mod middlewares;
mod models;
mod routes;

#[launch]
fn rocket() -> Rocket<Build> {
    dotenv().ok();
    let prometheus = PrometheusMetrics::new();

    let mut server = rocket::build()
        .manage(db::connect_rdb())
        .attach(fairings::cors::CORS)
        .attach(prometheus.clone())
        .mount(
            "/metadata/",
            openapi_get_routes![
                routes::index,
                metadata::create_dbschema,
                metadata::get_dbschemas,
                metadata::update_dbschema,
                metadata::get_dbschema_by_id,
                metadata::get_dbschema_by_id_userland,
                metadata::create_dbschema_branch,
                metadata::update_dbschema_branch,
                metadata::update_or_create_service,
                metadata::get_services_and_envs,
                metadata::get_service_and_env_by_id,
                metadata::get_service_by_id,
                metadata::create_or_update_package,
                metadata::get_user_packages,
                metadata::get_dbschemas_and_tables,
                metadata::update_pipeline_status,
                metadata::create_organization,
                metadata::update_block_positions,
                metadata::get_workspaces,
                metadata::get_workspace,
                metadata::get_workspace_details,
                metadata::delete_workspace,
                metadata::get_services_and_envs_user_land,
                metadata::get_service_and_env_by_id_user_land,
                metadata::get_current_workspace,
                metadata::get_package_version_plain_text,
                metadata::update_db_pipeline,
                metadata::get_services_and_envs_public,
                metadata::get_workspace_public,
                metadata::get_dbschemas_and_tables_public,
                metadata::get_user_packages_public,
                metadata::get_service_and_env_by_id_public,
                metadata::get_dbschema_by_id_public,
                metadata::get_all_templates,
                metadata::get_user_packages_user_land,
                metadata::get_dbschemas_and_tables_user_land,
                metadata::create_snapshot,
                metadata::get_snapshots,
                metadata::get_package_version
            ],
        )
        .mount(
            "/metadata/api-docs",
            make_swagger_ui(&SwaggerUIConfig {
                url: "../openapi.json".to_owned(),
                ..Default::default()
            }),
        )
        .mount("/metadata/metrics", prometheus);

    match env::var("MONGO_URI") {
        Ok(mongo_uri) => match env::var("MONGO_DB_NAME") {
            Ok(mongo_db_name) => {
                println!("Attempting to connect to mongo");
                server = server.manage(db::connect_mongo(mongo_uri, mongo_db_name))
            }
            Err(_) => {
                println!("Not connecting to mongo, missing MONGO_DB_NAME")
            }
        },
        Err(_) => println!("Not connecting to mongo, missing MONGO_URI"),
    };

    match env::var("REDIS_URI") {
        Ok(redis_uri) => {
            println!("Attempting to connect to redis");
            server = server.manage(create_redis_pool(&redis_uri))
        }
        Err(_) => println!("Not connecting to redis"),
    }

    server
}

// Unit testings
#[cfg(test)]
mod tests;
