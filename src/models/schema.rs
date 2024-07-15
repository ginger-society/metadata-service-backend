#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
use diesel::Insertable;
use chrono::NaiveDate;
use diesel::{deserialize::Queryable, table, Selectable};
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
            #[max_length = 10]
            version ->Varchar,
            created_at ->Timestamptz,
            updated_at ->Timestamptz,
            #[max_length = 10000]
            data ->Varchar,
            #[max_length = 500]
            group_id ->Varchar,
            id ->BigInt,
            
        }
    }
    
    table! {
        dbschema_branch (id) {
            parent_id ->BigInt,
            #[max_length = 100]
            branch_name ->Varchar,
            #[max_length = 10000]
            data ->Varchar,
            created_at ->Timestamptz,
            updated_at ->Timestamptz,
            merged ->Bool,
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
            id ->BigInt,
            
        }
    }
    
    table! {
        service_envs (id) {
            parent_id ->BigInt,
            #[max_length = 10000]
            spec ->Varchar,
            env ->Varchar,
            #[max_length = 100]
            base_url ->Varchar,
            id ->BigInt,
            
        }
    }
    
    
        
    
        diesel::joinable!(dbschema_branch -> dbschema (parent_id));
    
        
    
        
    
        diesel::joinable!(service_envs -> service (parent_id));
    

    diesel::allow_tables_to_appear_in_same_query!(
        dbschema,
        dbschema_branch,
        templates,
        service,
        service_envs,
        
    );
}

use schema::{ dbschema,dbschema_branch,templates,service,service_envs, };



#[derive(Queryable, Debug, Selectable, Serialize, Deserialize, JsonSchema,Identifiable)]

#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = dbschema)]
pub struct Dbschema {
    pub name:String,
    pub description:Option<String>,
    pub version:String,
    pub created_at:DateTime<Utc>,
    pub updated_at:DateTime<Utc>,
    pub data:String,
    pub group_id:String,
    pub id:i64,
    
}


#[derive(Queryable, Debug, Selectable, Serialize, Deserialize, JsonSchema,Identifiable,Associations)]
#[diesel(belongs_to(Dbschema, foreign_key = parent_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = dbschema_branch)]
pub struct Dbschema_Branch {
    pub parent_id:i64,
    pub branch_name:String,
    pub data:String,
    pub created_at:DateTime<Utc>,
    pub updated_at:DateTime<Utc>,
    pub merged:bool,
    pub id:i64,
    
}


#[derive(Queryable, Debug, Selectable, Serialize, Deserialize, JsonSchema,Identifiable)]

#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = templates)]
pub struct Templates {
    pub short_name:String,
    pub description:String,
    pub repo_link:String,
    pub identifier:String,
    pub id:i64,
    
}


#[derive(Queryable, Debug, Selectable, Serialize, Deserialize, JsonSchema,Identifiable)]

#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = service)]
pub struct Service {
    pub identifier:String,
    pub group_id:Option<String>,
    pub id:i64,
    
}


#[derive(Queryable, Debug, Selectable, Serialize, Deserialize, JsonSchema,Identifiable,Associations)]
#[diesel(belongs_to(Service, foreign_key = parent_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = service_envs)]
pub struct Service_Envs {
    pub parent_id:i64,
    pub spec:String,
    pub env:String,
    pub base_url:String,
    pub id:i64,
    
}




#[derive(Queryable, Debug, Selectable, Serialize, Deserialize, Insertable, JsonSchema)]

#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = dbschema)]
pub struct DbschemaInsertable {
    pub name:String,
    pub description:Option<String>,
    pub version:String,
    pub created_at:DateTime<Utc>,
    pub updated_at:DateTime<Utc>,
    pub data:String,
    pub group_id:String,
    
}


#[derive(Queryable, Debug, Selectable, Serialize, Deserialize, Insertable, JsonSchema,Associations)]
#[diesel(belongs_to(Dbschema, foreign_key = parent_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = dbschema_branch)]
pub struct Dbschema_BranchInsertable {
    pub parent_id:i64,
    pub branch_name:String,
    pub data:String,
    pub created_at:DateTime<Utc>,
    pub updated_at:DateTime<Utc>,
    pub merged:bool,
    
}


#[derive(Queryable, Debug, Selectable, Serialize, Deserialize, Insertable, JsonSchema)]

#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = templates)]
pub struct TemplatesInsertable {
    pub short_name:String,
    pub description:String,
    pub repo_link:String,
    pub identifier:String,
    
}


#[derive(Queryable, Debug, Selectable, Serialize, Deserialize, Insertable, JsonSchema)]

#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = service)]
pub struct ServiceInsertable {
    pub identifier:String,
    pub group_id:Option<String>,
    
}


#[derive(Queryable, Debug, Selectable, Serialize, Deserialize, Insertable, JsonSchema,Associations)]
#[diesel(belongs_to(Service, foreign_key = parent_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = service_envs)]
pub struct Service_EnvsInsertable {
    pub parent_id:i64,
    pub spec:String,
    pub env:String,
    pub base_url:String,
    
}
