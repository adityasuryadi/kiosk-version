use std::string;

use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(KioskVersionPlatform::Table)
                    .if_not_exists()
                    .col(pk_auto(KioskVersionPlatform::Id))
                    .col(integer(KioskVersionPlatform::KioskVersionId))
                    .col(string(KioskVersionPlatform::Url))
                    .col(string(KioskVersionPlatform::Platform))
                    .col(string(KioskVersionPlatform::Signature))
                    .col(string(KioskVersionPlatform::FileName).null())
                    .col(timestamp_with_time_zone(KioskVersionPlatform::CreatedAt))
                    .col(timestamp_with_time_zone(KioskVersionPlatform::UpdatedAt).null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(KioskVersionPlatform::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum KioskVersionPlatform {
    Table,
    Id,
    KioskVersionId,
    Platform,
    Url,
    Signature,
    FileName,
    CreatedAt,
    UpdatedAt,
}
