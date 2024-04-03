use pace_error::{DatabaseStorageErrorKind, PaceOptResult, PaceResult};
use sea_orm::{EntityTrait, IntoActiveModel};

use crate::entity::tags::{Entity as TagEntity, Model as TagModel};
use crate::repository::Repository;

#[derive(Debug)]
pub struct TagRepository<'conn, C> {
    connection: &'conn C,
}

impl<'conn, C> TagRepository<'conn, C> {
    pub const fn new(connection: &'conn C) -> Self {
        Self { connection }
    }
}

impl<'conn> Repository<TagModel> for TagRepository<'conn, sea_orm::DatabaseConnection> {
    async fn read(&self, id: &str) -> PaceOptResult<TagModel> {
        Ok(TagEntity::find_by_id(id)
            .one(self.connection)
            .await
            .map_err(|source| DatabaseStorageErrorKind::RepositoryReadFailed {
                source,
                item_type: "tag".to_string(),
                item_id: id.to_string(),
            })?)
    }

    async fn read_all(&self) -> PaceOptResult<Vec<TagModel>> {
        let items = TagEntity::find()
            .all(self.connection)
            .await
            .map_err(|source| DatabaseStorageErrorKind::RepositoryReadFailed {
                source,
                item_type: "tag".to_string(),
                item_id: "all".to_string(),
            })?;

        if items.is_empty() {
            return Ok(None);
        }

        Ok(Some(items))
    }

    async fn create(&self, model: &TagModel) -> PaceResult<String> {
        // TODO: What else should we do with ActiveModel here?
        let active_model = model.clone().into_active_model();

        let id = TagEntity::insert(active_model)
            .exec(self.connection)
            .await
            .map_err(|source| DatabaseStorageErrorKind::RepositoryCreateFailed {
                source,
                item_type: "tag".to_string(),
            })?
            .last_insert_id;

        Ok(id)
    }

    async fn update(&self, id: &str, model: &TagModel) -> PaceResult<()> {
        unimplemented!()
    }

    async fn delete(&self, id: &str) -> PaceOptResult<TagModel> {
        let item = self.read(id).await?;

        // TODO: Unused result here, what should we do with the rows affected?
        _ = TagEntity::delete_by_id(id)
            .exec(self.connection)
            .await
            .map_err(|source| DatabaseStorageErrorKind::RepositoryDeleteFailed {
                source,
                item_type: "tag".to_string(),
                item_id: id.to_string(),
            })?;

        Ok(item)
    }
}
