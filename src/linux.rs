use std::io::Result;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, RawFd};
use std::path::Path;

use crate::io_uring;
use crate::unix;

#[derive(Debug)]
enum LinuxFile {
    Uring(io_uring::File),
    Pos(unix::File),
}

#[derive(Debug)]
pub struct File(LinuxFile);

impl File {
    pub(crate) async fn open_with_options(
        options: &tokio::fs::OpenOptions,
        path: impl AsRef<Path>,
    ) -> Result<Self> {
        match io_uring::File::open_with_options(options, path.as_ref()).await {
            Ok(file) => Ok(Self(LinuxFile::Uring(file))),
            Err(e) if matches!(e.kind(), std::io::ErrorKind::Unsupported) => Ok(Self(
                LinuxFile::Pos(unix::File::open_with_options(options, path.as_ref()).await?),
            )),
            Err(e) => Err(e),
        }
    }

    pub async fn create(path: impl AsRef<Path>) -> Result<Self> {
        match io_uring::File::create(path.as_ref()).await {
            Ok(file) => Ok(Self(LinuxFile::Uring(file))),
            Err(e) if matches!(e.kind(), std::io::ErrorKind::Unsupported) => Ok(Self(
                LinuxFile::Pos(unix::File::create(path.as_ref()).await?),
            )),
            Err(e) => Err(e),
        }
    }

    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        match io_uring::File::open(path.as_ref()).await {
            Ok(file) => Ok(Self(LinuxFile::Uring(file))),
            Err(e) if matches!(e.kind(), std::io::ErrorKind::Unsupported) => {
                Ok(Self(LinuxFile::Pos(unix::File::open(path.as_ref()).await?)))
            }
            Err(e) => Err(e),
        }
    }

    pub async fn metadata(&self) -> Result<std::fs::Metadata> {
        match &self.0 {
            LinuxFile::Uring(file) => file.metadata().await,
            LinuxFile::Pos(file) => file.metadata().await,
        }
    }

    pub async fn write_at(&self, pos: u64, buf: &[u8]) -> Result<usize> {
        match &mut self.0 {
            LinuxFile::Uring(file) => file.write_at(pos, buf).await,
            LinuxFile::Pos(file) => file.write_at(pos, buf).await,
        }
    }

    pub async fn read_at(&self, pos: u64, buf: &mut [u8]) -> Result<usize> {
        match &mut self.0 {
            LinuxFile::Uring(file) => file.read_at(pos, buf).await,
            LinuxFile::Pos(file) => file.read_at(pos, buf).await,
        }
    }

    pub async fn sync_all(&self) -> Result<()> {
        match &mut self.0 {
            LinuxFile::Uring(file) => file.sync_all().await,
            LinuxFile::Pos(file) => file.sync_all().await,
        }
    }

    pub async fn sync_data(&self) -> Result<()> {
        match &mut self.0 {
            LinuxFile::Uring(file) => file.sync_data().await,
            LinuxFile::Pos(file) => file.sync_data().await,
        }
    }

    pub async fn set_len(&self, size: u64) -> Result<()> {
        match &mut self.0 {
            LinuxFile::Uring(file) => file.set_len(size).await,
            LinuxFile::Pos(file) => file.set_len(size).await,
        }
    }
}

impl From<tokio::fs::File> for File {
    fn from(file: tokio::fs::File) -> Self {
        match io_uring::init_uring() {
            Ok(_) => Self(LinuxFile::Uring(unsafe {
                io_uring::File::unsafe_from_file(file)
            })),
            Err(_) => Self(LinuxFile::Pos(unix::File::from(file))),
        }
    }
}

impl From<std::fs::File> for File {
    fn from(file: std::fs::File) -> Self {
        let file: tokio::fs::File = file.into();
        file.into()
    }
}

impl AsFd for File {
    fn as_fd(&self) -> BorrowedFd<'_> {
        match &self.0 {
            LinuxFile::Uring(file) => file.as_fd(),
            LinuxFile::Pos(file) => file.as_fd(),
        }
    }
}

impl AsRawFd for File {
    fn as_raw_fd(&self) -> RawFd {
        match &self.0 {
            LinuxFile::Uring(file) => file.as_raw_fd(),
            LinuxFile::Pos(file) => file.as_raw_fd(),
        }
    }
}

impl FromRawFd for File {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        match io_uring::init_uring() {
            Ok(_) => Self(LinuxFile::Uring(unsafe {
                io_uring::File::unsafe_from_raw_fd(fd)
            })),
            Err(_) => Self(LinuxFile::Pos(unix::File::from_raw_fd(fd))),
        }
    }
}
