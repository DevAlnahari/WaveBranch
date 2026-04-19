use std::net::TcpStream;
use std::path::Path;
use std::fs;

use crate::error::WaveBranchError;
use crate::network::protocol::{read_message, send_message, NetCommand, CloneResponse, FilePayload};

pub fn clone_repo(url: &str) -> Result<(), WaveBranchError> {
    let mut stream = TcpStream::connect(url)
        .map_err(|e| WaveBranchError::NetworkError(format!("Failed to connect to {}: {}", url, e)))?;
        
    send_message(&mut stream, &NetCommand::Clone)?;
    let response: CloneResponse = read_message(&mut stream)?;
    
    let current_dir = std::env::current_dir()?;
    let repo_dir = current_dir.join(".wavebranch");
    
    if repo_dir.exists() {
        return Err(WaveBranchError::RepoAlreadyExists);
    }
    
    fs::create_dir_all(&repo_dir)?;
    
    for file in response.files {
        let abs_path = repo_dir.join(&file.rel_path);
        if let Some(parent) = abs_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&abs_path, file.content)?;
    }
    
    println!("Successfully cloned from {}", url);
    

    let head_path = repo_dir.join("HEAD");
    if head_path.exists() {
        let head_content = fs::read_to_string(&head_path)?.trim().to_string();
        let target_hash = if head_content.starts_with("ref: ") {
            let ref_path = head_content.trim_start_matches("ref: ");
            let abs_ref_path = repo_dir.join(ref_path);
            if abs_ref_path.exists() {
                fs::read_to_string(abs_ref_path)?.trim().to_string()
            } else {
                String::new()
            }
        } else {
            head_content
        };
        
        if !target_hash.is_empty() {
            crate::core::reset::reset_to_commit(&target_hash)?;
            println!("Checked out HEAD precisely to {}", target_hash);
        }
    }
    
    Ok(())
}

pub fn push_to_remote(url: &str) -> Result<(), WaveBranchError> {
    let repo_dir = std::env::current_dir()?.join(".wavebranch");
    if !repo_dir.exists() {
        return Err(WaveBranchError::RepoNotFound);
    }
    
    let mut stream = TcpStream::connect(url)
        .map_err(|e| WaveBranchError::NetworkError(format!("Failed to connect to {}: {}", url, e)))?;
        
    let head_path = repo_dir.join("HEAD");
    let head_content = fs::read_to_string(&head_path)?.trim().to_string();
    let current_hash = if head_content.starts_with("ref: ") {
        let ref_path = head_content.trim_start_matches("ref: ");
        let abs_ref_path = repo_dir.join(ref_path);
        if abs_ref_path.exists() {
            fs::read_to_string(abs_ref_path)?.trim().to_string()
        } else {
            return Err(WaveBranchError::ObjectError("HEAD lacks active commit points".to_string()));
        }
    } else {
        head_content.clone()
    };
    
    send_message(&mut stream, &NetCommand::Push { new_head: current_hash.clone() })?;
    
    let response: NetCommand = read_message(&mut stream)?;
    if let NetCommand::Error(e) = response {
        return Err(WaveBranchError::NetworkError(e));
    }
    
    let mut files = Vec::new();
    collect_files_recursive(&repo_dir, &repo_dir, &mut files)?;
    
    let payload = CloneResponse { files };
    send_message(&mut stream, &payload)?;
    
    let final_res: NetCommand = read_message(&mut stream)?;
    if let NetCommand::Error(e) = final_res {
        return Err(WaveBranchError::NetworkError(e));
    }

    println!("Successfully pushed current HEAD [{}] to {}", current_hash, url);
    Ok(())
}

pub fn pull_from_remote(url: &str) -> Result<(), WaveBranchError> {
    let repo_dir = std::env::current_dir()?.join(".wavebranch");
    if !repo_dir.exists() {
        return Err(WaveBranchError::RepoNotFound);
    }
    
    let mut stream = TcpStream::connect(url)
        .map_err(|e| WaveBranchError::NetworkError(format!("Failed to connect to {}: {}", url, e)))?;
        
    send_message(&mut stream, &NetCommand::Pull)?;
    let response: CloneResponse = read_message(&mut stream)?;
    
    for file in response.files {
        let abs_path = repo_dir.join(&file.rel_path);
        if let Some(parent) = abs_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&abs_path, file.content)?;
    }
    
    let head_path = repo_dir.join("HEAD");
    if head_path.exists() {
        let head_content = fs::read_to_string(&head_path)?.trim().to_string();
        let target_hash = if head_content.starts_with("ref: ") {
            let ref_path = head_content.trim_start_matches("ref: ");
            let abs_ref_path = repo_dir.join(ref_path);
            if abs_ref_path.exists() {
                fs::read_to_string(abs_ref_path)?.trim().to_string()
            } else {
                String::new()
            }
        } else {
            head_content
        };
        
        if !target_hash.is_empty() {
            crate::core::reset::reset_to_commit(&target_hash)?;
            println!("Successfully pulled from {}. Working tree mapped to {}", url, target_hash);
        }
    }
    
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
