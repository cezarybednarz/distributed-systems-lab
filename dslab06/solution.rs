use std::path::PathBuf;
// You can add here other imports from std or crates listed in Cargo.toml.

// You can add any private types, structs, consts, functions, methods, etc., you need.

#[async_trait::async_trait]
pub trait StableStorage: Send + Sync {
    /// Stores `value` under  `key`.
    ///
    /// Detailed requirements are specified in the description of the assignment.
    async fn put(&mut self, key: &str, value: &[u8]) -> Result<(), String>;

    /// Retrieves value stored under `key`.
    ///
    /// Detailed requirements are specified in the description of the assignment.
    async fn get(&self, key: &str) -> Option<Vec<u8>>;
}

/// Creates a new instance of stable storage.
pub async fn build_stable_storage(root_storage_dir: PathBuf) -> Box<dyn StableStorage> {
    unimplemented!()
}
