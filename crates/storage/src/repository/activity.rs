use pace_error::{DatabaseStorageErrorKind, PaceOptResult, PaceResult};
use sea_orm::{EntityTrait, IntoActiveModel};

use crate::entity::activities::{Entity as ActivityEntity, Model as ActivityModel};
use crate::entity::categories::{Entity as CategoryEntity, Model as CategoryModel};
use crate::repository::Repository;

#[derive(Debug)]
pub struct ActivityRepository<'conn, C> {
    connection: &'conn C,
}

impl<'conn, C> ActivityRepository<'conn, C> {
    pub const fn new(connection: &'conn C) -> Self {
        Self { connection }
    }
}

// TODO!: Implement query for related entities
impl<'conn> Repository<ActivityModel> for ActivityRepository<'conn, sea_orm::DatabaseConnection> {
    async fn read(&self, id: &str) -> PaceOptResult<ActivityModel> {
        Ok(ActivityEntity::find_by_id(id)
            .one(self.connection)
            .await
            .map_err(|source| DatabaseStorageErrorKind::RepositoryReadFailed {
                source,
                item_type: "activity".to_string(),
                item_id: id.to_string(),
            })?)
    }

    async fn read_all(&self) -> PaceOptResult<Vec<ActivityModel>> {
        let items = ActivityEntity::find()
            .all(self.connection)
            .await
            .map_err(|source| DatabaseStorageErrorKind::RepositoryReadFailed {
                source,
                item_type: "activity".to_string(),
                item_id: "all".to_string(),
            })?;

        if items.is_empty() {
            return Ok(None);
        }

        Ok(Some(items))
    }

    async fn create(&self, model: &ActivityModel) -> PaceResult<String> {
        // TODO: What else should we do with ActiveModel here?
        let active_model = model.clone().into_active_model();

        let id = ActivityEntity::insert(active_model)
            .exec(self.connection)
            .await
            .map_err(|source| DatabaseStorageErrorKind::RepositoryCreateFailed {
                source,
                item_type: "activity".to_string(),
            })?
            .last_insert_id;

        Ok(id)
    }

    async fn update(&self, id: &str, model: &ActivityModel) -> PaceResult<()> {
        unimplemented!()
    }

    async fn delete(&self, id: &str) -> PaceOptResult<ActivityModel> {
        let item = self.read(id).await?;

        // TODO: Unused result here, what should we do with the rows affected?
        _ = ActivityEntity::delete_by_id(id)
            .exec(self.connection)
            .await
            .map_err(|source| DatabaseStorageErrorKind::RepositoryDeleteFailed {
                source,
                item_type: "activity".to_string(),
                item_id: id.to_string(),
            })?;

        Ok(item)
    }
}
