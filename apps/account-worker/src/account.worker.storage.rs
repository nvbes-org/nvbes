use std::sync::Arc;

use nvbes_storage::ObjectStore;

#[derive(Clone)]
pub struct AccountStorage {
    objects: Arc<dyn ObjectStore>,
}

impl AccountStorage {
    pub fn new(objects: Arc<dyn ObjectStore>) -> Self {
        Self { objects }
    }

    pub async fn delete_avatar(&self, object_key: Option<&str>) -> anyhow::Result<()> {
        let Some(object_key) = object_key else {
            return Ok(());
        };
        self.objects
            .delete_objects(&[object_key.to_string()])
            .await?;
        Ok(())
    }
}
