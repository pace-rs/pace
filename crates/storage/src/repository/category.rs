use pace_error::{DatabaseStorageErrorKind, PaceOptResult, PaceResult};
use sea_orm::{EntityTrait, IntoActiveModel};

use crate::entity::categories::{Entity as CategoryEntity, Model as CategoryModel};
use crate::repository::Repository;

#[derive(Debug)]
pub struct CategoryRepository<'conn, C> {
    connection: &'conn C,
}

impl<'conn, C> CategoryRepository<'conn, C> {
    pub const fn new(connection: &'conn C) -> Self {
        Self { connection }
    }
}

impl<'conn> Repository<CategoryModel> for CategoryRepository<'conn, sea_orm::DatabaseConnection> {
    async fn read(&self, id: &str) -> PaceOptResult<CategoryModel> {
        Ok(CategoryEntity::find_by_id(id)
            .one(self.connection)
            .await
            .map_err(|source| DatabaseStorageErrorKind::RepositoryReadFailed {
                source,
                item_type: "category".to_string(),
                item_id: id.to_string(),
            })?)
    }

    async fn read_all(&self) -> PaceOptResult<Vec<CategoryModel>> {
        let items = CategoryEntity::find()
            .all(self.connection)
            .await
            .map_err(|source| DatabaseStorageErrorKind::RepositoryReadFailed {
                source,
                item_type: "category".to_string(),
                item_id: "all".to_string(),
            })?;

        if items.is_empty() {
            return Ok(None);
        }

        Ok(Some(items))
    }

    async fn create(&self, model: &CategoryModel) -> PaceResult<String> {
        // TODO: What else should we do with ActiveModel here?
        let active_model = model.clone().into_active_model();

        let id = CategoryEntity::insert(active_model)
            .exec(self.connection)
            .await
            .map_err(|source| DatabaseStorageErrorKind::RepositoryCreateFailed {
                source,
                item_type: "category".to_string(),
            })?
            .last_insert_id;

        Ok(id)
    }

    async fn update(&self, id: &str, model: &CategoryModel) -> PaceResult<()> {
        unimplemented!()
    }

    async fn delete(&self, id: &str) -> PaceOptResult<CategoryModel> {
        let item = self.read(id).await?;

        // TODO: Unused result here, what should we do with the rows affected?
        _ = CategoryEntity::delete_by_id(id)
            .exec(self.connection)
            .await
            .map_err(|source| DatabaseStorageErrorKind::RepositoryDeleteFailed {
                source,
                item_type: "category".to_string(),
                item_id: id.to_string(),
            })?;

        Ok(item)
    }
}
