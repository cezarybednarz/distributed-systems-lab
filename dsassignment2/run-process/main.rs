use assignment_2_solution::{run_register_process, Configuration, PublicConfiguration};
use tempfile::tempdir;
use tokio::net::{TcpStream, TcpListener};

// sudo lsof -i -P -n | grep TCP

#[tokio::main]
async fn main() {
    env_logger::init();

    check_things().await;

    let hmac_client_key = [5; 32];
    let tcp_port = 12_345;
    let storage_dir = tempdir().unwrap();

    let config = Configuration {
        public: PublicConfiguration {
            tcp_locations: vec![("127.0.0.1".to_string(), tcp_port)],
            self_rank: 1,
            max_sector: 20,
            storage_dir: storage_dir.into_path(),
        },
        hmac_system_key: [1; 64],
        hmac_client_key,
    };
    run_register_process(config).await;
    
}

async fn check_things() {
    //let mut stream = TcpStream::connect("127.0.0.1:12356").await.unwrap();
    
    // stream.write_all(b"hello world!").await.unwrap();

}
