use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

#[derive(Debug)]
pub(crate) enum ImageError {
    InvalidGptHeaderRevision(u32),
    InvalidGptHeaderSignature(Vec<u8>),
    InvalidGptHeaderSize(u32),
    InvalidPartitionEntry(String),
    InvalidPartitionType { expected: String, actual: String },
    InvalidSignature([u8; 2]),
}

impl Display for ImageError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Self::InvalidGptHeaderRevision(rev) => write!(f, "Invalid GPT header revision: 0x{:04x}", rev),
            Self::InvalidGptHeaderSignature(sig) => {
                f.write_str("Invalid GPT header signature: ")?;
                for b in sig {
                    write!(f, "{:02x}", b)?
                }
                Ok(())
            }
            Self::InvalidGptHeaderSize(size) => write!(f, "Invalid GPT header size: {}", size),
            Self::InvalidPartitionEntry(msg) => write!(f, "Invalid partition entry: {}", msg),
            Self::InvalidPartitionType { expected, actual } => {
                write!(f, "Invalid partition type; expected {}, actual {}", expected, actual)
            }
            Self::InvalidSignature(sig) => {
                write!(f, "Invalid signature: expected [0x55, 0xaa], actual {}", hex::encode(sig))
            }
        }
    }
}

impl Error for ImageError {}
