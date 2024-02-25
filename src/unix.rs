use std::io::Result;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, RawFd};
use std::path::Path;

#[derive(Debug)]
pub struct File(tokio::fs::File);

struct Ptr(*const libc::c_void);

struct MutPtr(*mut libc::c_void);

unsafe impl Send for Ptr {}

unsafe impl Sync for Ptr {}

unsafe impl Send for MutPtr {}

unsafe impl Sync for MutPtr {}

impl File {
    pub(crate) async fn open_with_options(
        options: &tokio::fs::OpenOptions,
        path: impl AsRef<Path>,
    ) -> Result<Self> {
        Ok(Self(options.open(path).await?))
    }

    pub async fn create(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self(tokio::fs::File::create(path).await?))
    }

    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self(tokio::fs::File::open(path).await?))
    }

    pub async fn metadata(&self) -> Result<std::fs::Metadata> {
        self.0.metadata().await
    }

    fn write_at_sync(fd: i32, pos: u64, buf: Ptr, len: usize) -> Result<usize> {
        unsafe {
            let cnt = libc::pwrite(fd, buf.0, len, pos as libc::off_t);
            if cnt < 0 {
                Err(std::io::Error::from_raw_os_error(*libc::__errno_location()))
            } else {
                Ok(cnt as usize)
            }
        }
    }

    fn anchor<T>(&self, _buf: T) {}

    pub async fn write_at(&self, pos: u64, buf: &[u8]) -> Result<usize> {
        let fd = self.0.as_raw_fd();
        let ptr = Ptr(buf.as_ptr() as *const libc::c_void);
        let len = buf.len();
        let ret = tokio::task::spawn_blocking(move || -> Result<usize> {
            Self::write_at_sync(fd, pos, ptr, len)
        })
        .await
        .unwrap();
        self.anchor(buf);
        ret
    }

    fn read_at_sync(fd: i32, pos: u64, buf: MutPtr, len: usize) -> Result<usize> {
        unsafe {
            let cnt = libc::pread(fd, buf.0, len, pos as libc::off_t);
            if cnt < 0 {
                Err(std::io::Error::from_raw_os_error(*libc::__errno_location()))
            } else {
                Ok(cnt as usize)
            }
        }
    }

    pub async fn read_at(&self, pos: u64, buf: &mut [u8]) -> Result<usize> {
        let fd = self.0.as_raw_fd();
        let ptr = MutPtr(buf.as_mut_ptr() as *mut libc::c_void);
        let len = buf.len();
        let ret = tokio::task::spawn_blocking(move || -> Result<usize> {
            Self::read_at_sync(fd, pos, ptr, len)
        })
        .await
        .unwrap();
        self.anchor(buf);
        ret
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
        Self(file)
    }
}

impl From<std::fs::File> for File {
    fn from(file: std::fs::File) -> Self {
        Self(file.into())
    }
}

impl AsFd for File {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl AsRawFd for File {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl FromRawFd for File {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(tokio::fs::File::from_raw_fd(fd))
    }
}
