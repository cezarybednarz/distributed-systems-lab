use assignment_2_solution::{
    run_register_process, serialize_register_command, ClientCommandHeader, ClientRegisterCommand,
    ClientRegisterCommandContent, Configuration, PublicConfiguration, RegisterCommand, SectorVec,
    MAGIC_NUMBER,
};
use assignment_2_test_utils::system::*;
use hmac::{Mac, NewMac};
use ntest::timeout;
use std::convert::TryInto;
use std::time::Duration;
use tempfile::tempdir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::test]
#[serial_test::serial]
//#[timeout(10000)]
async fn write_and_read_on_the_same_sector() {
    env_logger::init();
    // given
    let port_range_start = 21518;
    let config = TestProcessesConfig::new(1, port_range_start);
    config.start().await;
    let mut stream = config.connect(0).await;
    let mut _other_stream = config.connect(0).await;

    // when
    config
        .send_cmd(
            &RegisterCommand::Client(ClientRegisterCommand {
                header: ClientCommandHeader {
                    request_identifier: 1234,
                    sector_idx: 0,
                },
                content: ClientRegisterCommandContent::Write {
                    data: SectorVec(vec![1; 4096]),
                },
            }),
            &mut stream,
        )
        .await;
    
    //tokio::time::sleep(Duration::from_millis(2000)).await;

    log::debug!("after sending WRITE");
    config.read_response(&mut stream).await.unwrap();
    


    // config
    //     .send_cmd(
    //         &RegisterCommand::Client(ClientRegisterCommand {
    //             header: ClientCommandHeader {
    //                 request_identifier: 1235,
    //                 sector_idx: 0,
    //             },
    //             content: ClientRegisterCommandContent::Read,
    //         }),
    //         &mut stream,
    //     )
    //     .await;
    // let response = config.read_response(&mut stream).await.unwrap();

    // match response.content {
    //     RegisterResponseContent::Read(SectorVec(sector)) => {
    //         assert!(sector == vec![1; 4096] || sector == vec![254; 4096]);
    //     }
    //     _ => panic!("Expected read response"),
    // }
}

async fn send_cmd(register_cmd: &RegisterCommand, stream: &mut TcpStream, hmac_client_key: &[u8]) {
    let mut data = Vec::new();
    serialize_register_command(register_cmd, &mut data, hmac_client_key)
        .await
        .unwrap();

    stream.write_all(&data).await.unwrap();
}

fn hmac_tag_is_ok(key: &[u8], data: &[u8]) -> bool {
    let boundary = data.len() - HMAC_TAG_SIZE;
    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(&data[..boundary]);
    mac.verify(&data[boundary..]).is_ok()
}
