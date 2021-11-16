use std::{collections::HashMap, path::PathBuf, mem};
use sha2::{Sha256, Digest};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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

static TMP_DIR: &str = "dir/tmp";
static DST_DIR: &str = "dir/dst";

fn get_hash(name: &str) -> String {
    let mut sha256 = Sha256::new();
    sha256.update(name);
    let hash = sha256.finalize();
    return format!("{:X}", hash);
}

impl dyn StableStorage {
    async fn put(&mut self, key: &str, value: &[u8]) -> Result<(), String> {
        if mem::size_of_val(key) > 255 {
            return Err("Size of key too big".to_string());
        }
        if mem::size_of_val(value) > 65535 {
            return Err("Size of value too big".to_string());
        }

        let filename = get_hash(key);
        let tmp_file = [TMP_DIR, "/", &filename].join("");

        fs::create_dir(TMP_DIR).await.unwrap();
        let mut file = fs::File::create(tmp_file).await.unwrap();
        file.write_all(value).await.unwrap();
        file.sync_data().await.unwrap();
        fs::rename(TMP_DIR, DST_DIR).await.unwrap();
        file.sync_data().await.unwrap();

        return Ok(());
    }

    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let filename = get_hash(key);
        let dst_file = [DST_DIR, "/", &filename].join("");
        let file = fs::File::open(dst_file).await;
        match file {
            Ok(mut file) => {
                let mut contents = vec![];
                file.read_to_end(&mut contents).await;
                return Some(contents);
            }
            Err(_) => {
                None
            }
        }
    }
}

/// Creates a new instance of stable storage.
pub async fn build_stable_storage(root_storage_dir: PathBuf) -> Box<dyn StableStorage> {
    unimplemented!()
}
