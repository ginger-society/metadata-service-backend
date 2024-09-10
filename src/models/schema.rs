#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
use diesel::Insertable;
use chrono::NaiveDate;
use diesel::{deserialize::Queryable, Selectable};
use schemars::JsonSchema;
use serde::Serialize;
use chrono::offset::Utc;
use chrono::DateTime;
use diesel::Identifiable;
use diesel::Associations;
use rocket::serde::Deserialize;

pub mod schema {
    use diesel::table;

    table! {
        dbschema (id) {
            #[max_length = 100]
            name ->Varchar,
            #[max_length = 500]
            description ->Nullable<Varchar>,
            created_at ->Timestamptz,
            updated_at ->Timestamptz,
            #[max_length = 10000]
            data ->Nullable<Varchar>,
            #[max_length = 500]
            group_id ->Nullable<Varchar>,
            #[max_length = 100]
            identifier ->Nullable<Varchar>,
            #[max_length = 100]
            organization_id ->Nullable<Varchar>,
            #[max_length = 200]
            repo_origin ->Nullable<Varchar>,
            db_type ->Varchar,
            id ->BigInt,
            
        }
    }
    
    table! {
        dbschema_branch (id) {
            parent_id ->BigInt,
            #[max_length = 100]
            branch_name ->Varchar,
            #[max_length = 10000]
            data ->Nullable<Varchar>,
            created_at ->Timestamptz,
            updated_at ->Timestamptz,
            #[max_length = 50]
            version ->Nullable<Varchar>,
            pipeline_status ->Nullable<Varchar>,
            id ->BigInt,
            
        }
    }
    
    table! {
        templates (id) {
            #[max_length = 100]
            short_name ->Varchar,
            #[max_length = 600]
            description ->Varchar,
            #[max_length = 100]
            repo_link ->Varchar,
            #[max_length = 40]
            identifier ->Varchar,
            id ->BigInt,
            
        }
    }
    
    table! {
        service (id) {
            #[max_length = 50]
            identifier ->Varchar,
            #[max_length = 100]
            group_id ->Nullable<Varchar>,
            #[max_length = 100]
            db_schema_id ->Nullable<Varchar>,
            #[max_length = 1000]
            tables_json ->Nullable<Varchar>,
            #[max_length = 1000]
            dependencies_json ->Nullable<Varchar>,
            service_type ->Varchar,
            #[max_length = 50]
            lang ->Nullable<Varchar>,
            #[max_length = 2000]
            description ->Nullable<Varchar>,
            #[max_length = 100]
            organization_id ->Nullable<Varchar>,
            #[max_length = 200]
            repo_origin ->Nullable<Varchar>,
            #[max_length = 250]
            cache_schema_id ->Nullable<Varchar>,
            id ->BigInt,
            
        }
    }
    
    table! {
        service_envs (id) {
            parent_id ->BigInt,
            #[max_length = 35000]
            spec ->Varchar,
            env ->Varchar,
            #[max_length = 100]
            base_url ->Varchar,
            updated_at ->Nullable<Timestamptz>,
            #[max_length = 30]
            version ->Varchar,
            pipeline_status ->Nullable<Varchar>,
            id ->BigInt,
            
        }
    }
    
    table! {
        package (id) {
            #[max_length = 100]
            identifier ->Varchar,
            package_type ->Varchar,
            #[max_length = 50]
            lang ->Varchar,
            #[max_length = 100]
            group_id ->Nullable<Varchar>,
            updated_at ->Timestamptz,
            created_at ->Nullable<Timestamptz>,
            #[max_length = 100]
            organization_id ->Nullable<Varchar>,
            #[max_length = 25000]
            description ->Nullable<Varchar>,
            #[max_length = 1000]
            dependencies_json ->Nullable<Varchar>,
            #[max_length = 200]
            repo_origin ->Nullable<Varchar>,
            id ->BigInt,
            
        }
    }
    
    table! {
        package_env (id) {
            #[max_length = 50]
            version ->Varchar,
            env ->Varchar,
            parent_id ->BigInt,
            pipeline_status ->Nullable<Varchar>,
            id ->BigInt,
            
        }
    }
    
    table! {
        organization (id) {
            #[max_length = 100]
            slug ->Varchar,
            #[max_length = 100]
            group_id ->Varchar,
            is_active ->Bool,
            #[max_length = 40000]
            blocks_positions ->Nullable<Varchar>,
            #[max_length = 100]
            name ->Nullable<Varchar>,
            id ->BigInt,
            
        }
    }
    
    
        
    
        diesel::joinable!(dbschema_branch -> dbschema (parent_id));
    
        
    
        
    
        diesel::joinable!(service_envs -> service (parent_id));
    
        
    
        diesel::joinable!(package_env -> package (parent_id));
    
        
    

    diesel::allow_tables_to_appear_in_same_query!(
        dbschema,
        dbschema_branch,
        templates,
        service,
        service_envs,
        package,
        package_env,
        organization,
        
    );
}

use schema::{ dbschema,dbschema_branch,templates,service,service_envs,package,package_env,organization, };



#[derive(Queryable, Debug, Clone, Selectable, Serialize, Deserialize, JsonSchema,Identifiable)]

#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = dbschema)]
pub struct Dbschema {
    pub name:String,
    pub description:Option<String>,
    pub created_at:DateTime<Utc>,
    pub updated_at:DateTime<Utc>,
    pub data:Option<String>,
    pub group_id:Option<String>,
    pub identifier:Option<String>,
    pub organization_id:Option<String>,
    pub repo_origin:Option<String>,
    pub db_type:String,
    pub id:i64,
    
}


#[derive(Queryable, Debug, Clone, Selectable, Serialize, Deserialize, JsonSchema,Identifiable,Associations)]
#[diesel(belongs_to(Dbschema, foreign_key = parent_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = dbschema_branch)]
pub struct Dbschema_Branch {
    pub parent_id:i64,
    pub branch_name:String,
    pub data:Option<String>,
    pub created_at:DateTime<Utc>,
    pub updated_at:DateTime<Utc>,
    pub version:Option<String>,
    pub pipeline_status:Option<String>,
    pub id:i64,
    
}


#[derive(Queryable, Debug, Clone, Selectable, Serialize, Deserialize, JsonSchema,Identifiable)]

#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = templates)]
pub struct Templates {
    pub short_name:String,
    pub description:String,
    pub repo_link:String,
    pub identifier:String,
    pub id:i64,
    
}


#[derive(Queryable, Debug, Clone, Selectable, Serialize, Deserialize, JsonSchema,Identifiable)]

#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = service)]
pub struct Service {
    pub identifier:String,
    pub group_id:Option<String>,
    pub db_schema_id:Option<String>,
    pub tables_json:Option<String>,
    pub dependencies_json:Option<String>,
    pub service_type:String,
    pub lang:Option<String>,
    pub description:Option<String>,
    pub organization_id:Option<String>,
    pub repo_origin:Option<String>,
    pub cache_schema_id:Option<String>,
    pub id:i64,
    
}


#[derive(Queryable, Debug, Clone, Selectable, Serialize, Deserialize, JsonSchema,Identifiable,Associations)]
#[diesel(belongs_to(Service, foreign_key = parent_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = service_envs)]
pub struct Service_Envs {
    pub parent_id:i64,
    pub spec:String,
    pub env:String,
    pub base_url:String,
    pub updated_at:Option<DateTime<Utc>>,
    pub version:String,
    pub pipeline_status:Option<String>,
    pub id:i64,
    
}


#[derive(Queryable, Debug, Clone, Selectable, Serialize, Deserialize, JsonSchema,Identifiable)]

#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = package)]
pub struct Package {
    pub identifier:String,
    pub package_type:String,
    pub lang:String,
    pub group_id:Option<String>,
    pub updated_at:DateTime<Utc>,
    pub created_at:Option<DateTime<Utc>>,
    pub organization_id:Option<String>,
    pub description:Option<String>,
    pub dependencies_json:Option<String>,
    pub repo_origin:Option<String>,
    pub id:i64,
    
}


#[derive(Queryable, Debug, Clone, Selectable, Serialize, Deserialize, JsonSchema,Identifiable,Associations)]
#[diesel(belongs_to(Package, foreign_key = parent_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = package_env)]
pub struct Package_Env {
    pub version:String,
    pub env:String,
    pub parent_id:i64,
    pub pipeline_status:Option<String>,
    pub id:i64,
    
}


#[derive(Queryable, Debug, Clone, Selectable, Serialize, Deserialize, JsonSchema,Identifiable)]

#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = organization)]
pub struct Organization {
    pub slug:String,
    pub group_id:String,
    pub is_active:bool,
    pub blocks_positions:Option<String>,
    pub name:Option<String>,
    pub id:i64,
    
}




#[derive(Queryable, Debug, Clone, Selectable, Serialize, Deserialize, Insertable, JsonSchema)]

#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = dbschema)]
pub struct DbschemaInsertable {
    pub name:String,
    pub description:Option<String>,
    pub created_at:DateTime<Utc>,
    pub updated_at:DateTime<Utc>,
    pub data:Option<String>,
    pub group_id:Option<String>,
    pub identifier:Option<String>,
    pub organization_id:Option<String>,
    pub repo_origin:Option<String>,
    pub db_type:String,
    
}


#[derive(Queryable, Debug, Clone, Selectable, Serialize, Deserialize, Insertable, JsonSchema,Associations)]
#[diesel(belongs_to(Dbschema, foreign_key = parent_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = dbschema_branch)]
pub struct Dbschema_BranchInsertable {
    pub parent_id:i64,
    pub branch_name:String,
    pub data:Option<String>,
    pub created_at:DateTime<Utc>,
    pub updated_at:DateTime<Utc>,
    pub version:Option<String>,
    pub pipeline_status:Option<String>,
    
}


#[derive(Queryable, Debug, Clone, Selectable, Serialize, Deserialize, Insertable, JsonSchema)]

#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = templates)]
pub struct TemplatesInsertable {
    pub short_name:String,
    pub description:String,
    pub repo_link:String,
    pub identifier:String,
    
}


#[derive(Queryable, Debug, Clone, Selectable, Serialize, Deserialize, Insertable, JsonSchema)]

#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = service)]
pub struct ServiceInsertable {
    pub identifier:String,
    pub group_id:Option<String>,
    pub db_schema_id:Option<String>,
    pub tables_json:Option<String>,
    pub dependencies_json:Option<String>,
    pub service_type:String,
    pub lang:Option<String>,
    pub description:Option<String>,
    pub organization_id:Option<String>,
    pub repo_origin:Option<String>,
    pub cache_schema_id:Option<String>,
    
}


#[derive(Queryable, Debug, Clone, Selectable, Serialize, Deserialize, Insertable, JsonSchema,Associations)]
#[diesel(belongs_to(Service, foreign_key = parent_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = service_envs)]
pub struct Service_EnvsInsertable {
    pub parent_id:i64,
    pub spec:String,
    pub env:String,
    pub base_url:String,
    pub updated_at:Option<DateTime<Utc>>,
    pub version:String,
    pub pipeline_status:Option<String>,
    
}


#[derive(Queryable, Debug, Clone, Selectable, Serialize, Deserialize, Insertable, JsonSchema)]

#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = package)]
pub struct PackageInsertable {
    pub identifier:String,
    pub package_type:String,
    pub lang:String,
    pub group_id:Option<String>,
    pub updated_at:DateTime<Utc>,
    pub created_at:Option<DateTime<Utc>>,
    pub organization_id:Option<String>,
    pub description:Option<String>,
    pub dependencies_json:Option<String>,
    pub repo_origin:Option<String>,
    
}


#[derive(Queryable, Debug, Clone, Selectable, Serialize, Deserialize, Insertable, JsonSchema,Associations)]
#[diesel(belongs_to(Package, foreign_key = parent_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = package_env)]
pub struct Package_EnvInsertable {
    pub version:String,
    pub env:String,
    pub parent_id:i64,
    pub pipeline_status:Option<String>,
    
}


#[derive(Queryable, Debug, Clone, Selectable, Serialize, Deserialize, Insertable, JsonSchema)]

#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = organization)]
pub struct OrganizationInsertable {
    pub slug:String,
    pub group_id:String,
    pub is_active:bool,
    pub blocks_positions:Option<String>,
    pub name:Option<String>,
    
}
