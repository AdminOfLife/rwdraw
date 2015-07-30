// TODO lock cd image so it cannot be edited by some program else while open here

use std::path::Path;
use std::fs::File;
use std::io::{self, Seek, SeekFrom, Read, BufReader};
use std::collections::HashMap;
use byteorder::{self, ReadBytesExt, LittleEndian};
use std::iter;

pub enum Error {
    InvalidImage,
    NonExistingEntry,
    IoError(byteorder::Error),
}

pub type Result<T> = ::std::result::Result<T, Error>;

impl From<byteorder::Error> for Error {
    fn from(e: byteorder::Error) -> Error {
        Error::IoError(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::from(byteorder::Error::from(e))
    }
}

#[derive(Debug)]
pub struct CdEntry {
    offset: u32,         // in sectors
    streaming_size: u16, // in sectors
    stored_size: u16,    // in sectors
    // TODO next?
}

#[derive(Debug)]
pub struct CdImage {
    file: File,
    dir: HashMap<String, CdEntry>,
}

impl CdImage {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<CdImage> {
        let (file, entries) = {
            let mut reader = BufReader::new(try!(File::open(path)));

            let entries = match try!(reader.read_u32::<LittleEndian>()) {
                0x32524556 => { // VER2
                    let num_entries = try!(reader.read_u32::<LittleEndian>()) as usize;
                    try!(CdImage::read_directory_ver2(&mut reader, num_entries))
                },
                _ => return Err(Error::InvalidImage),
            };

            (reader.into_inner(), entries)
        };

        Ok(CdImage {
            file: file,
            dir: entries,
        })
    }

    pub fn read(&mut self, name: &str) -> Result<Vec<u8>> {
        // TODO faster lowercase lookup pls
        // TODO streaming buffer instead of allocating a Vec everytime?
        let name = name.to_lowercase();

        let data = match self.dir.get(&name) {
            Some(entry) => {
                try!(self.file.seek(SeekFrom::Start(2048 * entry.offset as u64)));
                try!(CdImage::read_bytes(&mut self.file, 2048 * entry.stored_size as usize))
            },
            None => return Err(Error::NonExistingEntry),
        };

        Ok(data)
    }

    fn read_directory_ver2<R: Read>(f: &mut R, num_entries: usize)
                                                  -> Result<HashMap<String, CdEntry>> {
        let mut entries = HashMap::with_capacity(num_entries);
        let mut chars25: [u8; 25] = [0; 25]; // last byte used for safe null terminator

        for _ in 0..num_entries {
            let mut entry = CdEntry {
                offset:         try!(f.read_u32::<LittleEndian>()),
                streaming_size: try!(f.read_u16::<LittleEndian>()),
                stored_size:    try!(f.read_u16::<LittleEndian>()),
            };

            if entry.stored_size == 0 {
                entry.stored_size = entry.streaming_size;
            }

            let name = unsafe {
                use std::ffi::CStr;
                use std::mem;
                try!(CdImage::read_full(f, &mut chars25[..24]));
                let ptr = mem::transmute::<_, &[i8]>(&chars25[..]).as_ptr();
                String::from_utf8_lossy(CStr::from_ptr(ptr).to_bytes()).into_owned()
            };

            entries.insert(name.to_lowercase(), entry);
        }

        Ok(entries)
    }




    // TODO common function read_bytes
    fn read_bytes<R: Read>(f: &mut R, size: usize) -> byteorder::Result<Vec<u8>> {
        unsafe {
            let mut v = Vec::with_capacity(size);
            v.set_len(size);
            Ok(try!(CdImage::read_full(f, &mut v[..]).map(|_| v)))
        }
    }

    // TODO common function read_full
    fn read_full<R: Read>(f: &mut R, buf: &mut [u8]) -> byteorder::Result<()> {
        use byteorder::*;
        use std::io;
        let mut nread = 0usize;
        while nread < buf.len() {
            match f.read(&mut buf[nread..]) {
                Ok(0) => return Err(Error::UnexpectedEOF),
                Ok(n) => nread += n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {},
                Err(e) => return Err(From::from(e))
            }
        }
        Ok(())
    }
}

