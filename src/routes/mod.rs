use crate::middlewares::groups::GroupMemberships;
use crate::middlewares::jwt::Claims;
use crate::models::response::MessageResponse;
use diesel::r2d2;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use r2d2_redis::RedisConnectionManager;

use crate::middlewares::IAMService_config::IAMService_config;
use rocket::serde::json::Json;
use rocket::State;
use rocket_okapi::openapi;
use IAMService::apis::default_api::{
    identity_get_group_memberships, routes_index, routes_protected_route,
};
use IAMService::models::MessageResponse as OtherMessageResponse;
/// This is a description. <br />You can do simple html <br /> like <b>this<b/>
#[openapi()]
#[get("/")]
pub fn index(
    rdb: &State<r2d2::Pool<ConnectionManager<PgConnection>>>,
    cache: &State<Pool<RedisConnectionManager>>,
) -> Json<MessageResponse> {
    Json(MessageResponse {
        message: "Ok".to_string(),
    })
}

// async fn get_healthcheck(
//     openapi_configuration: &Configuration,
// ) -> Result<OtherMessageResponse, Box<dyn std::error::Error>> {
//     match routes_index(openapi_configuration).await {
//         Ok(response) => Ok(response),
//         Err(e) => Err(Box::new(e)),
//     }
// }

#[openapi()]
#[get("/route2")]
pub async fn route2(
    rdb: &State<r2d2::Pool<ConnectionManager<PgConnection>>>,
    cache: &State<Pool<RedisConnectionManager>>,
    claims: Claims,
    iam_service_config: IAMService_config,
    groups: GroupMemberships,
) -> Json<MessageResponse> {
    match routes_protected_route(&iam_service_config.0).await {
        Ok(status) => println!("{:?}", status),
        Err(e) => {
            // Handle the error appropriately
            println!("{:?}", e);
            return Json(MessageResponse {
                message: "Health check failed".to_string(),
            });
        }
    }

    println!("{:?}", groups);

    // match identity_get_group_memberships(&iam_service_config.0).await {
    //     Ok(groups) => {
    //         println!("{:?}", groups)
    //     }
    //     Err(_) => {}
    // }

    Json(MessageResponse {
        message: format!("Hello, {}! This is a protected route.", claims.user_id),
    })
}
