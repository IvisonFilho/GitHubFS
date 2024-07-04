use fuser::{FileAttr, FileType, Filesystem, Request, ReplyAttr, ReplyData, ReplyEntry, ReplyDirectory, ReplyXattr};
use libc::{EINVAL, ENOENT, ENODATA};
use log::{debug, error, info};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::{self, ErrorKind};
use std::time::{Duration, UNIX_EPOCH};
use std::os::unix::ffi::OsStrExt;
use fuser::KernelConfig;

const GITHUB_API_URL: &str = "https://api.github.com";

#[derive(Debug, Deserialize)]
pub struct GitHubRepository {
    name: String,
    full_name: String,
}

#[derive(Debug, Deserialize, Clone)]
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
    next_inode: u64,
}

impl GitHubFS {
    pub fn new(username: String, token: String) -> io::Result<Self> {
        info!("Initializing GitHubFS for user: {}", username);

        let mut fs = Self {
            client: Client::new(),
            username,
            token,
            repos: HashMap::new(),
            files: HashMap::new(),
            next_inode: 2, // Start from 2 as 1 is reserved for root
        };

        // Fetch and load repositories during initialization
        let repos = fs.fetch_repositories()?;
        let mut repo_inodes = Vec::new();
        for (index, repo) in repos.into_iter().enumerate() {
            let inode = index as u64 + 2; // Inode starts from 2
            fs.repos.insert(inode, repo);
            repo_inodes.push(inode);
        }

        // Load all repositories' root files
        for &repo_inode in &repo_inodes {
            if let Err(err) = fs.load_files(repo_inode, "") {
                error!("Failed to load root directory files for inode {}: {}", repo_inode, err);
            }
        }

        info!("Initialized with {} repositories", fs.repos.len());
        Ok(fs)
    }

    pub fn fetch_repositories(&self) -> Result<Vec<GitHubRepository>, io::Error> {
        let api_url = format!("{}/user/repos", GITHUB_API_URL);
        debug!("Fetching repositories from URL: {}", api_url);

        let response = self.client.get(&api_url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", "GitHubFS")
            .send()
            .map_err(|err| {
                error!("Failed to send request to GitHub API: {}", err);
                io::Error::new(io::ErrorKind::Other, format!("Failed to send request to GitHub API: {}", err))
            })?;

        if response.status().is_success() {
            let repos = response.json::<Vec<GitHubRepository>>()
                .map_err(|err| {
                    error!("Failed to parse JSON response: {}", err);
                    io::Error::new(io::ErrorKind::Other, format!("Failed to parse JSON response: {}", err))
                })?;
            debug!("Fetched {} repositories", repos.len());
            Ok(repos)
        } else {
            let status = response.status();
            let error_message = response.text().unwrap_or_else(|_| "No additional error message".to_string());
            let full_error_message = format!("GitHub API request failed with status {}: {}", status, error_message);
            error!("{}", full_error_message);
            Err(io::Error::new(io::ErrorKind::Other, full_error_message))
        }
    }

    fn fetch_file_content(&self, repo_full_name: &str, path: &str) -> Result<Vec<u8>, io::Error> {
        let api_url = format!("{}/repos/{}/contents/{}", GITHUB_API_URL, repo_full_name, path);
        debug!("Fetching file content from URL: {}", api_url);

        let response = self.client.get(&api_url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", "GitHubFS")
            .send()
            .map_err(|err| {
                error!("Failed to send request to GitHub API: {}", err);
                io::Error::new(io::ErrorKind::Other, format!("Failed to send request to GitHub API: {}", err))
            })?;

        if response.status().is_success() {
            let content = response.json::<GitHubFileContent>()
                .map_err(|err| {
                    error!("Failed to parse JSON response: {}", err);
                    io::Error::new(io::ErrorKind::Other, format!("Failed to parse JSON response: {}", err))
                })?;
            if content.encoding == "base64" {
                base64::decode(&content.content)
                    .map_err(|err| {
                        error!("Failed to decode base64 content: {}", err);
                        io::Error::new(io::ErrorKind::Other, format!("Failed to decode base64 content: {}", err))
                    })
            } else {
                error!("Unknown content encoding: {}", content.encoding);
                Err(io::Error::new(io::ErrorKind::Other, format!("Unknown content encoding: {}", content.encoding)))
            }
        } else {
            let status = response.status();
            let error_message = response.text().unwrap_or_else(|_| "No additional error message".to_string());
            let full_error_message = format!("GitHub API request failed with status {}: {}", status, error_message);
            error!("{}", full_error_message);
            Err(io::Error::new(io::ErrorKind::Other, full_error_message))
        }
    }

    fn next_inode(&mut self) -> u64 {
        let inode = self.next_inode;
        self.next_inode += 1;
        inode
    }

    pub fn load_files(&mut self, repo_id: u64, path: &str) -> io::Result<Vec<GitHubFile>> {
        let repo = self.repos.get(&repo_id).ok_or_else(|| io::Error::new(ErrorKind::NotFound, "Repository not found"))?;
        let api_url = format!("{}/repos/{}/contents/{}", GITHUB_API_URL, repo.full_name, path);
        debug!("Fetching files from URL: {}", api_url);
    
        let response = self.client.get(&api_url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", "GitHubFS")
            .send()
            .map_err(|err| {
                error!("Failed to send request to GitHub API: {}", err);
                io::Error::new(io::ErrorKind::Other, format!("Failed to send request to GitHub API: {}", err))
            })?;
    
        if response.status().is_success() {
            let files = response.json::<Vec<GitHubFile>>()
                .map_err(|err| {
                    error!("Failed to parse JSON response: {}", err);
                    io::Error::new(io::ErrorKind::Other, format!("Failed to parse JSON response: {}", err))
                })?;
            
            debug!("Fetched {} files", files.len());
    
            let mut loaded_files = Vec::new();
    
            for file in &files {
                let inode = self.next_inode();
                loaded_files.push((inode, file.clone()));
    
                if file.file_type == "dir" {
                    let sub_files = self.load_files(repo_id, &file.path)?;
                    self.files.insert(inode, sub_files);
                }
            }
    
            self.files.insert(repo_id, files.clone());
            Ok(loaded_files.into_iter().map(|(_, file)| file).collect())
        } else {
            let status = response.status();
            let error_message = response.text().unwrap_or_else(|_| "No additional error message".to_string());
            let full_error_message = format!("GitHub API request failed with status {}: {}", status, error_message);
            error!("{}", full_error_message);
            Err(io::Error::new(io::ErrorKind::Other, full_error_message))
        }
    }
    
    

    fn attr(&self, ino: u64) -> io::Result<FileAttr> {
        let kind = if ino == 1 || self.repos.contains_key(&ino) {
            FileType::Directory
        } else {
            FileType::RegularFile
        };

        Ok(FileAttr {
            ino,
            size: 0,
            blocks: 1,
            atime: UNIX_EPOCH,
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind,
            perm: 0o755,
            nlink: 2,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 512, // Adicionando o campo blksize
            flags: 0,
        })
    }
}

impl Filesystem for GitHubFS {
    fn init(&mut self, _req: &Request<'_>, _config: &mut KernelConfig) -> Result<(), libc::c_int> {
        info!("GitHubFS initialized");
    
        // Verifica se há pelo menos um repositório carregado
        if let Some((&repo_id, _)) = self.repos.iter().next() {
            // Carrega os arquivos e diretórios do primeiro repositório carregado
            if let Err(err) = self.load_files(repo_id, "") {
                error!("Failed to load root directory files: {}", err);
            }
        } else {
            error!("No repositories loaded");
            return Err(libc::ENOENT);
        }
    
        Ok(())
    }
    

    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        debug!("lookup(parent: {}, name: {:?})", parent, name);

        if parent == 1 {
            // Root directory, look for repositories
            if let Some((&inode, _repo)) = self.repos.iter().find(|(_inode, repo)| OsStr::new(&repo.name) == name) {
                reply.entry(&Duration::new(1, 0), &self.attr(inode).unwrap(), 0);
                return;
            }
        } else {
            // Look for files in repositories
            if let Some(files) = self.files.get(&parent) {
                for file in files {
                    if OsStr::new(&file.name) == name {
                        let inode = self.next_inode();
                        reply.entry(&Duration::new(1, 0), &self.attr(inode).unwrap(), 0);
                        return;
                    }
                }
            }
        }

        reply.error(ENOENT);
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        debug!("getattr(ino: {})", ino);

        match self.attr(ino) {
            Ok(attr) => reply.attr(&Duration::new(1, 0), &attr),
            Err(_) => reply.error(ENOENT),
        }
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        debug!("readdir(ino: {}, offset: {})", ino, offset);
    
        if offset != 0 {
            reply.ok();
            return;
        }
    
        reply.add(ino, 1, FileType::Directory, ".");
        reply.add(ino, 2, FileType::Directory, "..");
    
        if ino == 1 {
            for (inode, repo) in &self.repos {
                reply.add(*inode, *inode as i64, FileType::Directory, &repo.name);
            }
        } else if let Some(files) = self.files.get(&ino) {
            // Criar uma cópia dos arquivos para evitar problemas de mutabilidade
            let files = files.clone();
    
            // Iterar sobre os arquivos sem a necessidade de mutar self.files diretamente
            for (i, file) in files.iter().enumerate() {
                let kind = if file.file_type == "dir" { FileType::Directory } else { FileType::RegularFile };
                let inode = self.next_inode();
    
                // Inserir o arquivo no HashMap usando uma nova entrada de vetor
                self.files.insert(inode, vec![file.clone()]);
    
                // Adicionar a entrada ao reply
                reply.add(inode, (i + 3) as i64, kind, &file.name);
            }
        }
    
        reply.ok();
    }
    
    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        debug!("read(ino: {}, offset: {}, size: {})", ino, offset, size);

        for files in self.files.values() {
            if let Some(file) = files.iter().find(|file| file.path == ino.to_string()) {
                if let Some(ref download_url) = file.download_url {
                    match self.fetch_file_content(&file.path, &file.path) {
                        Ok(content) => {
                            let data = &content[offset as usize..std::cmp::min(content.len(), (offset + size as i64) as usize)];
                            reply.data(data);
                        }
                        Err(err) => {
                            error!("Failed to fetch file content: {}", err);
                            reply.error(ENOENT);
                        }
                    }
                } else {
                    reply.error(ENOENT);
                }
                return;
            }
        }

        reply.error(ENOENT);
    }
}
