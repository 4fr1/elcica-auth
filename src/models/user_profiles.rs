use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use super::{plans, roles, usages};

pub use super::_entities::user_profiles::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for ActiveModel {}

#[derive(Debug, Deserialize, Serialize)]
pub struct Access {
    pub role: String,
    pub plan: String,
    pub quota: i64,
    pub sent: i64,
}

impl Model {
    /// # Errors
    /// Gagal jika query DB gagal.
    pub async fn find_by_user<C>(db: &C, user_id: i64) -> ModelResult<Option<Self>>
    where
        C: ConnectionTrait,
    {
        let row = Entity::find()
            .filter(
                model::query::condition()
                    .eq(user_profiles::Column::UserId, user_id)
                    .build(),
            )
            .one(db)
            .await?;
        Ok(row)
    }

    /// Pastikan user punya profil. Default: role `user`, plan `free`.
    ///
    /// # Errors
    /// Gagal jika seed `user`/`free` belum ada atau DB error.
    pub async fn ensure_for_user(db: &DatabaseConnection, user_id: i64) -> ModelResult<Self> {
        if let Some(profile) = Self::find_by_user(db, user_id).await? {
            return Ok(profile);
        }

        let role = roles::Model::find_by_name(db, "user").await?;
        let plan = plans::Model::find_by_name(db, "free").await?;

        let txn = db.begin().await?;
        if let Some(profile) = Self::find_by_user(&txn, user_id).await? {
            txn.commit().await?;
            return Ok(profile);
        }

        let profile = ActiveModel {
            user_id: Set(user_id),
            role_id: Set(role.id),
            plan_id: Set(plan.id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        txn.commit().await?;
        Ok(profile)
    }

    /// Ringkasan akses user: role, paket, kuota, dan pemakaian bulan ini.
    ///
    /// # Errors
    /// Gagal jika profil/role/plan/usage tidak bisa disiapkan.
    pub async fn access(
        db: &DatabaseConnection,
        user_id: i64,
        period: &str,
    ) -> ModelResult<Access> {
        let profile = Self::ensure_for_user(db, user_id).await?;

        let role = roles::Entity::find_by_id(profile.role_id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?;
        let plan = plans::Entity::find_by_id(profile.plan_id)
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)?;
        let usage = usages::Model::current(db, user_id, period).await?;

        Ok(Access {
            role: role.name,
            plan: plan.name,
            quota: plan.quota,
            sent: usage.sent,
        })
    }
}
