use crate::File;
use std::io::Result;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct OpenOptions(tokio::fs::OpenOptions);

impl OpenOptions {
    pub fn new() -> Self {
        Self(tokio::fs::OpenOptions::new())
    }

    pub fn read(&mut self, read: bool) -> &mut Self {
        self.0.read(read);
        self
    }

    pub fn write(&mut self, write: bool) -> &mut Self {
        self.0.write(write);
        self
    }

    pub fn append(&mut self, append: bool) -> &mut Self {
        self.0.append(append);
        self
    }

    pub fn truncate(&mut self, truncate: bool) -> &mut Self {
        self.0.truncate(truncate);
        self
    }

    pub fn create(&mut self, create: bool) -> &mut Self {
        self.0.create(create);
        self
    }

    pub fn create_new(&mut self, create_new: bool) -> &mut Self {
        self.0.create_new(create_new);
        self
    }

    pub async fn open(&self, path: impl AsRef<Path>) -> Result<File> {
        File::open_with_options(&self.0, path).await
    }

    #[cfg(unix)]
    pub fn mode(&mut self, mode: u32) -> &mut Self {
        self.0.mode(mode);
        self
    }

    #[cfg(unix)]
    pub fn custom_flags(&mut self, flags: i32) -> &mut Self {
        self.0.custom_flags(flags);
        self
    }

    #[cfg(windows)]
    pub fn access_mode(&mut self, access: u32) -> &mut Self {
        self.0.access_mode(access);
        self
    }

    #[cfg(windows)]
    pub fn share_mode(&mut self, share: u32) -> &mut Self {
        self.0.share_mode(share);
        self
    }

    #[cfg(windows)]
    pub fn custom_flags(&mut self, flags: u32) -> &mut Self {
        self.0.custom_flags(flags);
        self
    }

    #[cfg(windows)]
    pub fn attributes(&mut self, attributes: u32) -> &mut Self {
        self.0.attributes(attributes);
        self
    }

    #[cfg(windows)]
    pub fn security_qos_flags(&mut self, flags: u32) -> &mut Self {
        self.0.security_qos_flags(flags);
        self
    }
}

impl From<tokio::fs::OpenOptions> for OpenOptions {
    fn from(opts: tokio::fs::OpenOptions) -> Self {
        Self(opts)
    }
}

impl From<std::fs::OpenOptions> for OpenOptions {
    fn from(opts: std::fs::OpenOptions) -> Self {
        Self(opts.into())
    }
}
