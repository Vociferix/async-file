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
pub use linux::File;

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
pub use unix::File;

#[cfg(any(
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub use bsd::File;

#[cfg(target_os = "windows")]
pub use windows::File;

pub use options::OpenOptions;
