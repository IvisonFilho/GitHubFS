use fuser::{FileType, Request};
use std::io::{self, ErrorKind};
use std::time::{Duration, SystemTime};
use libc::ENOENT;
use std::ffi::OsStr;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::HashMap;

const GITHUB_API_URL: &str = "https://api.github.com";

#[derive(Debug, Deserialize)]
struct GitHubRepository {
    name: String,
    full_name: String,
}

#[derive(Debug, Deserialize)]
struct GitHubFile {
    name: String,
    path: String,
    #[serde(rename = "type")]
    file_type: String,
    #[serde(rename = "download_url")]
    download_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubFileContent {
    content: String,
    encoding: String,
}

pub struct GitHubFS {
    client: Client,
    username: String,
    token: String,
    repos: HashMap<u64, GitHubRepository>,
    files: HashMap<u64, Vec<GitHubFile>>,
}

impl GitHubFS {
    pub fn new(username: String, token: String) -> Self {
        Self {
            client: Client::new(),
            username,
            token,
            repos: HashMap::new(),
            files: HashMap::new(),
        }
    }

    fn fetch_repositories(&self) -> Result<Vec<GitHubRepository>, io::Error> {
        let api_url = format!("{}/users/{}/repos", GITHUB_API_URL, self.username);
        let response = self.client.get(&api_url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("Failed to send request to GitHub API: {}", err)))?;

        if response.status().is_success() {
            response.json::<Vec<GitHubRepository>>()
                .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("Failed to parse JSON response: {}", err)))
        } else {
            Err(io::Error::new(io::ErrorKind::Other, format!("GitHub API request failed with status: {}", response.status())))
        }
    }

    fn fetch_files(&self, repo_full_name: &str) -> Result<Vec<GitHubFile>, io::Error> {
        let api_url = format!("{}/repos/{}/contents", GITHUB_API_URL, repo_full_name);
        let response = self.client.get(&api_url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("Failed to send request to GitHub API: {}", err)))?;

        if response.status().is_success() {
            response.json::<Vec<GitHubFile>>()
                .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("Failed to parse JSON response: {}", err)))
        } else {
            Err(io::Error::new(io::ErrorKind::Other, format!("GitHub API request failed with status: {}", response.status())))
        }
    }

    fn fetch_file_content(&self, repo_full_name: &str, path: &str) -> Result<Vec<u8>, io::Error> {
        let api_url = format!("{}/repos/{}/contents/{}", GITHUB_API_URL, repo_full_name, path);
        let response = self.client.get(&api_url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("Failed to send request to GitHub API: {}", err)))?;

        if response.status().is_success() {
            let content = response.json::<GitHubFileContent>()
                .map_err(|err| io::Error::new(io::ErrorKind::Other, format!("Failed to parse JSON response: {}", err)))?;
            
            let decoded_content = if content.encoding == "base64" {
                base64::decode(content.content.trim()).map_err(|err| io::Error::new(io::ErrorKind::Other, format!("Failed to decode base64 content: {}", err)))?
            } else {
                return Err(io::Error::new(io::ErrorKind::Other, "Unsupported content encoding"));
            };
            Ok(decoded_content)
        } else {
            Err(io::Error::new(io::ErrorKind::Other, format!("GitHub API request failed with status: {}", response.status())))
        }
    }

    fn load_repositories(&mut self) -> io::Result<()> {
        let repos = self.fetch_repositories()?;
        for (i, repo) in repos.into_iter().enumerate() {
            self.repos.insert((i + 2) as u64, repo);
        }
        Ok(())
    }

    fn load_files(&mut self, repo_ino: u64) -> io::Result<()> {
        let repo = self.repos.get(&repo_ino).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Repository not found"))?;
        let files = self.fetch_files(&repo.full_name)?;
        self.files.insert(repo_ino, files);
        Ok(())
    }
}

impl fuser::Filesystem for GitHubFS {
    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, _offset: i64, mut reply: fuser::ReplyDirectory) {
        if ino == 1 {
            if let Err(err) = self.load_repositories() {
                eprintln!("Error loading repositories: {:?}", err);
                reply.error(ENOENT);
                return;
            }

            for (&ino, repo) in &self.repos {
                if reply.add(ino, 0, FileType::Directory, OsStr::new(&repo.name)) {
                    continue;
                } else {
                    break;
                }
            }
            reply.ok();
        } else {
            if self.repos.contains_key(&ino) && self.load_files(ino).is_ok() {
                if let Some(files) = self.files.get(&ino) {
                    for (i, file) in files.iter().enumerate() {
                        let file_type = if file.file_type == "dir" {
                            FileType::Directory
                        } else {
                            FileType::RegularFile
                        };
                        if reply.add(ino * 1000 + (i + 1) as u64, 0, file_type, OsStr::new(&file.name)) {
                            continue;
                        } else {
                            break;
                        }
                    }
                }
                reply.ok();
            } else {
                reply.error(ENOENT);
            }
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: fuser::ReplyAttr) {
        let kind = if ino == 1 || self.repos.contains_key(&ino) {
            FileType::Directory
        } else if let Some(files) = self.files.get(&(ino / 1000)) {
            if files.iter().any(|file| file.path == format!("{}", (ino % 1000) as usize)) {
                FileType::RegularFile
            } else {
                reply.error(ENOENT);
                return;
            }
        } else {
            reply.error(ENOENT);
            return;
        };

        let attr = fuser::FileAttr {
            ino,
            size: 0,
            blocks: 0,
            atime: SystemTime::now(),
            mtime: SystemTime::now(),
            ctime: SystemTime::now(),
            crtime: SystemTime::now(),
            kind,
            perm: if kind == FileType::Directory { 0o755 } else { 0o644 },
            nlink: 2,
            uid: _req.uid(),
            gid: _req.gid(),
            rdev: 0,
            flags: 0,
            blksize: 4096,
        };

        reply.attr(&Duration::from_secs(1), &attr);
    }

    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, size: u32, _flags: i32, _lock: Option<u64>, reply: fuser::ReplyData) {
        let repo_ino = ino / 1000;
        let file_index = (ino % 1000) as usize - 1;

        if let Some(files) = self.files.get(&repo_ino) {
            if let Some(file) = files.get(file_index) {
                let repo = self.repos.get(&repo_ino).expect("Repository not found");
                match self.fetch_file_content(&repo.full_name, &file.path) {
                    Ok(content) => {
                        let start = offset as usize;
                        let end = std::cmp::min(start + size as usize, content.len());
                        reply.data(&content[start..end]);
                    }
                    Err(err) => {
                        eprintln!("Error reading file {}: {:?}", file.path, err);
                        reply.error(ENOENT);
                    }
                }
            } else {
                reply.error(ENOENT);
            }
        } else {
            reply.error(ENOENT);
        }
    }
}
