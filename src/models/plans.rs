use loco_rs::prelude::*;

pub use super::_entities::plans::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// # Errors
    /// `EntityNotFound` jika nama paket belum ada di tabel `plans`.
    pub async fn find_by_name(db: &DatabaseConnection, name: &str) -> ModelResult<Self> {
        plans::Entity::find()
            .filter(
                model::query::condition()
                    .eq(plans::Column::Name, name)
                    .build(),
            )
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)
    }
}
