use std::io::Result;
use std::os::unix::io::{AsRawFd, RawFd};
use std::task::Poll;
use tokio::io::bsd::{Aio, AioSource};

#[derive(Debug)]
pub struct File(tokio::fs::File);

struct Source<T>(T);

struct AioFut<T: mio_aio::SourceApi>(Aio<Source<T>>);

impl<T: mio_aio::SourceApi> AioSource for Source<T> {
    fn register(&mut self, kq: RawFd, token: usize) {
        self.0.register_raw(kq, token);
    }

    fn deregister(&mut self) {
        self.0.deregister_raw();
    }
}

impl<T: mio_aio::SourceApi> std::futures::Future for AioFut<T> {
    type Output = Result<T::Output>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let poll_result = self.0.poll_ready(cx);
        match poll_result {
            Poll::Pending => {
                if !self.0 .0.in_progress() {
                    let p = unsafe { self.map_unchecked_mut(|s| &mut s.0 .0) };
                    match p.submit() {
                        Ok(()) => (),
                        Err(e) => return Poll::Ready(Err(io::Error::from_raw_os_error(e as i32))),
                    }
                }
                Poll::Pending
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Ready(Ok(_ev)) => {
                let p = unsafe { self.map_unchecked_mut(|s| &mut s.0 .0) };
                let result = p.aio_return();
                match result {
                    Ok(r) => Poll::Ready(Ok(r)),
                    Err(e) => Poll::Ready(Err(std::io::Error::from_raw_os_error(e as i32))),
                }
            }
        }
    }
}

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

    pub async fn write_at(&self, pos: u64, buf: &[u8]) -> Result<usize> {
        AioFut(Aio::new_for_aio(Source(mio_aio::WriteAt::write_at(
            self.0.as_raw_fd(),
            pos,
            buf,
            0,
        )))?)
        .await
    }

    pub async fn read_at(&self, pos: u64, buf: &mut [u8]) -> Result<usize> {
        AioFut(Aio::new_for_aio(Source(mio_aio::ReadAt::read_at(
            self.0.as_raw_fd(),
            pos,
            buf,
            0,
        )))?)
        .await
    }

    pub async fn sync_all(&self) -> Result<()> {
        AioFut(Aio::new_for_aio(Source(mio_aio::Fsync::fsync(
            self.0.as_raw_fd(),
            mio_aio::AioFsyncMode::O_SYNC,
            0,
        )))?)
        .await
    }

    pub async fn sync_data(&self) -> Result<()> {
        self.sync_all().await
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
        self.as_raw_fd()
    }
}

impl FromRawFd for File {
    unsafe fn from_raw_fd(fd: RawFd) -> Self {
        Self(tokio::fs::File::from_raw_fd(fd))
    }
}
