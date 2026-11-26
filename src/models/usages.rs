use loco_rs::prelude::*;

pub use super::_entities::usages::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Periode bulan berjalan, contoh `2026-11`.
    #[must_use]
    pub fn current_period() -> String {
        chrono::Utc::now().format("%Y-%m").to_string()
    }

    /// Kunci unik kombinasi user + periode.
    #[must_use]
    pub fn bucket(user_id: i64, period: &str) -> String {
        format!("{user_id}:{period}")
    }

    async fn find_current<C>(db: &C, user_id: i64, period: &str) -> ModelResult<Option<Self>>
    where
        C: ConnectionTrait,
    {
        let bucket = Self::bucket(user_id, period);
        let row = Entity::find()
            .filter(
                model::query::condition()
                    .eq(usages::Column::Bucket, bucket)
                    .build(),
            )
            .one(db)
            .await?;
        Ok(row)
    }

    /// Ambil (atau buat sekali) baris usage untuk periode ini.
    ///
    /// # Errors
    /// Gagal jika DB tidak bisa dibaca atau baris baru tidak bisa dibuat.
    pub async fn current(db: &DatabaseConnection, user_id: i64, period: &str) -> ModelResult<Self> {
        if let Some(row) = Self::find_current(db, user_id, period).await? {
            return Ok(row);
        }

        let txn = db.begin().await?;
        if let Some(row) = Self::find_current(&txn, user_id, period).await? {
            txn.commit().await?;
            return Ok(row);
        }

        let bucket = Self::bucket(user_id, period);
        let row = ActiveModel {
            bucket: Set(bucket),
            user_id: Set(user_id),
            period: Set(period.to_owned()),
            sent: Set(0),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        txn.commit().await?;
        Ok(row)
    }

    /// Pakai satu kuota. Mengembalikan total pesan setelah update.
    ///
    /// # Errors
    /// `kuota habis` jika `sent >= quota`, atau error DB.
    pub async fn consume(
        db: &DatabaseConnection,
        user_id: i64,
        period: &str,
        quota: i64,
    ) -> ModelResult<i64> {
        let txn = db.begin().await?;
        let Some(row) = Self::find_current(&txn, user_id, period).await? else {
            txn.rollback().await?;
            return Err(ModelError::msg("usage tidak ditemukan"));
        };

        if row.sent >= quota {
            txn.rollback().await?;
            return Err(ModelError::msg("kuota habis"));
        }

        let next = row.sent + 1;
        let mut active = row.into_active_model();
        active.sent = Set(next);
        let saved = active.update(&txn).await?;
        txn.commit().await?;
        Ok(saved.sent)
    }
}
