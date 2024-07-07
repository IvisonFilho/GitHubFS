use clap::Parser;
use fuser::MountOption;
use log::{debug, error, info};
use std::io::{self, Error, ErrorKind};
use std::fs::create_dir_all;
use std::path::PathBuf;

mod fs;

#[derive(Parser, Debug)]
struct Args {
    /// The owner of the GitHub repository.
    owner: String,

    /// The filesystem options.
    #[arg(short, long)]
    options: Vec<String>,
}

fn ensure_mountpoint(mountpoint: &PathBuf) -> io::Result<()> {
    if !mountpoint.exists() {
        create_dir_all(mountpoint)?;
    }
    Ok(())
}

fn main() -> io::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let mountpoint = PathBuf::from("/mnt/githubfs");
    ensure_mountpoint(&mountpoint)?;

    let github_token = "Seu token".to_string();

    let mut fs = fs::GitHubFS::new(args.owner.clone(), github_token)?;

    // Carrega repositórios no início
    if let Err(e) = fs.fetch_repositories() {
        error!("Error loading repositories: {:?}", e);
        return Err(e);
    }

    let mut options = Vec::new();
    for opt in &args.options {
        debug!("Parsing option {}", opt);
        let fsopt = match opt.as_str() {
            "dev" => MountOption::Dev,
            "nodev" => MountOption::NoDev,
            "suid" => MountOption::Suid,
            "nosuid" => MountOption::NoSuid,
            "ro" => MountOption::RO,
            "exec" => MountOption::Exec,
            "noexec" => MountOption::NoExec,
            "atime" => MountOption::Atime,
            "noatime" => MountOption::NoAtime,
            "dirsync" => MountOption::DirSync,
            "sync" => MountOption::Sync,
            "async" => MountOption::Async,
            "rw" => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "GitHubFS filesystem must be read-only",
                ));
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unknown option ({})", opt),
                ));
            }
        };
        options.push(fsopt);
    }

    debug!("Mounting filesystem at {:?}", mountpoint);
    match fuser::mount2(fs, &mountpoint, &options) {
        Ok(_) => println!("Filesystem mounted successfully"),
        Err(e) => {
            error!("Failed to mount filesystem: {:?}", e);
            return Err(e);
        }
    };

    Ok(())
}
