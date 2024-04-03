pub mod activity;
pub mod category;
pub mod tag;

use getset::Getters;
use pace_error::{PaceOptResult, PaceResult};

pub(crate) trait Repository<T> {
    /// Read a single entity by its id.
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the entity to read.
    ///
    /// # Errors
    ///
    /// Returns an error if there was a problem reading the entity.
    ///
    /// # Returns
    ///
    /// Returns the entity if it exists or None if it does not.
    async fn read(&self, id: &str) -> PaceOptResult<T>;

    /// Read all entities of a given type.
    ///
    /// # Errors
    ///
    /// Returns an error if there was a problem reading the entities.
    ///
    /// # Returns
    ///
    /// Returns a vector of all entities of the given type or an
    /// empty vector if there are none.
    async fn read_all(&self) -> PaceOptResult<Vec<T>>;

    /// Create a new entity of a given type.
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to create.
    ///
    /// # Errors
    ///
    /// Returns an error if there was a problem creating the entity.
    ///
    /// # Returns
    ///
    /// Returns the id of the created entity.
    async fn create(&self, model: &T) -> PaceResult<String>;

    /// Update an existing entity of a given type.
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the entity to update.
    /// * `entity` - The entity to update.
    ///
    /// # Errors
    ///
    /// Returns an error if there was a problem updating the entity.
    ///
    /// # Returns
    ///
    /// Returns nothing if the entity was updated successfully.
    async fn update(&self, id: &str, model: &T) -> PaceResult<()>;

    /// Delete an existing entity of a given type.
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the entity to delete.
    ///
    /// # Errors
    ///
    /// Returns an error if there was a problem deleting the entity.
    ///
    /// # Returns
    ///
    /// Returns the deleted entity if it exists.
    async fn delete(&self, id: &str) -> PaceOptResult<T>;
}

#[derive(Debug, Getters)]
#[getset(get = "pub")]
pub struct SeaOrmRepository<'conn> {
    activity: activity::ActivityRepository<'conn, sea_orm::DatabaseConnection>,
    category: category::CategoryRepository<'conn, sea_orm::DatabaseConnection>,
    tag: tag::TagRepository<'conn, sea_orm::DatabaseConnection>,
}

impl<'conn> SeaOrmRepository<'conn> {
    #[must_use]
    pub const fn new(connection: &'conn sea_orm::DatabaseConnection) -> Self {
        Self {
            activity: activity::ActivityRepository::new(connection),
            category: category::CategoryRepository::new(connection),
            tag: tag::TagRepository::new(connection),
        }
    }
}
