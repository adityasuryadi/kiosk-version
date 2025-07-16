use sea_orm::{entity::prelude::*, sqlx::types::chrono};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "kiosk_version")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub version: String,
    pub note: String,
    pub url: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    KioskVersionPlatform,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::KioskVersionPlatform => {
                Entity::has_many(super::kiosk_version_platform::Entity).into()
            }
        }
    }
}

impl Related<super::kiosk_version_platform::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::KioskVersionPlatform.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
