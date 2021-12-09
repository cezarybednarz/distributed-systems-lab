mod domain;
mod utils;

pub use crate::domain::*;
pub use atomic_register_public::*;
use hmac::Hmac;
pub use register_client_public::*;
pub use sectors_manager_public::*;
use sha2::Sha256;
pub use stable_storage_public::*;
pub use transfer_public::*;

type HmacSha256 = Hmac<Sha256>;

pub async fn run_register_process(config: Configuration) {
    unimplemented!()
}

pub mod atomic_register_public {
    use crate::{
        ClientRegisterCommand, OperationComplete, RegisterClient, SectorsManager, StableStorage,
        SystemRegisterCommand,
    };
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;

    #[async_trait::async_trait]
    pub trait AtomicRegister: Send + Sync {
        /// Send client command to the register. After it is completed, we expect
        /// callback to be called. Note that completion of client command happens after
        /// delivery of multiple system commands to the register, as the algorithm specifies.
        async fn client_command(
            &mut self,
            cmd: ClientRegisterCommand,
            operation_complete: Box<
                dyn FnOnce(OperationComplete) -> Pin<Box<dyn Future<Output = ()> + Send>>
                    + Send
                    + Sync,
            >,
        );

        /// Send system command to the register.
        async fn system_command(&mut self, cmd: SystemRegisterCommand);
    }

    /// Idents are numbered starting at 1 (up to the number of processes in the system).
    /// Storage for atomic register algorithm data is separated into StableStorage.
    /// Communication with other processes of the system is to be done by register_client.
    /// And sectors must be stored in the sectors_manager instance.
    pub async fn build_atomic_register(
        self_ident: u8,
        metadata: Box<dyn StableStorage>,
        register_client: Arc<dyn RegisterClient>,
        sectors_manager: Arc<dyn SectorsManager>,
        processes_count: usize,
    ) -> Box<dyn AtomicRegister> {
        unimplemented!()
    }
}

pub mod sectors_manager_public {
    use crate::{SectorIdx, SectorVec};
    use std::path::PathBuf;
    use std::sync::Arc;

    #[async_trait::async_trait]
    pub trait SectorsManager: Send + Sync {
        /// Returns 4096 bytes of sector data by index.
        async fn read_data(&self, idx: SectorIdx) -> SectorVec;

        /// Returns timestamp and write rank of the process which has saved this data.
        /// Timestamps and ranks are relevant for atomic register algorithm, and are described
        /// there.
        async fn read_metadata(&self, idx: SectorIdx) -> (u64, u8);

        /// Writes a new data, along with timestamp and write rank to some sector.
        async fn write(&self, idx: SectorIdx, sector: &(SectorVec, u64, u8));
    }

    /// Path parameter points to a directory to which this method has exclusive access.
    pub fn build_sectors_manager(path: PathBuf) -> Arc<dyn SectorsManager> {
        unimplemented!()
    }
}

pub mod transfer_public {
    use crate::{utils::*, HmacSha256, Configuration, ClientCommandHeader, ClientRegisterCommand, ClientRegisterCommandContent, OperationComplete, SystemRegisterCommandContent, SystemRegisterCommand, SystemCommandHeader};
    use crate::{RegisterCommand, MAGIC_NUMBER};
    use std::convert::TryInto;
    use std::io::{Error, ErrorKind};
    use bytes::{BytesMut, BufMut};
    use log::{debug, error};
    use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};
    use hmac::{NewMac, Mac, Hmac};
    use uuid::Uuid;
    use crate::{SectorVec};

    pub async fn deserialize_register_command(
        data: &mut (dyn AsyncRead + Send + Unpin),
        hmac_system_key: &[u8; 64],
        hmac_client_key: &[u8; 32],
    ) -> Result<(RegisterCommand, bool), Error> {
        // slide over bytes until MAGIC_NUMBER appears
        let mut magic_buffer = vec![0; 4];
        data.read_exact(&mut magic_buffer).await?;
        while magic_buffer != MAGIC_NUMBER {
            let mut magic_byte = vec![0; 1];
            data.read_exact(&mut magic_byte).await?;
            magic_buffer = [magic_buffer, magic_byte].concat();
            magic_buffer.remove(0);
        }
        // check message type
        let mut type_buffer = vec![0; 4];
        data.read_exact(&mut type_buffer).await?;
        let mut message_type = MessageType::from(type_buffer[3]);
        if message_type == MessageType::Error {
            error!("wrong message type: {}", type_buffer[3]);
            return Err(Error::new(ErrorKind::InvalidData, "wrong message type"));
        }
        // read rest of the message according to message_type
        let mut buffer = vec![0; message_type.content_size()];
        data.read_exact(&mut buffer).await?;
        match message_type {
            MessageType::Read  => {
                let request_number = &buffer[0..8];
                let sector_index = &buffer[8..16];
                let hmac = &buffer[16..48];
                let mut mac = HmacSha256::new_from_slice(hmac_client_key).unwrap();
                let message_without_hmac = [magic_buffer, type_buffer, buffer[0..16].to_vec()].concat();
                mac.update(&message_without_hmac[..]);
                let result = mac.finalize().into_bytes();
                let hmac_valid = result.as_slice() == hmac;
                if !hmac_valid {
                    debug!("invalid hmac of message");
                }
                return Ok((RegisterCommand::Client(
                    ClientRegisterCommand {
                        header: ClientCommandHeader {
                            request_identifier: u64::from_be_bytes(request_number.try_into().unwrap()),
                            sector_idx: u64::from_be_bytes(sector_index.try_into().unwrap()),
                        },
                        content: ClientRegisterCommandContent::Read
                    }
                ), hmac_valid));
            }
            MessageType::Write => {
                unimplemented!();
            }
            MessageType::ReadProc => {
                let process_identifier = type_buffer[6];
                let msg_ident = &buffer[0..16];
                let read_ident = &buffer[16..24];
                let sector_index = &buffer[24..32];
                let hmac = &buffer[32..64];
                let mut mac = HmacSha256::new_from_slice(hmac_system_key).unwrap();
                let message_without_hmac = [magic_buffer, type_buffer, buffer[0..32].to_vec()].concat();
                mac.update(&message_without_hmac[..]);
                let result = mac.finalize().into_bytes();
                let hmac_valid = result.as_slice() == hmac;
                if !hmac_valid {
                    debug!("invalid hmac of message");
                }
                return Ok((RegisterCommand::System(
                    SystemRegisterCommand {
                        header: SystemCommandHeader {
                            process_identifier,
                            msg_ident: Uuid::from_slice(&msg_ident).unwrap(),
                            read_ident: u64::from_be_bytes(read_ident.try_into().unwrap()),
                            sector_idx: u64::from_be_bytes(sector_index.try_into().unwrap()),
                        },
                        content: SystemRegisterCommandContent::ReadProc
                    }
                ), hmac_valid));
            }
            MessageType::Value => {
                unimplemented!();
            }
            MessageType::WriteProc => {
                unimplemented!();
            }
            MessageType::Ack => {
                unimplemented!();
            }
            _ => {
                Err(Error::new(ErrorKind::InvalidData, "wrong message type"))
            }
        }
    }

    pub async fn serialize_register_command(
        cmd: &RegisterCommand,
        writer: &mut (dyn AsyncWrite + Send + Unpin),
        hmac_key: &[u8],
    ) -> Result<(), Error> {
        // buffer for whole message
        let mut buf = BytesMut::new();
        match cmd {
            RegisterCommand::System(system_cmd) => {
                let header = system_cmd.clone().header;
                let content = system_cmd.clone().content;
                // put data to header of message into byte buffer
                buf.put_slice(&MAGIC_NUMBER);
                buf.put_slice(&[0; 2]);
                buf.put_u8(header.process_identifier);
                buf.put_u8(
                    match content { 
                        SystemRegisterCommandContent::ReadProc => MessageType::ReadProc as u8,
                        SystemRegisterCommandContent::Ack => MessageType::Ack as u8,
                        SystemRegisterCommandContent::Value { timestamp: _, write_rank: _, sector_data: _} => MessageType::Value as u8,
                        SystemRegisterCommandContent::WriteProc { timestamp: _, write_rank: _, data_to_write: _ } => MessageType::WriteProc as u8,
                    }
                );
                buf.put_slice(header.msg_ident.as_bytes());
                buf.put_slice(&header.read_ident.to_be_bytes());
                buf.put_slice(&header.sector_idx.to_be_bytes());
                // todo walidować wartosci
                // put content of message
                match content { 
                    SystemRegisterCommandContent::ReadProc | SystemRegisterCommandContent::Ack => {
                        // no content
                    },
                    SystemRegisterCommandContent::Value { timestamp, write_rank, sector_data } => {
                        unimplemented!();
                    },
                    SystemRegisterCommandContent::WriteProc { timestamp, write_rank, data_to_write } => {
                        unimplemented!();
                    },
                }
            }
            RegisterCommand::Client(client_cmd) => {
                let header = client_cmd.clone().header;
                let content = client_cmd.clone().content;
                // put data to header of message into byte buffer
                buf.put_slice(&MAGIC_NUMBER);
                buf.put_slice(&[0; 3]);
                buf.put_u8(
                    match content {
                        ClientRegisterCommandContent::Read => MessageType::Read as u8,
                        ClientRegisterCommandContent::Write { data: _ } => MessageType::Write as u8,
                    }
                );
                buf.put_slice(&header.request_identifier.to_be_bytes());
                buf.put_slice(&header.sector_idx.to_be_bytes());
                // todo walidowac wartości
                // put content of message
                match content {
                    ClientRegisterCommandContent::Read => {
                        // no content
                    }
                    ClientRegisterCommandContent::Write { data } => {
                        let mut d = data.clone();
                        buf.put_slice(&d.to_array());
                    }
                }
            }
        }
        // put hmac of message to the end of message
        let mut mac = HmacSha256::new_from_slice(hmac_key).unwrap();
        mac.update(&buf);
        let result = mac.finalize().into_bytes();
        buf.put_slice(&result);
        // send the message
        writer.write_all(&buf).await?;
        Ok(())
    }

    async fn serialize_operation_complete_command(
        cmd: &OperationComplete,
        writer: &mut (dyn AsyncWrite + Send + Unpin),
        hmac_key: &[u8],
    ) -> Result<(), Error> {
        unimplemented!();
    }
}

pub mod register_client_public {
    use crate::SystemRegisterCommand;
    use std::sync::Arc;

    #[async_trait::async_trait]
    /// We do not need any public implementation of this trait. It is there for use
    /// in AtomicRegister. In our opinion it is a safe bet to say some structure of
    /// this kind must appear in your solution.
    pub trait RegisterClient: core::marker::Send + core::marker::Sync {
        /// Sends a system message to a single process.
        async fn send(&self, msg: Send);

        /// Broadcasts a system message to all processes in the system, including self.
        async fn broadcast(&self, msg: Broadcast);
    }

    pub struct Broadcast {
        pub cmd: Arc<SystemRegisterCommand>,
    }

    pub struct Send {
        pub cmd: Arc<SystemRegisterCommand>,
        /// Identifier of the target process. Those start at 1.
        pub target: usize,
    }
}

pub mod stable_storage_public {
    #[async_trait::async_trait]
    /// A helper trait for small amount of durable metadata needed by the register algorithm
    /// itself. Again, it is only for AtomicRegister definition. StableStorage in unit tests
    /// is durable, as one could expect.
    pub trait StableStorage: Send + Sync {
        async fn put(&mut self, key: &str, value: &[u8]) -> Result<(), String>;

        async fn get(&self, key: &str) -> Option<Vec<u8>>;
    }
}
