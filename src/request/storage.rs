use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt, SeekFrom};
use std::collections::HashMap;
use log::{debug};


pub struct TorrentPersisted {
    file: File,
}

impl TorrentPersisted {
    pub async fn new(file_name: &str, total_size: u64) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(file_name)
            .await?; // <-- .await qui

        file.set_len(total_size).await?;

        Ok(Self { file })
    }

    pub async fn write_pieces(&mut self, data: &mut HashMap<usize, Vec<u8>>, piece_length: usize) -> std::io::Result<()> {
        for (i, piece) in data.drain() {
            let offset = (i as u64) * (piece_length as u64);

            self.file.seek(SeekFrom::Start(offset)).await?;
            self.file.write_all(&piece).await?;
        }

        self.file.sync_data().await?;
        debug!("Flushed downloaded pieces to storage");
        Ok(())
    }
}