use std::io::Result;
use std::path::Path;

#[cfg(unix)]
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, RawFd};

#[cfg(windows)]
use std::os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, RawHandle};

mod options;

#[cfg(target_os = "linux")]
pub(crate) mod linux;

#[cfg(target_os = "linux")]
pub(crate) mod io_uring;

#[cfg(any(
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub(crate) mod bsd;

#[cfg(all(
    unix,
    not(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    ))
))]
pub(crate) mod unix;

#[cfg(target_os = "windows")]
pub(crate) mod windows;

#[cfg(target_os = "linux")]
use linux::File as FileImpl;

#[cfg(all(
    unix,
    not(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd"
    ))
))]
use unix::File as FileImpl;

#[cfg(any(
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
use bsd::File as FileImpl;

#[cfg(target_os = "windows")]
use windows::File as FileImpl;

pub use options::OpenOptions;

pub struct File(FileImpl);

impl File {
    pub(crate) async fn open_with_options(
        options: &tokio::fs::OpenOptions,
        path: impl AsRef<Path>,
    ) -> Result<Self> {
        Ok(Self(FileImpl::open_with_options(options, path).await?))
    }

    pub async fn create(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self(FileImpl::create(path).await?))
    }

    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self(FileImpl::open(path).await?))
    }

    pub async fn metadata(&self) -> Result<std::fs::Metadata> {
        self.0.metadata().await
    }

    pub async fn write_at(&self, pos: u64, buf: &[u8]) -> Result<usize> {
        self.0.write_at(pos, buf).await
    }

    pub async fn read_at(&self, pos: u64, buf: &mut [u8]) -> Result<usize> {
        self.0.read_at(pos, buf).await
    }

    pub async fn sync_all(&self) -> Result<()> {
        self.0.sync_all().await
    }

    pub async fn sync_data(&self) -> Result<()> {
        self.0.sync_data().await
    }

    pub async fn set_len(&self, size: u64) -> Result<()> {
        self.0.set_len(size).await
    }
}

impl From<tokio::fs::File> for File {
    fn from(file: tokio::fs::File) -> Self {
        Self(file.into())
    }
}

impl From<std::fs::File> for File {
    fn from(file: std::fs::File) -> Self {
        Self(file.into())
    }
}

#[cfg(unix)]
impl AsFd for File {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

#[cfg(unix)]
impl AsRawFd for File {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

#[cfg(unix)]
impl FromRawFd for File {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(FileImpl::from_raw_fd(fd))
    }
}

#[cfg(windows)]
impl AsHandle for File {
    fn as_handle(&self) -> BorrowedHandle<'_> {
        self.0.as_handle()
    }
}

#[cfg(windows)]
impl AsRawHandle for File {
    fn as_raw_handle(&self) -> RawHandle {
        self.0.as_raw_handle()
    }
}

#[cfg(windows)]
impl FromRawHandle for File {
    unsafe fn from_raw_handle(handle: RawHandle) -> Self {
        Self(FileImpl::from_raw_handle(handle))
    }
}
