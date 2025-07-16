pub use sea_orm_migration::prelude::*;

mod m20250711_090750_create_kiosk_versions_table;
mod m20250715_063842_create_kiosk_version_platforms_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250711_090750_create_kiosk_versions_table::Migration),
            Box::new(m20250715_063842_create_kiosk_version_platforms_table::Migration),
        ]
    }
}
