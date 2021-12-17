use log::error;

use crate::{SectorVec, SectorIdx};


pub static PAGE_SIZE: usize = 4096;
static MAX_DESCRIPTORS: usize = 1024;
static MAX_CLIENTS: usize = 16;
static HMAC_KEY_SIZE: usize = 32;
pub static WORKERS_COUNT: u64 = 4; // todo zwiekszyc na 256

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