use clap::Parser;
use fuser::MountOption;
use log::debug;
use std::io::{self};
use std::fs::create_dir_all;
use std::path::PathBuf;

mod fs;

#[derive(Parser, Debug)]
struct Args {
    /// The owner of the GitHub repository.
    owner: String,

    /// The name of the GitHub repository.
    repo: String,

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

    // Construct the mount point directory
    let mountpoint = PathBuf::from("/mnt/githubfs"); // Substitute with your desired mount directory path
    ensure_mountpoint(&mountpoint)?;

    // Example of where to store or how to obtain the GitHub access token
    let github_token = "Your-Token".to_string(); // Substitute with your GitHub personal access token

    let fs = fs::GitHubFS::new(
        args.owner.clone(),
        github_token,
    );

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
        Err(e) => eprintln!("Failed to mount filesystem: {:?}", e),
    };

    Ok(())
}
