use pace_error::{PaceOptResult, PaceResult};
use sea_orm::{sea_query::Query, DatabaseConnection};

use crate::entity::categories::{Entity as CategoryEntity, Model as CategoryModel};
use crate::entity::tags::Entity as TagEntity;
use crate::repository::Repository;

pub struct CategoryRepository {
    connection: DatabaseConnection,
}

impl Repository<CategoryEntity> for CategoryRepository {
    fn read(&self, id: &str) -> PaceOptResult<CategoryEntity> {
        unimplemented!()
    }

    fn read_all(&self) -> PaceOptResult<Vec<CategoryEntity>> {
        unimplemented!()
    }

    fn create(&self, entity: &CategoryEntity) -> PaceResult<String> {
        unimplemented!()
    }

    fn update(&self, id: &str, entity: &CategoryEntity) -> PaceResult<()> {
        unimplemented!()
    }

    fn delete(&self, id: &str) -> PaceResult<()> {
        unimplemented!()
    }
}
