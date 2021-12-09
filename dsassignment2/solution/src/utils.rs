use log::error;


static PAGE_SIZE: usize = 4096;
static MAX_DESCRIPTORS: usize = 1024;
static MAX_CLIENTS: usize = 16;
static HMAC_KEY_SIZE: usize = 32;

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MessageType {
    Error, // todo może Ok i Error na końcu
    Read,
    Write,
    ReadProc,
    Value,
    WriteProc,
    Ack,
}

impl MessageType {
    pub fn content_size(&mut self) -> usize {
        match self {
            MessageType::Error => 0,
            MessageType::Read => 16 + HMAC_KEY_SIZE,
            MessageType::Write => 16 + PAGE_SIZE + HMAC_KEY_SIZE,
            MessageType::ReadProc => 32 + HMAC_KEY_SIZE,
            MessageType::Value => 48 + PAGE_SIZE + HMAC_KEY_SIZE,
            MessageType::WriteProc => 48 + PAGE_SIZE + HMAC_KEY_SIZE,
            MessageType::Ack => 32 + HMAC_KEY_SIZE,
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
            _ => MessageType::Error,
        }
    }
}