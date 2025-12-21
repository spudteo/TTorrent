use log::debug;
use std::collections::{HashMap, HashSet};
use tokio::fs::{File, OpenOptions, read_to_string};
use tokio::io::{AsyncSeekExt, AsyncWriteExt, SeekFrom};

pub struct TorrentPersisted {
    file: File,
    file_name: String,
    checkpoint_file: File,
}

impl TorrentPersisted {
    pub async fn new(file_name: &str, total_size: u64) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(file_name)
            .await?;

        file.set_len(total_size).await?;

        let checkpoint_file = OpenOptions::new()
            .read(true)
            .append(true)
            .write(true)
            .create(true)
            .open(format!("{}.{}", file_name, "checkpoint"))
            .await?;

        //fixme unwrap ?
        Ok(Self {
            file,
            file_name: file_name.parse().unwrap(),
            checkpoint_file,
        })
    }

    pub async fn read_checkpoint(&self) -> std::io::Result<HashSet<usize>> {
        let path = format!("{}.checkpoint", self.file_name);

        if !std::path::Path::new(&path).exists() {
            return Ok(HashSet::new());
        }

        let content = read_to_string(path).await?;

        let completed_pieces: HashSet<usize> = content
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<usize>().ok())
            .collect();

        Ok(completed_pieces)
    }

    pub async fn write_pieces(
        &mut self,
        data: &mut HashMap<usize, Vec<u8>>,
        piece_length: usize,
    ) -> std::io::Result<()> {
        let mut piece_id: Vec<u32> = vec![];
        for (i, piece) in data.drain() {
            let offset = (i as u64) * (piece_length as u64);
            self.file.seek(SeekFrom::Start(offset)).await?;
            self.file.write_all(&piece).await?;
            piece_id.push(i as u32);
        }

        self.file.sync_data().await?;
        let data: String = piece_id.iter().map(|id| format!("{},", id)).collect();
        self.checkpoint_file.write_all(data.as_bytes()).await?;
        debug!("Flushed downloaded pieces to storage");
        Ok(())
    }
}
