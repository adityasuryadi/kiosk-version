use sea_orm::{entity::prelude::*, sqlx::types::chrono};
use serde::{Deserialize, Serialize};

use crate::entity::kiosk_version;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "kiosk_version")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub kiosk_version_id: i32,
    pub platform: String,
    pub url: String,
    pub signature: String,
    pub filename: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::kiosk_version::Entity",
        from = "Column::KioskVersionId",
        to = "super::kiosk_version::Column::Id"
    )]
    KioskVersion,
}

impl Related<kiosk_version::Entity> for Entity {
    fn to() -> RelationDef {
        // define the relationship here
        Relation::KioskVersion.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
