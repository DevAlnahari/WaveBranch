use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::fs;

use crate::error::WaveBranchError;
use crate::network::protocol::{read_message, send_message, NetCommand, FilePayload, CloneResponse};

pub fn start_server(port: u16) -> Result<(), WaveBranchError> {
    let address = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&address)
        .map_err(|e| WaveBranchError::NetworkError(e.to_string()))?;
        
    println!("WaveBranch server listening on {}", address);

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("Accepted connection from: {:?}", stream.peer_addr());
                if let Err(e) = handle_connection(&mut stream) {
                    eprintln!("Connection error: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
            }
        }
    }
    
    Ok(())
}

fn handle_connection(stream: &mut TcpStream) -> Result<(), WaveBranchError> {
    let repo_root = std::env::current_dir()
        .map_err(|e| WaveBranchError::IoError(e))?
        .join(".wavebranch");
        
    if !repo_root.exists() {
        return Err(WaveBranchError::RepoNotFound);
    }

    loop {
        match read_message::<NetCommand>(stream) {
            Ok(cmd) => {
                match cmd {
                    NetCommand::Clone => {
                        println!("Handling Clone request");
                        handle_clone(stream, &repo_root)?;
                    }
                    NetCommand::Push { new_head } => {
                        println!("Handling Push request (new_head: {})", new_head);
                        handle_push(stream, &repo_root, &new_head)?;
                    }
                    NetCommand::Pull => {
                        println!("Handling Pull request");
                        handle_pull(stream, &repo_root)?;
                    }
                    _ => {
                        let _ = send_message(stream, &NetCommand::Error("Unsupported command".to_string()));
                        break;
                    }
                }
            }
            Err(_) => {

                println!("Client disconnected");
                break;
            }
        }
    }
    
    Ok(())
}

fn handle_clone(stream: &mut TcpStream, repo_root: &Path) -> Result<(), WaveBranchError> {
    let mut files = Vec::new();
    collect_files_recursive(repo_root, repo_root, &mut files)?;
    
    let response = CloneResponse { files };
    send_message(stream, &response)?;
    Ok(())
}

fn collect_files_recursive(base: &Path, current: &Path, files: &mut Vec<FilePayload>) -> Result<(), WaveBranchError> {
    if current.is_dir() {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                collect_files_recursive(base, &path, files)?;
            } else {
                let rel_path = path.strip_prefix(base)
                    .map_err(|e| WaveBranchError::PathError(e.to_string()))?
                    .to_string_lossy()
                    .replace("\\", "/");
                
                let content = fs::read(&path)?;
                files.push(FilePayload {
                    rel_path,
                    content,
                });
            }
        }
    }
    Ok(())
}

fn handle_push(stream: &mut TcpStream, repo_root: &Path, new_head: &str) -> Result<(), WaveBranchError> {
    send_message(stream, &NetCommand::Ok)?;
    let pushed_data: CloneResponse = read_message(stream)?;
    
    for file in pushed_data.files {
        let target_path = repo_root.join(&file.rel_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target_path, file.content)?;
    }
    
    let head_path = repo_root.join("HEAD");
    if head_path.exists() {
        let head_content = fs::read_to_string(&head_path)?
            .trim()
            .to_string();
            
        if head_content.starts_with("ref: ") {
            let ref_path = head_content.trim_start_matches("ref: ");
            let abs_ref_path = repo_root.join(ref_path);
            if let Some(parent) = abs_ref_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(abs_ref_path, new_head)?;
        } else {
            fs::write(&head_path, new_head)?;
        }
    }
    
    send_message(stream, &NetCommand::Ok)?;
    Ok(())
}

fn handle_pull(stream: &mut TcpStream, repo_root: &Path) -> Result<(), WaveBranchError> {
    let mut files = Vec::new();
    collect_files_recursive(repo_root, repo_root, &mut files)?;
    
    let response = CloneResponse { files };
    send_message(stream, &response)?;
    Ok(())
}
