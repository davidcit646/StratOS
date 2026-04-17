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
}

impl Message {
    pub fn new(sender_id: u32, opcode: u16, args: Vec<Argument>) -> Self {
        let mut length = 0u16;
        for arg in &args {
            length += arg.size();
        }
        Message {
            header: MessageHeader {
                sender_id,
                opcode,
                length,
            },
            args,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        // Serialize header
        bytes.extend_from_slice(&self.header.sender_id.to_le_bytes());
        bytes.extend_from_slice(&self.header.opcode.to_le_bytes());
        bytes.extend_from_slice(&self.header.length.to_le_bytes());
        
        // Serialize arguments
        for arg in &self.args {
            arg.serialize_to(&mut bytes);
        }
        
        // Pad to 32-bit boundary
        while bytes.len() % 4 != 0 {
            bytes.push(0);
        }
        
        bytes
    }

    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }
        
        let sender_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let opcode = u16::from_le_bytes([data[4], data[5]]);
        let length = u16::from_le_bytes([data[6], data[7]]) as usize;
        
        if data.len() < 8 + length {
            return None;
        }
        
        let args_data = &data[8..8 + length];
        let args = Argument::deserialize_args(args_data);
        
        Some(Message {
            header: MessageHeader {
                sender_id,
                opcode,
                length: length as u16,
            },
            args,
        })
    }
}

impl Argument {
    pub fn size(&self) -> u16 {
        match self {
            Argument::Uint(_) => 4,
            Argument::Int(_) => 4,
            Argument::Fixed(_) => 4,
            Argument::String(s) => ((4 + s.len() + 1 + 3) & !3) as u16, // 4-byte length + string + null + padding
            Argument::Object(_) => 4 as u16,
            Argument::NewId(_) => 4,
            Argument::Array(v) => ((4 + v.len() + 3) & !3) as u16, // 4-byte length + data + padding
            Argument::Fd(_) => 4,
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
            Argument::Fd(_) => bytes.extend_from_slice(&0u32.to_le_bytes()), // FDs sent via SCM_RIGHTS
        }
    }

    pub fn deserialize_args(data: &[u8]) -> Vec<Argument> {
        let mut args = Vec::new();
        let mut offset = 0;
        
        while offset < data.len() {
            if offset + 4 > data.len() {
                break;
            }
            
            let value = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;
            
            // Simplified: treat all as Uint for now
            // Full implementation would need type information from protocol
            args.push(Argument::Uint(value));
        }
        
        args
    }
}
