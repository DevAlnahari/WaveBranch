use std::io::{Read, Write};
use std::net::TcpStream;
use serde::{Deserialize, Serialize};

use crate::error::WaveBranchError;

#[derive(Serialize, Deserialize, Debug)]
pub enum NetCommand {
    Clone,
    Push { new_head: String },
    Pull,
    Ok,
    Error(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FilePayload {
    pub rel_path: String,
    pub content: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CloneResponse {
    pub files: Vec<FilePayload>,
}

/// Sends a length-prefixed protocol message.
pub fn send_message<T: Serialize>(stream: &mut TcpStream, payload: &T) -> Result<(), WaveBranchError> {
    let serialized = serde_json::to_vec(payload)
        .map_err(|e| WaveBranchError::NetworkError(e.to_string()))?;
        
    let len_prefix = (serialized.len() as u32).to_be_bytes();
    
    stream.write_all(&len_prefix)
        .map_err(|e| WaveBranchError::NetworkError(e.to_string()))?;
        
    stream.write_all(&serialized)
        .map_err(|e| WaveBranchError::NetworkError(e.to_string()))?;
        
    stream.flush()
        .map_err(|e| WaveBranchError::NetworkError(e.to_string()))?;
        
    Ok(())
}

/// Reads a length-prefixed protocol message.
pub fn read_message<T: for<'a> Deserialize<'a>>(stream: &mut TcpStream) -> Result<T, WaveBranchError> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)
        .map_err(|e| WaveBranchError::NetworkError(e.to_string()))?;
        
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut data_buf = vec![0u8; len];
    
    stream.read_exact(&mut data_buf)
        .map_err(|e| WaveBranchError::NetworkError(e.to_string()))?;
        
    let msg: T = serde_json::from_slice(&data_buf)
        .map_err(|e| WaveBranchError::NetworkError(e.to_string()))?;
        
    Ok(msg)
}

