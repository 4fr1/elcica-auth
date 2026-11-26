use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "roles",
            &[
                ("id", ColType::PkAuto),
                ("name", ColType::StringUniq),
                ("label", ColType::StringNull),
            ],
            &[],
        )
        .await?;

        create_table(
            m,
            "plans",
            &[
                ("id", ColType::PkAuto),
                ("name", ColType::StringUniq),
                ("quota", ColType::IntegerWithDefault(100)),
                ("price_idr", ColType::BigIntegerWithDefault(0)),
            ],
            &[],
        )
        .await?;

        create_table(
            m,
            "user_profiles",
            &[
                ("id", ColType::PkAuto),
                ("user_id", ColType::BigIntegerUniq),
                ("role_id", ColType::BigInteger),
                ("plan_id", ColType::BigInteger),
            ],
            &[
                ("user", "user_id"),
                ("role", "role_id"),
                ("plan", "plan_id"),
            ],
        )
        .await?;

        create_table(
            m,
            "usages",
            &[
                ("id", ColType::PkAuto),
                ("bucket", ColType::StringUniq),
                ("user_id", ColType::BigInteger),
                ("period", ColType::String),
                ("sent", ColType::IntegerWithDefault(0)),
            ],
            &[("user", "user_id")],
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "usages").await?;
        drop_table(m, "user_profiles").await?;
        drop_table(m, "plans").await?;
        drop_table(m, "roles").await?;
        Ok(())
    }
}
