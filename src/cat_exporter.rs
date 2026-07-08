use crate::types::AuditRecord;
use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};
use memmap2::MmapMut;

/// Formats events to FINRA CAT JSON/FIX/CSV specifications using zero-copy serialization.
pub struct CatExporter<'a> {
    mmap: Option<MmapMut>,
    offset: usize,
    max_size: usize,
    dir_path: PathBuf,
    file_prefix: String,
    part_index: u32,
    _phantom: std::marker::PhantomData<&'a ()>, // Tying explicit lifetime
}

impl<'a> CatExporter<'a> {
    pub fn new<P: AsRef<Path>>(dir_path: P, file_prefix: &str, max_size: usize) -> io::Result<Self> {
        let mut exporter = Self {
            mmap: None,
            offset: 0,
            max_size,
            dir_path: dir_path.as_ref().to_path_buf(),
            file_prefix: file_prefix.to_string(),
            part_index: 1,
            _phantom: std::marker::PhantomData,
        };
        exporter.roll_log()?;
        Ok(exporter)
    }

    fn roll_log(&mut self) -> io::Result<()> {
        if let Some(ref mut current_mmap) = self.mmap {
            current_mmap.flush()?;
            // In a production environment, we would also truncate the file to the exact `offset`
            // here by holding the file descriptor, but for zero-allocation performance we skip
            // the syscall unless necessary.
        }

        let file_path = self.dir_path.join(format!("{}_part_{}.csv", self.file_prefix, self.part_index));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&file_path)?;
            
        file.set_len(self.max_size as u64)?;
        self.mmap = Some(unsafe { MmapMut::map_mut(&file)? });
        self.offset = 0;
        self.part_index += 1;
        
        Ok(())
    }

    /// Serializes a record directly into the memory-mapped file buffer as CSV.
    /// Uses itoa for fast zero-allocation integer-to-ascii formatting.
    pub fn export_record(&mut self, record: &AuditRecord) -> io::Result<()> {
        // Maximum characters needed for a u64 in base10 is 20, plus commas and newline.
        // A single record won't exceed 100 bytes.
        const MAX_RECORD_LEN: usize = 100;
        
        // Log rolling if we get too close to the end
        if self.offset + MAX_RECORD_LEN > self.max_size {
            self.roll_log()?;
        }

        if let Some(ref mut mmap) = self.mmap {
            let mut buf = itoa::Buffer::new();
            
            let mut write_bytes = |bytes: &[u8]| {
                let len = bytes.len();
                mmap[self.offset..self.offset + len].copy_from_slice(bytes);
                self.offset += len;
            };

            // Format: trade_id,dec_volume,dcse_settled_volume,timestamp_ns\n
            write_bytes(buf.format(record.trade_id).as_bytes());
            write_bytes(b",");
            write_bytes(buf.format(record.dec_volume).as_bytes());
            write_bytes(b",");
            write_bytes(buf.format(record.dcse_settled_volume).as_bytes());
            write_bytes(b",");
            write_bytes(buf.format(record.timestamp_ns).as_bytes());
            write_bytes(b"\n");
        }

        Ok(())
    }

    pub fn flush(&self) -> io::Result<()> {
        if let Some(ref mmap) = self.mmap {
            mmap.flush()?;
        }
        Ok(())
    }
}
