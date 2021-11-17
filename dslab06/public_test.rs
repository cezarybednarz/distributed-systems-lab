#[cfg(test)]
mod tests {
    use std::io::Read;

    use ntest::timeout;
    use solution::build_stable_storage;
    use tempfile::tempdir;

    #[tokio::test]
    #[timeout(500)]
    async fn storage_retrieves_inserted_key() {
        // Given:
        let root_storage_dir = tempdir().unwrap();
        let mut storage = build_stable_storage(root_storage_dir.path().to_path_buf()).await;

        // When:
        let before_insertion = storage.get("key").await;
        // Then:
        assert_eq!(before_insertion, None);

        // When:
        storage
            .put("key", vec![1 as u8, 2, 3].as_slice())
            .await
            .unwrap();
        // Then:
        assert_eq!(storage.get("key").await.unwrap(), vec![1 as u8, 2, 3]);
    }

    #[tokio::test]
    #[timeout(500)]
    async fn restore_test() {
        // Given:
        let root_storage_dir = tempdir().unwrap();
        let root_storage_dir_path = root_storage_dir.path().to_path_buf();
        let root_storage_dir_path_clone = root_storage_dir_path.clone();

        tokio::spawn(async move {
            let mut storage = build_stable_storage(root_storage_dir_path.clone()).await;

            // When:
            let before_insertion = storage.get("key").await;
            // Then:
            assert_eq!(before_insertion, None);

            storage
                .put("key", vec![1 as u8, 2, 3].as_slice())
                .await
                .unwrap();
            assert_eq!(storage.get("key").await.unwrap(), vec![1 as u8, 2, 3]);

            let paths = std::fs::read_dir(root_storage_dir_path).unwrap();
            for path in paths {
                let path = path.unwrap().path();
                println!("Name: {}", path.display());

                if std::fs::metadata(&path).unwrap().is_dir() {
                    println!("Directory");
                } else {
                    let mut file = std::fs::File::open(path).unwrap();
                    let mut contents = String::new();
                    file.read_to_string(&mut contents).unwrap();

                    println!("{:?}", contents.as_bytes());
                }
            }
        })
        .await
        .unwrap();

        tokio::spawn(async move {
            println!();
            println!("Second thread.");
            println!();

            
            let paths = std::fs::read_dir(root_storage_dir_path_clone.clone()).unwrap();
            for path in paths {
                let path = path.unwrap().path();
                println!("Name: {}", path.display());

                if std::fs::metadata(&path).unwrap().is_dir() {
                    println!("Directory");
                } else {
                    let mut file = std::fs::File::open(path).unwrap();
                    let mut contents = String::new();
                    file.read_to_string(&mut contents).unwrap();

                    println!("{:?}", contents.as_bytes());
                }
            }

            let storage = build_stable_storage(root_storage_dir_path_clone).await;
            assert_eq!(storage.get("key2").await, None);
            assert_eq!(storage.get("key").await.unwrap(), vec![1 as u8, 2, 3]);
        })
        .await
        .unwrap();

        // When:
        // Then:
    }
}


