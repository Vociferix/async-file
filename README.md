# async-file

`async-file` is a Rust crate that implements an asynchronous file type for
the Tokio ecosystem. Unlike `tokio::fs::File`, this implementation attempts
to use platform specific APIs (`io_uring`, `aio`, `OVERLAPPED`) to provide
actual async file I/O without userland I/O offload threads. However, not all
platforms support async file I/O and no platform provides async APIs for
all file I/O operations, such as setting the length of a file. So this crate
falls back to the Tokio implementation for operations when needed.

`async_file::File` additionally does not implement `AsyncRead`, `AsyncWrite`,
or `AsyncSeek`. Async file reading and writing on all supported platforms is
"positioned", meaning that instead of treating the file as a stream, each
read and write requires the user to provide the offset at which the read or
write should start. So `async_file::File` has methods `read_at` and `write_at`,
instead.
