use std::io::Result;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, RawFd};
use std::path::Path;
use std::sync::OnceLock;

static URING: OnceLock<Result<rio::Rio>> = OnceLock::new();

struct UringPtr(*const rio::Rio);

unsafe impl Send for UringPtr {}

unsafe impl Sync for UringPtr {}

static mut UNSAFE_URING: UringPtr = UringPtr(std::ptr::null());

pub(crate) fn init_uring() -> Result<()> {
    match URING.get_or_init(rio::new) {
        Ok(ring) => {
            unsafe {
                UNSAFE_URING.0 = ring as *const rio::Rio;
            }
            Ok(())
        }
        Err(e) => Err(std::io::Error::from(e.kind())),
    }
}

unsafe fn uring() -> &'static rio::Rio {
    &*UNSAFE_URING.0
}

#[derive(Debug)]
pub struct File(tokio::fs::File);

impl File {
    pub(crate) async fn open_with_options(
        options: &tokio::fs::OpenOptions,
        path: impl AsRef<Path>,
    ) -> Result<Self> {
        init_uring()?;
        Ok(Self(options.open(path).await?))
    }

    pub async fn create(path: impl AsRef<Path>) -> Result<Self> {
        init_uring()?;
        Ok(Self(tokio::fs::File::create(path).await?))
    }

    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        init_uring()?;
        Ok(Self(tokio::fs::File::open(path).await?))
    }

    pub async fn metadata(&self) -> Result<std::fs::Metadata> {
        self.0.metadata().await
    }

    pub async fn write_at(&self, pos: u64, buf: &[u8]) -> Result<usize> {
        unsafe { uring().write_at(&self.0, &buf, pos).await }
    }

    pub async fn read_at(&self, pos: u64, buf: &mut [u8]) -> Result<usize> {
        unsafe { uring().read_at(&self.0, &buf, pos).await }
    }

    pub async fn sync_all(&self) -> Result<()> {
        unsafe {
            let f = std::fs::File::from_raw_fd(self.0.as_raw_fd());
            uring().fsync(&f).await?;
            std::mem::forget(f)
        }
        Ok(())
    }

    pub async fn sync_data(&self) -> Result<()> {
        unsafe {
            let f = std::fs::File::from_raw_fd(self.0.as_raw_fd());
            uring().fdatasync(&f).await?;
            std::mem::forget(f)
        }
        Ok(())
    }

    pub async fn set_len(&self, size: u64) -> Result<()> {
        self.0.set_len(size).await
    }

    pub(crate) unsafe fn unsafe_from_file(file: tokio::fs::File) -> Self {
        Self(file)
    }

    pub(crate) unsafe fn unsafe_from_raw_fd(fd: RawFd) -> Self {
        Self(tokio::fs::File::from_raw_fd(fd))
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
