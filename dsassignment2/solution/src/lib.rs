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
    use uuid::Uuid;

    use crate::{utils::*, SectorVec, ClientRegisterCommandContent, ClientRegisterCommand, OperationComplete, SystemRegisterCommand, SystemCommandHeader, SystemRegisterCommandContent, Broadcast, StableStorage, RegisterClient, SectorsManager};
    use std::collections::{HashMap, HashSet};
    use std::convert::TryInto;
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
        // todo osobno folder na kazdy atomic register
        // todo inicjalizować całe stable storage na 0 
        /// Send system command to the register.
        async fn system_command(&mut self, cmd: SystemRegisterCommand);
    }


    #[async_trait::async_trait]
    impl AtomicRegister for MyAtomicRegister {
        async fn client_command(
            &mut self,
            cmd: ClientRegisterCommand,
            operation_complete: Box<
                dyn FnOnce(OperationComplete) -> Pin<Box<dyn Future<Output = ()> + Send>>
                    + Send
                    + Sync,
            >,
        ) {
            let sector_idx = cmd.header.sector_idx;
            self.recover_sector(sector_idx).await;
            match cmd.content {
                // upon event < nnar, Read > do
                //     rid := rid + 1;
                //     store(rid);
                //     readlist := [ _ ] `of length` N;
                //     acklist := [ _ ] `of length` N;
                //     reading := TRUE;
                //     trigger < sbeb, Broadcast | [READ_PROC, rid] >;
                ClientRegisterCommandContent::Read => {
                    self.rid += 1;
                    self.metadata.put("rid", &self.rid.to_be_bytes()).await.unwrap();
                    self.readlist = HashSet::new();
                    self.acklist = HashSet::new();
                    self.reading = true;
                    self.register_client.broadcast(
                        Broadcast {
                            cmd: Arc::new(
                                SystemRegisterCommand {
                                    header: SystemCommandHeader {
                                        process_identifier: self.self_ident,
                                        msg_ident: Uuid::new_v4(),
                                        read_ident: self.rid, 
                                        sector_idx,
                                    },
                                    content: SystemRegisterCommandContent::ReadProc
                                }
                            )
                        }
                    ).await;
                }
                // upon event < nnar, Write | v > do
                //     rid := rid + 1;
                //     writeval := v;
                //     acklist := [ _ ] `of length` N;
                //     readlist := [ _ ] `of length` N;
                //     writing := TRUE;
                //     store(rid);
                //     trigger < sbeb, Broadcast | [READ_PROC, rid] >;
                ClientRegisterCommandContent::Write {data} => {
                    self.rid += 1;
                    self.writeval = data;
                    self.acklist = HashSet::new();
                    self.readlist = HashSet::new();
                    self.writing = true;
                    self.metadata.put("rid", &self.rid.to_be_bytes()).await.unwrap();
                    self.register_client.broadcast(
                        Broadcast {
                            cmd: Arc::new(
                                SystemRegisterCommand {
                                    header: SystemCommandHeader {
                                        process_identifier: self.self_ident,
                                        msg_ident: Uuid::new_v4(),
                                        read_ident: self.rid, 
                                        sector_idx,
                                    },
                                    content: SystemRegisterCommandContent::ReadProc
                                }
                            )
                        }
                    ).await;
                }
            }
        }

        /// Send system command to the register.
        async fn system_command(&mut self, cmd: SystemRegisterCommand) {
            unimplemented!();
        }
    }
    impl MyAtomicRegister {
        // check if sector was already recovered
        // if not put data from stable storage to local hashmaps
        async fn recover_sector(&mut self, sector_idx: u64) {
            if self.recovered_sectors.contains(&sector_idx) {
                return;
            }
            let (ts, wr) = self.sectors_manager.read_metadata(sector_idx).await;
            self.recovered_sectors.insert(sector_idx);
            self.ts.insert(sector_idx, ts);
            self.wr.insert(sector_idx, wr);
        }
    }

    pub(crate) struct MyAtomicRegister {
        self_ident: u8,
        metadata: Box<dyn StableStorage>,
        register_client: Arc<dyn RegisterClient>,
        sectors_manager: Arc<dyn SectorsManager>,
        processes_count: usize,
        rid: u64,
        ts: HashMap<u64, u64>,
        wr: HashMap<u64, u8>,
        readlist: HashSet<u64>,
        acklist: HashSet<u64>,
        readval: SectorVec,
        writeval: SectorVec,
        reading: bool,
        writing: bool,
        write_phase: bool,
        recovered_sectors: HashSet<u64>,
        
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
        // Init
        let mut atomic_register = Box::new(
            MyAtomicRegister {
                self_ident,
                metadata,
                register_client, 
                sectors_manager,
                processes_count,
                rid: 0u64,
                ts: HashMap::new(),
                wr: HashMap::new(),
                readlist: HashSet::new(),
                acklist: HashSet::new(),
                readval: empty_sector(),
                writeval: empty_sector(),
                reading: false,
                writing: false,
                write_phase: false,
                recovered_sectors: HashSet::new(),
            }
        );
        // Recovery of rid
        atomic_register.rid = u64::from_be_bytes(
            atomic_register.metadata.get("rid").await.unwrap_or([0u8; 1].to_vec())[..].try_into().unwrap()
        );
        atomic_register
    }
}

pub mod sectors_manager_public {

    use crate::utils::{get_filename_from_idx};
    use crate::{SectorIdx, SectorVec, MyStableStorage, StableStorage};
    use std::convert::TryInto;
    use std::path::PathBuf;
    use std::sync::Arc;

    struct MySectorsManager {
        path: PathBuf,
        stable_storage: MyStableStorage,
    }

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
        Arc::new(MySectorsManager { 
            path: path.clone(),
            stable_storage: MyStableStorage { path }
        })
    }   

    #[async_trait::async_trait] 
    impl SectorsManager for MySectorsManager {
        async fn read_data(&self, idx: SectorIdx) -> SectorVec {
            let filename = get_filename_from_idx(idx);
            let data = self.stable_storage.get(&filename).await.unwrap();
            let data_bytes = &data[9..];
            SectorVec(data_bytes.to_vec())
        }

        async fn read_metadata(&self, idx: SectorIdx) -> (u64, u8) {
            let filename = get_filename_from_idx(idx);
            let data = self.stable_storage.get(&filename).await.unwrap();
            let ts_bytes = &data[0..8];
            let ts = u64::from_be_bytes(ts_bytes.try_into().unwrap());
            let wr = data[8];
            (ts, wr)
        }

        // key: <worker_id>_<sector_in_worker_id>
        // data: ts (8 bytes), wr (1 byte), sector_data (4096 bytes)
        async fn write(&self, idx: SectorIdx, sector: &(SectorVec, u64, u8)) {
            let (SectorVec(sector_data), ts, wr) = sector;
            let ts_bytes = &ts.to_be_bytes()[..];
            let wr_bytes = &wr.to_be_bytes()[..];
            let sector_data_bytes = sector_data.as_slice();
            let metadata_with_data = [ts_bytes, wr_bytes, sector_data_bytes].concat();
            let filename = get_filename_from_idx(idx);
            self.stable_storage.put_non_mut(&filename, &metadata_with_data).await.unwrap();
        }
    }
}

pub mod transfer_public {
    use crate::{utils::*, HmacSha256, ClientCommandHeader, ClientRegisterCommand, ClientRegisterCommandContent, OperationComplete, SystemRegisterCommandContent, SystemRegisterCommand, SystemCommandHeader, OperationReturn};
    use crate::{RegisterCommand, MAGIC_NUMBER};
    use std::convert::TryInto;
    use std::io::{Error, ErrorKind};
    use bytes::{BytesMut, BufMut};
    use log::{error};
    use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};
    use hmac::{NewMac, Mac};
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
                let message_without_hmac = [magic_buffer, type_buffer, buffer[0..16].to_vec()].concat();
                let mut mac = HmacSha256::new_from_slice(hmac_client_key).unwrap();
                mac.update(&message_without_hmac[..]);
                let result = mac.finalize().into_bytes();
                let hmac_valid = result.as_slice() == hmac;
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
                let request_number = &buffer[0..8];
                let sector_index = &buffer[8..16];
                let sector_data = &buffer[16..(16+PAGE_SIZE)];
                let hmac = &buffer[(16+PAGE_SIZE)..(16+PAGE_SIZE+32)];
                let message_without_hmac = [magic_buffer, type_buffer, buffer[0..(16+PAGE_SIZE)].to_vec()].concat();
                let mut mac = HmacSha256::new_from_slice(hmac_client_key).unwrap();
                mac.update(&message_without_hmac[..]);
                let result = mac.finalize().into_bytes();
                let hmac_valid = result.as_slice() == hmac;
                return Ok((RegisterCommand::Client(
                    ClientRegisterCommand {
                        header: ClientCommandHeader {
                            request_identifier: u64::from_be_bytes(request_number.try_into().unwrap()),
                            sector_idx: u64::from_be_bytes(sector_index.try_into().unwrap()),
                        },
                        content: ClientRegisterCommandContent::Write {
                            data: SectorVec(sector_data.to_vec())
                        }
                    }
                ), hmac_valid));
            }
            MessageType::ReadProc | MessageType::Ack => {
                let process_identifier = type_buffer[2];
                let msg_ident = &buffer[0..16];
                let read_ident = &buffer[16..24];
                let sector_index = &buffer[24..32];
                let hmac = &buffer[32..64];
                let message_without_hmac = [magic_buffer, type_buffer, buffer[0..32].to_vec()].concat();
                let mut mac = HmacSha256::new_from_slice(hmac_system_key).unwrap();
                mac.update(&message_without_hmac[..]);
                let result = mac.finalize().into_bytes();
                let hmac_valid = result.as_slice() == hmac;
                return Ok((RegisterCommand::System(
                    SystemRegisterCommand {
                        header: SystemCommandHeader {
                            process_identifier,
                            msg_ident: Uuid::from_slice(&msg_ident).unwrap(),
                            read_ident: u64::from_be_bytes(read_ident.try_into().unwrap()),
                            sector_idx: u64::from_be_bytes(sector_index.try_into().unwrap()),
                        },
                        content: match message_type {
                            MessageType::ReadProc => SystemRegisterCommandContent::ReadProc,
                            MessageType::Ack => SystemRegisterCommandContent::Ack,
                            _ => { return Err(Error::new(ErrorKind::InvalidData, "wrong message type")); }
                        } 
                    }
                ), hmac_valid));
            }
            MessageType::WriteProc | MessageType::Value => {
                let process_identifier = type_buffer[2];
                let msg_ident = &buffer[0..16];
                let read_ident = &buffer[16..24];
                let sector_index = &buffer[24..32];
                let timestamp = &buffer[32..40];
                let write_rank = buffer[47];
                let sector_data = &buffer[48..(48+PAGE_SIZE)];
                let hmac = &buffer[(48+PAGE_SIZE)..(48+PAGE_SIZE+32)];
                let message_without_hmac = [magic_buffer, type_buffer, buffer[0..(48+PAGE_SIZE)].to_vec()].concat();
                let mut mac = HmacSha256::new_from_slice(hmac_system_key).unwrap();
                mac.update(&message_without_hmac[..]);
                let result = mac.finalize().into_bytes();
                let hmac_valid = result.as_slice() == hmac;
                return Ok((RegisterCommand::System(
                    SystemRegisterCommand {
                        header: SystemCommandHeader {
                            process_identifier,
                            msg_ident: Uuid::from_slice(&msg_ident).unwrap(),
                            read_ident: u64::from_be_bytes(read_ident.try_into().unwrap()),
                            sector_idx: u64::from_be_bytes(sector_index.try_into().unwrap()),
                        },
                        content: match message_type {
                            MessageType::WriteProc => 
                                SystemRegisterCommandContent::WriteProc {
                                    timestamp: u64::from_be_bytes(timestamp.try_into().unwrap()),
                                    write_rank,
                                    data_to_write: SectorVec(sector_data.to_vec()),
                                },
                            MessageType::Value => 
                                SystemRegisterCommandContent::Value {
                                    timestamp: u64::from_be_bytes(timestamp.try_into().unwrap()),
                                    write_rank,
                                    sector_data: SectorVec(sector_data.to_vec()),
                                },
                            _ => { return Err(Error::new(ErrorKind::InvalidData, "wrong message type")); }
                        }
                    }       
                ), hmac_valid));
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
                // put content of message
                match content { 
                    SystemRegisterCommandContent::ReadProc | SystemRegisterCommandContent::Ack => {
                        // no content
                    },
                    SystemRegisterCommandContent::Value { timestamp, write_rank, sector_data } => {
                        buf.put_u64(timestamp);
                        buf.put_slice(&[0; 7]);
                        buf.put_u8(write_rank);
                        if sector_data.len() != PAGE_SIZE {
                            return Err(Error::new(ErrorKind::InvalidData, "sector length should be 4096 bytes"));
                        }
                        let mut d = sector_data.clone();
                        buf.put_slice(&d.to_array());
                    },
                    SystemRegisterCommandContent::WriteProc { timestamp, write_rank, data_to_write } => {
                        buf.put_u64(timestamp);
                        buf.put_slice(&[0; 7]);
                        buf.put_u8(write_rank);
                        if data_to_write.len() != PAGE_SIZE {
                            return Err(Error::new(ErrorKind::InvalidData, "sector length should be 4096 bytes"));
                        }
                        let mut d = data_to_write.clone();
                        buf.put_slice(&d.to_array());
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
        return writer.write_all(&buf).await;
    }

    // todo zmienić visibility tego
    pub async fn serialize_operation_complete_command(
        cmd: &OperationComplete,
        writer: &mut (dyn AsyncWrite + Send + Unpin),
        hmac_key: &[u8],
    ) -> Result<(), Error> {
        let mut buf = BytesMut::new();
        buf.put_slice(&MAGIC_NUMBER);
        buf.put_slice(&[0; 2]);
        buf.put_u8(cmd.status_code as u8);
        buf.put_u8(
            match cmd.op_return { 
                OperationReturn::Read(_) => MessageType::ReadResponse as u8,
                OperationReturn::Write => MessageType::WriteResponse as u8
            }
        );
        buf.put_u64(cmd.request_identifier);
        // put content of message
        match &cmd.op_return {
            OperationReturn::Read(read_return) => {
                match &read_return.read_data {
                    Some(data) => {
                        let mut d = data.clone();
                        buf.put_slice(&d.to_array());
                    }
                    None => {
                        // no content
                    }
                }
            }
            OperationReturn::Write => {
                // no content
            }
        }
        // put hmac of message to the end of message
        let mut mac = HmacSha256::new_from_slice(hmac_key).unwrap();
        mac.update(&buf);
        let result = mac.finalize().into_bytes();
        buf.put_slice(&result);
        // send the message
        return writer.write_all(&buf).await;
    }
}


pub mod register_client_public {
    use log::error;
    use tokio::{net::TcpStream, io::AsyncWriteExt};

    use crate::{SystemRegisterCommand, HmacSha256, RegisterCommand, serialize_register_command, SystemRegisterCommandContent};
    use std::{sync::Arc, io::Write};

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

    pub(crate) struct MyRegisterClient {
        pub(crate) hmac_system_key: [u8; 64],
        pub(crate) tcp_addrs: Vec<(String, u16)>,
    }

    #[async_trait::async_trait]
    impl RegisterClient for MyRegisterClient {
        async fn send(&self, msg: Send) {
            let mut writer = vec![];
            match serialize_register_command(
                &RegisterCommand::System((&*msg.cmd).clone()),
                writer.by_ref(), 
                &self.hmac_system_key,
            ).await {
                Ok(_) => {
                    let addr = self.tcp_addrs.get(msg.target-1).unwrap();
                    let stream = TcpStream::connect(addr).await;
                    match stream {
                        Ok(mut socket) => {
                            socket.write_all(writer.as_slice()).await.ok();
                        }
                        Err(err) => {
                            error!("send(): couln't send to socket: {}", err);
                        }
                    }
                }
                Err(err) => {
                    error!("error during send() to target {} in serializing: {}", msg.target, err);
                }
            }
        }

        // todo sprytniejsza logika do broadcasta (przesyłanie READ_PROC i WRITE_PROC wielokrotnie aż do ACK)
        async fn broadcast(&self, msg: Broadcast) {
            for target in 1..=self.tcp_addrs.len() {
                self.send(Send {
                    cmd: msg.cmd.clone(),
                    target,
                }).await;
            }
        }
    }
}


// todo nieprzetestowane
pub mod stable_storage_public {
    use core::num;
    use std::{path::PathBuf, collections::{HashMap, HashSet}, os::unix::prelude::OsStrExt, mem};
    use sha2::{Sha256, Digest};
    use tokio::{fs::{File, rename, read}, io::AsyncWriteExt};

    use crate::utils::*;

    /// A helper trait for small amount of durable metadata needed by the register algorithm
    /// itself. Again, it is only for AtomicRegister definition. StableStorage in unit tests
    /// is durable, as one could expect.
    #[async_trait::async_trait]
    pub trait StableStorage: Send + Sync {
        async fn put(&mut self, key: &str, value: &[u8]) -> Result<(), String>;

        async fn get(&self, key: &str) -> Option<Vec<u8>>;
    }
    
    fn get_hash(name: &str) -> String {
        let mut sha256 = Sha256::new();
        sha256.update(name);
        let hash = sha256.finalize();
        return format!("{:X}", hash);
    }

    pub(crate) struct MyStableStorage {
        pub(crate) path: PathBuf,
    }

    impl MyStableStorage {
        pub(crate) async fn put_non_mut(&self, key: &str, value: &[u8]) -> Result<(), String> {
            let filename = get_hash(key);
            let tmp_filename = "tmp_".to_owned() + &filename;
            let mut tmp_path = self.path.clone();
            let mut path = self.path.clone();
            tmp_path.push(tmp_filename);
            path.push(filename);
            let mut file = File::create(tmp_path.clone()).await.unwrap();
            file.write_all(value).await.unwrap();
            file.sync_data().await.unwrap();
            rename(tmp_path, path).await.unwrap();
            file.sync_data().await.unwrap();
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl StableStorage for MyStableStorage {
        async fn put(&mut self, key: &str, value: &[u8]) -> Result<(), String> {
            self.put_non_mut(key, value).await
        }

        async fn get(&self, key: &str) -> Option<Vec<u8>> {
            let filename = get_hash(key);
            let mut path = self.path.clone();
            path.push(filename);
            match read(path).await {
                Ok(value) => Some(value),
                Err(_) => None,
            }
        }

    }
    
}
