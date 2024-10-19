use crate::database::objects::Hash;
use crate::to_hex;

use async_fs::{rename, remove_file};
use async_fs::{File, OpenOptions};
use futures_lite::AsyncWriteExt;
use sha2::{Sha256, Digest};
use std::io::ErrorKind;
use rand::random;

pub struct TemporaryFile {
    handle: File,
    tmp_path: String,
    hasher: Sha256,
    size: usize,
}

impl TemporaryFile {
    pub async fn new() -> Result<Self, &'static str> {
        let hasher = Sha256::new();

        let (handle, tmp_path) = loop {
            let rubbish: u32 = random();
            let tmp_path = format!("files/tmp-{:08x?}.dat", rubbish);

            let mut options = OpenOptions::new();
            options.write(true);
            options.create_new(true);

            match options.open(&tmp_path).await {
                Ok(file) => break Ok((file, tmp_path)),
                Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
                _other => break Err("Couldn't create temporary file"),
            }
        }?;

        Ok(Self {
            handle,
            tmp_path,
            hasher,
            size: 0,
        })
    }

    pub async fn extend_from_slice(&mut self, bytes: &[u8]) {
        let _ = self.handle.write_all(bytes).await;
        self.hasher.update(bytes);
        self.size += bytes.len();
    }

    pub async fn finalize(mut self) -> (Hash, usize) {
        let _ = self.handle.close();

        let hash = to_hex(self.hasher.finalize().into());

        let final_path = format!("files/{}.dat", hash);

        if let Err(error) = rename(&self.tmp_path, final_path).await {
            if error.kind() == ErrorKind::AlreadyExists {
                let _ = remove_file(&self.tmp_path).await;
            } else {
                println!("Cannot rename {}: {:?}", self.tmp_path, error);
            }
        }

        (hash, self.size)
    }
}
