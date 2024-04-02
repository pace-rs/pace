pub mod category;

use pace_error::{PaceOptResult, PaceResult};

pub trait Repository<T> {
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
    fn read(&self, id: &str) -> PaceOptResult<T>;

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
    fn read_all(&self) -> PaceOptResult<Vec<T>>;

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
    fn create(&self, entity: &T) -> PaceResult<String>;

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
    fn update(&self, id: &str, entity: &T) -> PaceResult<()>;

    /// Delete an existing entity of a given type.
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the entity to delete.
    ///
    /// # Errors
    ///
    /// Returns an error if there was a problem deleting the entity.
    fn delete(&self, id: &str) -> PaceResult<()>;
}
