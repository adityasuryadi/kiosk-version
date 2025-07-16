use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(KioskVersion::Table)
                    .if_not_exists()
                    .col(pk_auto(KioskVersion::Id))
                    .col(string(KioskVersion::Version))
                    .col(string(KioskVersion::Note))
                    .col(string(KioskVersion::Url))
                    .col(timestamp_with_time_zone(KioskVersion::CreatedAt))
                    .col(timestamp_with_time_zone(KioskVersion::UpdatedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(KioskVersion::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum KioskVersion {
    Table,
    Id,
    Version,
    Note,
    Url,
    CreatedAt,
    UpdatedAt,
}
