use assignment_2_solution::{
  deserialize_register_command, serialize_register_command, ClientCommandHeader,
  ClientRegisterCommand, ClientRegisterCommandContent, RegisterCommand, SystemCommandHeader,
  SystemRegisterCommand, SystemRegisterCommandContent, SectorVec,
};
use assignment_2_test_utils::transfer::*;
use ntest::timeout;
use rand::Rng;
use uuid::Uuid;

fn get_random_sector_vec() -> SectorVec {
  let mut vec = Vec::new();
  let mut rng = rand::thread_rng();
  for _i in 0..4096 {
    vec.push(rng.gen());
  }
  SectorVec(vec)
}

#[tokio::test]
#[timeout(200)]
async fn write_serialize_deserialize_is_identity() {
    // given
    let request_identifier = 7;
    let sector_idx = 8;
    let sector_data = get_random_sector_vec();
    let register_cmd = RegisterCommand::Client(ClientRegisterCommand {
        header: ClientCommandHeader {
            request_identifier,
            sector_idx,
        },
        content: ClientRegisterCommandContent::Write {
          data: sector_data.clone()
        },
    });
    let mut sink: Vec<u8> = Vec::new();

    // when
    serialize_register_command(&register_cmd, &mut sink, &[0x00_u8; 32])
        .await
        .expect("Could not serialize?");
    let mut slice: &[u8] = &sink[..];
    let data_read: &mut (dyn tokio::io::AsyncRead + Send + Unpin) = &mut slice;
    let (deserialized_cmd, hmac_valid) =
        deserialize_register_command(data_read, &[0x00_u8; 64], &[0x00_u8; 32])
            .await
            .expect("Could not deserialize");

    // then
    assert!(hmac_valid);
    match deserialized_cmd {
        RegisterCommand::Client(ClientRegisterCommand {
            header,
            content: ClientRegisterCommandContent::Write {
              data
            },
        }) => {
            assert_eq!(header.sector_idx, sector_idx);
            assert_eq!(header.request_identifier, request_identifier);
            assert_eq!(data, sector_data);
        }
        _ => panic!("Expected Write command"),
    }
}

#[tokio::test]
#[timeout(200)]
async fn read_proc_serialize_deserialize_is_identity() {
    // given
    let process_identifier = 3;
    let msg_ident = Uuid::new_v4();
    let read_ident = 2134;
    let sector_idx = 8;
    let register_cmd = RegisterCommand::System(SystemRegisterCommand {
        header: SystemCommandHeader {
            process_identifier,
            msg_ident,
            read_ident,
            sector_idx,
        },
        content: SystemRegisterCommandContent::ReadProc
    });
    let mut sink: Vec<u8> = Vec::new();

    // when
    serialize_register_command(&register_cmd, &mut sink, &[0x00_u8; 32])
        .await
        .expect("Could not serialize?");
    let mut slice: &[u8] = &sink[..];
    let data_read: &mut (dyn tokio::io::AsyncRead + Send + Unpin) = &mut slice;
    let (deserialized_cmd, hmac_valid) =
        deserialize_register_command(data_read, &[0x00_u8; 64], &[0x00_u8; 32])
            .await
            .expect("Could not deserialize");

    // then
    assert!(hmac_valid);
    match deserialized_cmd {
        RegisterCommand::System(SystemRegisterCommand {
            header,
            content: SystemRegisterCommandContent::ReadProc
        }) => {
            assert_eq!(header.process_identifier, process_identifier);
            assert_eq!(header.msg_ident, msg_ident);
            assert_eq!(header.read_ident, read_ident);
            assert_eq!(header.sector_idx, sector_idx);
        }
        _ => panic!("Expected ReadProc command"),
    }
}

#[tokio::test]
#[timeout(200)]
async fn ack_serialize_deserialize_is_identity() {
    // given
    let process_identifier = 3;
    let msg_ident = Uuid::new_v4();
    let read_ident = 2134;
    let sector_idx = 8;
    let register_cmd = RegisterCommand::System(SystemRegisterCommand {
        header: SystemCommandHeader {
            process_identifier,
            msg_ident,
            read_ident,
            sector_idx,
        },
        content: SystemRegisterCommandContent::Ack
    });
    let mut sink: Vec<u8> = Vec::new();

    // when
    serialize_register_command(&register_cmd, &mut sink, &[0x00_u8; 32])
        .await
        .expect("Could not serialize?");
    let mut slice: &[u8] = &sink[..];
    let data_read: &mut (dyn tokio::io::AsyncRead + Send + Unpin) = &mut slice;
    let (deserialized_cmd, hmac_valid) =
        deserialize_register_command(data_read, &[0x00_u8; 64], &[0x00_u8; 32])
            .await
            .expect("Could not deserialize");

    // then
    assert!(hmac_valid);
    match deserialized_cmd {
        RegisterCommand::System(SystemRegisterCommand {
            header,
            content: SystemRegisterCommandContent::Ack
        }) => {
            assert_eq!(header.process_identifier, process_identifier);
            assert_eq!(header.msg_ident, msg_ident);
            assert_eq!(header.read_ident, read_ident);
            assert_eq!(header.sector_idx, sector_idx);
        }
        _ => panic!("Expected Ack command"),
    }
}

#[tokio::test]
#[timeout(200)]
async fn value_serialize_deserialize_is_identity() {
    // given
    let process_identifier = 3;
    let msg_ident = Uuid::new_v4();
    let read_ident = 2134;
    let sector_idx = 8;
    let timestamp = 1112;
    let write_rank = 3;
    let sector_data = get_random_sector_vec();
    let register_cmd = RegisterCommand::System(SystemRegisterCommand {
        header: SystemCommandHeader {
            process_identifier,
            msg_ident,
            read_ident,
            sector_idx,
        },
        content: SystemRegisterCommandContent::Value {
            timestamp,
            write_rank,
            sector_data: sector_data.clone(),
        }
    });
    let mut sink: Vec<u8> = Vec::new();

    // when
    serialize_register_command(&register_cmd, &mut sink, &[0x00_u8; 32])
        .await
        .expect("Could not serialize?");
    let mut slice: &[u8] = &sink[..];
    let data_read: &mut (dyn tokio::io::AsyncRead + Send + Unpin) = &mut slice;
    let (deserialized_cmd, hmac_valid) =
        deserialize_register_command(data_read, &[0x00_u8; 64], &[0x00_u8; 32])
            .await
            .expect("Could not deserialize");

    // then
    assert!(hmac_valid);
    match deserialized_cmd {
        RegisterCommand::System(SystemRegisterCommand {
            header,
            content: SystemRegisterCommandContent::Value {
                timestamp: ts,
                write_rank: wr,
                sector_data: sd,
            },
        }) => {
            assert_eq!(header.process_identifier, process_identifier);
            assert_eq!(header.msg_ident, msg_ident);
            assert_eq!(header.read_ident, read_ident);
            assert_eq!(header.sector_idx, sector_idx);
            assert_eq!(ts, timestamp);
            assert_eq!(wr, write_rank);
            assert_eq!(sd, sector_data);
        }
        _ => panic!("Expected Value command"),
    }
}

#[tokio::test]
#[timeout(200)]
async fn write_proc_serialize_deserialize_is_identity() {
    // given
    let process_identifier = 3;
    let msg_ident = Uuid::new_v4();
    let read_ident = 2134;
    let sector_idx = 8;
    let timestamp = 1112;
    let write_rank = 3;
    let data_to_write = get_random_sector_vec();
    let register_cmd = RegisterCommand::System(SystemRegisterCommand {
        header: SystemCommandHeader {
            process_identifier,
            msg_ident,
            read_ident,
            sector_idx,
        },
        content: SystemRegisterCommandContent::WriteProc {
            timestamp,
            write_rank,
            data_to_write: data_to_write.clone(),
        }
    });
    let mut sink: Vec<u8> = Vec::new();

    // when
    serialize_register_command(&register_cmd, &mut sink, &[0x00_u8; 32])
        .await
        .expect("Could not serialize?");
    let mut slice: &[u8] = &sink[..];
    let data_read: &mut (dyn tokio::io::AsyncRead + Send + Unpin) = &mut slice;
    let (deserialized_cmd, hmac_valid) =
        deserialize_register_command(data_read, &[0x00_u8; 64], &[0x00_u8; 32])
            .await
            .expect("Could not deserialize");

    // then
    assert!(hmac_valid);
    match deserialized_cmd {
        RegisterCommand::System(SystemRegisterCommand {
            header,
            content: SystemRegisterCommandContent::WriteProc {
                timestamp: ts,
                write_rank: wr,
                data_to_write: dtw,
            },
        }) => {
            assert_eq!(header.process_identifier, process_identifier);
            assert_eq!(header.msg_ident, msg_ident);
            assert_eq!(header.read_ident, read_ident);
            assert_eq!(header.sector_idx, sector_idx);
            assert_eq!(ts, timestamp);
            assert_eq!(wr, write_rank);
            assert_eq!(dtw, data_to_write);
        }
        _ => panic!("Expected WriteProc command"),
    }
}
