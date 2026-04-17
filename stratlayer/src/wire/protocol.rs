use std::os::unix::io::RawFd;

#[repr(C)]
pub struct MessageHeader {
    pub sender_id: u32,
    pub opcode: u16,
    pub length: u16,
}

pub enum Argument {
    Uint(u32),
    Int(i32),
    Fixed(i32),
    String(String),
    Object(u32),
    NewId(u32),
    Array(Vec<u8>),
    Fd(RawFd),
}

pub struct Message {
    pub header: MessageHeader,
    pub args: Vec<Argument>,
    pub raw_args: Vec<u8>,
}

impl Message {
    pub fn new(sender_id: u32, opcode: u16, args: Vec<Argument>) -> Self {
        let mut length: u16 = 8;
        for arg in &args {
            length += arg.size();
        }
        Message {
            header: MessageHeader { sender_id, opcode, length },
            args,
            raw_args: Vec::new(),
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.header.sender_id.to_le_bytes());
        bytes.extend_from_slice(&self.header.opcode.to_le_bytes());
        bytes.extend_from_slice(&self.header.length.to_le_bytes());

        for arg in &self.args {
            arg.serialize_to(&mut bytes);
        }

        while bytes.len() % 4 != 0 {
            bytes.push(0);
        }

        bytes
    }

    /// Parse header + capture raw arg bytes; defer typed parsing until interface is known.
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        let sender_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let opcode = u16::from_le_bytes([data[4], data[5]]);
        let length = u16::from_le_bytes([data[6], data[7]]) as usize;

        if length < 8 || data.len() < length {
            return None;
        }

        let raw_args = data[8..length].to_vec();

        Some(Message {
            header: MessageHeader {
                sender_id,
                opcode,
                length: length as u16,
            },
            args: Vec::new(),
            raw_args,
        })
    }

    pub fn parse_args(&self, signature: &str) -> Vec<Argument> {
        Argument::deserialize_args_typed(&self.raw_args, signature)
    }
}

impl Argument {
    pub fn size(&self) -> u16 {
        match self {
            Argument::Uint(_) => 4,
            Argument::Int(_) => 4,
            Argument::Fixed(_) => 4,
            Argument::String(s) => (4 + ((s.len() + 1 + 3) & !3)) as u16,
            Argument::Object(_) => 4,
            Argument::NewId(_) => 4,
            Argument::Array(v) => (4 + ((v.len() + 3) & !3)) as u16,
            Argument::Fd(_) => 0, // FDs travel out-of-band via SCM_RIGHTS
        }
    }

    pub fn serialize_to(&self, bytes: &mut Vec<u8>) {
        match self {
            Argument::Uint(v) => bytes.extend_from_slice(&v.to_le_bytes()),
            Argument::Int(v) => bytes.extend_from_slice(&v.to_le_bytes()),
            Argument::Fixed(v) => bytes.extend_from_slice(&v.to_le_bytes()),
            Argument::String(s) => {
                let len = s.len() + 1;
                bytes.extend_from_slice(&(len as u32).to_le_bytes());
                bytes.extend_from_slice(s.as_bytes());
                bytes.push(0);
                while bytes.len() % 4 != 0 {
                    bytes.push(0);
                }
            }
            Argument::Object(v) => bytes.extend_from_slice(&v.to_le_bytes()),
            Argument::NewId(v) => bytes.extend_from_slice(&v.to_le_bytes()),
            Argument::Array(v) => {
                let len = v.len();
                bytes.extend_from_slice(&(len as u32).to_le_bytes());
                bytes.extend_from_slice(v);
                while bytes.len() % 4 != 0 {
                    bytes.push(0);
                }
            }
            Argument::Fd(_) => {} // nothing inline
        }
    }

    pub fn deserialize_args_typed(data: &[u8], signature: &str) -> Vec<Argument> {
        let mut args = Vec::new();
        let mut offset = 0;

        for type_char in signature.chars() {
            match type_char {
                'u' => {
                    if offset + 4 > data.len() { break; }
                    let v = u32::from_le_bytes([
                        data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
                    ]);
                    args.push(Argument::Uint(v));
                    offset += 4;
                }
                'i' => {
                    if offset + 4 > data.len() { break; }
                    let v = i32::from_le_bytes([
                        data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
                    ]);
                    args.push(Argument::Int(v));
                    offset += 4;
                }
                's' => {
                    if offset + 4 > data.len() { break; }
                    let len = u32::from_le_bytes([
                        data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
                    ]) as usize;
                    offset += 4;
                    if len == 0 {
                        args.push(Argument::String(String::new()));
                        continue;
                    }
                    if offset + len > data.len() { break; }
                    let bytes = &data[offset..offset + len];
                    let null_pos = bytes.iter().position(|&b| b == 0).unwrap_or(len);
                    let s = String::from_utf8_lossy(&bytes[..null_pos]).to_string();
                    args.push(Argument::String(s));
                    offset += len;
                    let pad = (4 - (len % 4)) % 4;
                    offset += pad;
                }
                'o' => {
                    if offset + 4 > data.len() { break; }
                    let v = u32::from_le_bytes([
                        data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
                    ]);
                    args.push(Argument::Object(v));
                    offset += 4;
                }
                'n' => {
                    if offset + 4 > data.len() { break; }
                    let v = u32::from_le_bytes([
                        data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
                    ]);
                    args.push(Argument::NewId(v));
                    offset += 4;
                }
                'a' => {
                    if offset + 4 > data.len() { break; }
                    let len = u32::from_le_bytes([
                        data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
                    ]) as usize;
                    offset += 4;
                    if offset + len > data.len() { break; }
                    let v = data[offset..offset + len].to_vec();
                    args.push(Argument::Array(v));
                    offset += len;
                    let pad = (4 - (len % 4)) % 4;
                    offset += pad;
                }
                'f' => {
                    if offset + 4 > data.len() { break; }
                    let v = i32::from_le_bytes([
                        data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
                    ]);
                    args.push(Argument::Fixed(v));
                    offset += 4;
                }
                _ => {
                    offset += 4;
                }
            }
        }

        args
    }
}
