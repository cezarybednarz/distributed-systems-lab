use std::{path::PathBuf, mem};
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

fn get_hash(name: &str) -> String {
    let mut sha256 = Sha256::new();
    sha256.update(name);
    let hash = sha256.finalize();
    return format!("{:X}", hash);
}

struct Storage {
    dst_path: PathBuf,
}

impl Storage {
    fn new(dst_path: PathBuf) -> Self {
        Self {
            dst_path,
        }
    }
}

static TMP_DIR_NAME: &str = "tmp";
static DST_DIR_NAME: &str = "dst";

#[async_trait::async_trait]
impl StableStorage for Storage {
    async fn put(&mut self, key: &str, value: &[u8]) -> Result<(), String> {
        if mem::size_of_val(key) > 255 {
            return Err("Size of key too big".to_string());
        }
        if mem::size_of_val(value) > 65535 {
            return Err("Size of value too big".to_string());
        }

        let mut tmp_dir = self.dst_path.clone();
        tmp_dir.push(TMP_DIR_NAME);
        
        let filename = get_hash(key);
        let mut tmp_file = tmp_dir.clone();
        tmp_file.push(filename);

        let mut dst_dir = self.dst_path.clone();
        dst_dir.push(DST_DIR_NAME);
        
        println!("{}\n{}\n{}\n", tmp_dir.display(), tmp_file.display(), self.dst_path.display());

        fs::create_dir(tmp_dir.clone()).await.unwrap();
        let mut file = fs::File::create(tmp_file).await.unwrap();
        file.write_all(value).await.unwrap();
        file.sync_data().await.unwrap();
        fs::rename(tmp_dir, dst_dir.clone()).await.unwrap();
        file.sync_data().await.unwrap();

        return Ok(());
    }

    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let filename = get_hash(key);
        
        let mut dst_file = self.dst_path.clone();
        dst_file.push(DST_DIR_NAME);
        dst_file.push(filename);
    
        let file = fs::File::open(dst_file).await;
        match file {
            Ok(mut file) => {
                let mut contents = vec![];
                file.read_to_end(&mut contents).await.unwrap();
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
    let storage = Storage::new(root_storage_dir);
    return Box::new(storage);
}
