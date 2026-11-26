use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                r#"
                INSERT INTO roles (name, label) VALUES
                  ('admin', 'Superadmin'),
                  ('employee', 'Karyawan'),
                  ('user', 'Pengguna')
                ON CONFLICT (name) DO NOTHING
                "#,
            )
            .await?;

        m.get_connection()
            .execute_unprepared(
                r#"
                INSERT INTO plans (name, quota, price_idr) VALUES
                  ('free', 100, 0),
                  ('paid', 5000, 25000)
                ON CONFLICT (name) DO NOTHING
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, _m: &SchemaManager) -> Result<(), DbErr> {
        // Seed sengaja dibiarkan agar profil user tidak kehilangan role/plan.
        Ok(())
    }
}
