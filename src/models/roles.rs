use loco_rs::prelude::*;

pub use super::_entities::roles::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// # Errors
    /// `EntityNotFound` jika nama role belum ada di tabel `roles`.
    pub async fn find_by_name(db: &DatabaseConnection, name: &str) -> ModelResult<Self> {
        roles::Entity::find()
            .filter(
                model::query::condition()
                    .eq(roles::Column::Name, name)
                    .build(),
            )
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)
    }
}
