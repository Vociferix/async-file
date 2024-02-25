use winapi::shared::minwindef::DWORD;
use winapi::shared::minwindef::LPCVOID;
use winapi::shared::minwindef::LPVOID;
use winapi::um::fileapi::ReadFileEx;
use winapi::um::fileapi::WriteFileEx;
use winapi::um::minwinbase::OVERLAPPED;
use winapi::um::winnt::HANDLE;

use std::io::Result;
use std::os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, RawHandle};
use std::path::Path;
use std::sync::Mutex;

#[derive(Debug)]
pub struct File(tokio::fs::File);

enum State<T> {
    Init,
    Pending(std::task::Waker),
    Ready(Result<T>),
}

struct OverlappedFut<T> {
    overlapped: OVERLAPPED,
    state: Mutex<State<T>>,
}

impl<T> OverlappedFut<T> {
    fn new() -> Self {
        Self {
            overlapped: unsafe { std::mem::zeroed() },
            state: Mutex::new(State::Init),
        }
    }

    fn from_overlapped(ptr: *mut OVERLAPPED) -> *mut OverlappedFut<T> {
        unsafe {
            (ptr as *mut () as *mut u8).sub(memoffset::offset_of!(Self, overlapped))
                as *mut OverlappedFut<T>
        }
    }
}

impl<'a, T> std::future::Future for OverlappedFut<T> {
    type Output = Result<T>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut state_guard = self.state.lock().unwrap();
        let state = std::mem::replace(&mut *state_guard, State::Init);
        match state {
            State::Init => {
                *state_guard = State::Pending(cx.waker().clone());
                std::task::Poll::Pending
            }
            State::Pending(waker) => {
                *state_guard = State::Pending(waker);
                std::task::Poll::Pending
            }
            State::Ready(res) => std::task::Poll::Ready(res),
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

    async fn write_at_impl<'a>(&'a self, pos: u64, buf: &[u8]) -> Result<usize> {
        let mut fut = OverlappedFut::<usize>::new();
        let pos_low = (pos & 0xffffffff) as DWORD;
        let pos_hi = (pos >> 32) as DWORD;
        unsafe {
            let s = fut.overlapped.u.s_mut();
            s.Offset = pos_low;
            s.OffsetHigh = pos_hi;
        }
        unsafe {
            extern "system" fn callback(err: DWORD, bytes: DWORD, overlapped: *mut OVERLAPPED) {
                unsafe {
                    let fut: &'static mut OverlappedFut<usize> =
                        &mut *OverlappedFut::from_overlapped(overlapped);
                    let mut state_guard = fut.state.lock().unwrap();
                    let state = std::mem::replace(&mut *state_guard, State::Init);
                    if err != 0 {
                        *state_guard =
                            State::Ready(Err(std::io::Error::from_raw_os_error(err as i32)));
                    } else {
                        *state_guard = State::Ready(Ok(bytes as usize));
                    }
                    match state {
                        State::Pending(waker) => {
                            waker.wake();
                        }
                        _ => (),
                    }
                }
            }
            if WriteFileEx(
                self.0.as_raw_handle() as HANDLE,
                buf.as_ptr() as LPCVOID,
                buf.len() as DWORD,
                &mut fut.overlapped as *mut OVERLAPPED,
                Some(callback),
            ) == 0
            {
                return Err(std::io::Error::last_os_error());
            }
        }
        fut.await
    }

    pub async fn write_at(&self, mut pos: u64, mut buf: &[u8]) -> Result<usize> {
        let mut total: usize = 0;
        while buf.len() > DWORD::MAX as usize {
            let written = self.write_at_impl(pos, &buf[..DWORD::MAX as usize]).await?;
            pos += written as u64;
            buf = &buf[written..];
            total += written;
            if written == 0 {
                return Ok(total);
            }
        }
        while !buf.is_empty() {
            let written = self.write_at_impl(pos, buf).await?;
            pos += written as u64;
            buf = &buf[written..];
            total += written;
            if written == 0 {
                return Ok(total);
            }
        }
        Ok(total)
    }

    async fn read_at_impl<'a>(&'a self, pos: u64, buf: &mut [u8]) -> Result<usize> {
        let mut fut = OverlappedFut::<usize>::new();
        let pos_low = (pos & 0xffffffff) as DWORD;
        let pos_hi = (pos >> 32) as DWORD;
        unsafe {
            let s = fut.overlapped.u.s_mut();
            s.Offset = pos_low;
            s.OffsetHigh = pos_hi;
        }
        unsafe {
            extern "system" fn callback(err: DWORD, bytes: DWORD, overlapped: *mut OVERLAPPED) {
                unsafe {
                    let fut: &'static mut OverlappedFut<usize> =
                        &mut *OverlappedFut::from_overlapped(overlapped);
                    let mut state_guard = fut.state.lock().unwrap();
                    let state = std::mem::replace(&mut *state_guard, State::Init);
                    if err != 0 {
                        *state_guard =
                            State::Ready(Err(std::io::Error::from_raw_os_error(err as i32)));
                    } else {
                        *state_guard = State::Ready(Ok(bytes as usize));
                    }
                    match state {
                        State::Pending(waker) => {
                            waker.wake();
                        }
                        _ => (),
                    }
                }
            }
            if ReadFileEx(
                self.0.as_raw_handle() as HANDLE,
                buf.as_mut_ptr() as LPVOID,
                buf.len() as DWORD,
                &mut fut.overlapped as *mut OVERLAPPED,
                Some(callback),
            ) == 0
            {
                return Err(std::io::Error::last_os_error());
            }
        }
        fut.await
    }

    pub async fn read_at(&self, mut pos: u64, mut buf: &mut [u8]) -> Result<usize> {
        let mut total: usize = 0;
        while buf.len() > DWORD::MAX as usize {
            let read = self
                .read_at_impl(pos, &mut buf[..DWORD::MAX as usize])
                .await?;
            pos += read as u64;
            buf = &mut buf[read..];
            total += read;
            if read == 0 {
                return Ok(total);
            }
        }
        while !buf.is_empty() {
            let read = self.read_at_impl(pos, buf).await?;
            pos += read as u64;
            buf = &mut buf[read..];
            total += read;
            if read == 0 {
                return Ok(total);
            }
        }
        Ok(total)
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

impl AsHandle for File {
    fn as_handle(&self) -> BorrowedHandle<'_> {
        self.0.as_handle()
    }
}

impl AsRawHandle for File {
    fn as_raw_handle(&self) -> RawHandle {
        self.0.as_raw_handle()
    }
}

impl FromRawHandle for File {
    unsafe fn from_raw_handle(handle: RawHandle) -> Self {
        Self(tokio::fs::File::from_raw_handle(handle))
    }
}
