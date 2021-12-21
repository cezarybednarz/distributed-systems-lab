use std::{convert::TryInto, collections::HashMap, mem};

use log::error;

use crate::{SectorVec, SectorIdx};


pub static PAGE_SIZE: usize = 4096;
pub static WORKERS_COUNT: u64 = 4; // todo zwiekszyc na 256
static HMAC_KEY_SIZE: usize = 32;
static MAX_DESCRIPTORS: usize = 1024;
static MAX_CLIENTS: usize = 16;

pub(crate) fn empty_sector() -> SectorVec {
    SectorVec([0u8; 4096].to_vec())
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MessageType {
    Error, 
    Read,
    Write,
    ReadProc,
    Value,
    WriteProc,
    Ack,
    ReadResponse = 0x41,
    WriteResponse,
    ReadProcResponse,
    ValueResponse,
    WriteProcResponse,
    AckResponse,
}

impl MessageType {
    // message length without first 8 bytes
    pub(crate) fn content_size(&mut self) -> usize {
        match self {
            MessageType::Error => 0,
            MessageType::Read => 16 + HMAC_KEY_SIZE,
            MessageType::Write => 16 + PAGE_SIZE + HMAC_KEY_SIZE,
            MessageType::ReadProc => 32 + HMAC_KEY_SIZE,
            MessageType::Value => 48 + PAGE_SIZE + HMAC_KEY_SIZE,
            MessageType::WriteProc => 48 + PAGE_SIZE + HMAC_KEY_SIZE,
            MessageType::Ack => 32 + HMAC_KEY_SIZE,
            _ => {
                error!("wrong usage of content_size()");
                0
            }
        }
    }
}

impl From<u8> for MessageType {
    fn from(val: u8) -> Self {
        match val {
            b if b == MessageType::Read as u8 => MessageType::Read,
            b if b == MessageType::Write as u8 => MessageType::Write,
            b if b == MessageType::ReadProc as u8 => MessageType::ReadProc,
            b if b == MessageType::Value as u8 => MessageType::Value,
            b if b == MessageType::WriteProc as u8 => MessageType::WriteProc,
            b if b == MessageType::Ack as u8 => MessageType::Ack,
            b if b == MessageType::ReadResponse as u8 => MessageType::ReadResponse,
            b if b == MessageType::WriteResponse as u8 => MessageType::WriteResponse,
            b if b == MessageType::ReadProcResponse as u8 => MessageType::ReadProcResponse,
            b if b == MessageType::ValueResponse as u8 => MessageType::ValueResponse,
            b if b == MessageType::WriteProcResponse as u8 => MessageType::WriteProcResponse,
            b if b == MessageType::AckResponse as u8 => MessageType::AckResponse,
            _ => MessageType::Error,
        }
    }
}

impl SectorVec {
    pub(crate) fn to_array(&mut self) -> &[u8] {
        match self {
            SectorVec(vec) => {
                return &vec[..];
            }
        }
    }
    pub(crate) fn len(&self) -> usize {
        match self {
            SectorVec(vec) => {
                return vec.len();
            }
        }
    }
}

pub(crate) fn get_worker_id(idx: SectorIdx) -> u64 {
    idx % WORKERS_COUNT
}

pub(crate) fn get_sector_in_worker_id(idx: SectorIdx) -> u64 {
    idx / WORKERS_COUNT
}

pub(crate) fn get_filename_from_idx(idx: SectorIdx) -> String {
    get_worker_id(idx).to_string() + &String::from("_") + &get_sector_in_worker_id(idx).to_string()
}

// serialize hashmaps and hashsets
pub(crate) fn serialize_hashmap_u64_u64(map: HashMap<u64, u64>) -> Vec<u8> {
    let mut data = vec![];
    for (key, value) in &map {
        data.extend(key.to_be_bytes().iter().cloned());
        data.extend(value.to_be_bytes().iter().cloned());
    }
    data
}

pub(crate) fn serialize_hashmap_u64_u8(map: HashMap<u64, u8>) -> Vec<u8> {
    let mut data = vec![];
    for (key, value) in &map {
        data.extend(key.to_be_bytes().iter().cloned());
        data.extend(value.to_be_bytes().iter().cloned());
    }
    data
}

pub(crate) fn serialize_hashmap_u64_sector_vec(map: HashMap<u64, SectorVec>) -> Vec<u8> {
    let mut data = vec![];
    for (key, SectorVec(value)) in &map {
        data.extend(key.to_be_bytes().iter().cloned());
        data.extend(value.iter().cloned());
    }
    data
}

// deserialize hashmaps and hashsets
pub(crate) fn deserialize_hashmap_u64_u64(bytes: Vec<u8>) -> HashMap<u64, u64> {
    let mut map = HashMap::new();
    let bytes_chunks = bytes.chunks_exact(8+8);
    for bytes_chunk in bytes_chunks {
        let key = &bytes_chunk[..8];
        let val = &bytes_chunk[8..16];
        map.insert(
            u64::from_be_bytes(key.try_into().unwrap()),
            u64::from_be_bytes(val.try_into().unwrap())
        );
    }
    map
}

pub(crate) fn deserialize_hashmap_u64_u8(bytes: Vec<u8>) -> HashMap<u64, u8> {
    let mut map = HashMap::new();
    let bytes_chunks = bytes.chunks_exact(8+1);
    for bytes_chunk in bytes_chunks {
        let key = &bytes_chunk[..8];
        let val = &bytes_chunk[8..9];
        map.insert(
            u64::from_be_bytes(key.try_into().unwrap()),
            u8::from_be_bytes(val.try_into().unwrap())
        );
    }
    map
}

pub(crate) fn deserialize_hashmap_u64_sector_vec(bytes: Vec<u8>) -> HashMap<u64, SectorVec> {
    let mut map = HashMap::new();
    let bytes_chunks = bytes.chunks_exact(8+PAGE_SIZE);
    for bytes_chunk in bytes_chunks {
        let key = &bytes_chunk[..8];
        let val = &bytes_chunk[8..(8+PAGE_SIZE)];
        map.insert(
            u64::from_be_bytes(key.try_into().unwrap()),
            SectorVec(val.try_into().unwrap())
        );
    }
    map
}