use crate::tiff::{ParseTiffError, Parser, Tiff};
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

pub fn read(path: &str) {
    let mut file = File::open(path).unwrap();
    match decode_file(file) {
        Ok(tiff) => println!("{:?}", tiff),
        Err(e) => println!("Error {:?}", e),
    }
}
fn decode_file(mut file: File) -> Result<Tiff, Box<dyn error::Error>> {
    let buf = read_bytes(&mut file, 0, 16)?;
    let (p, ifd_offset) = Parser::header(&buf)?;
    let size = p.size();
    let mut next_ifd = Some(ifd_offset);
    let mut directories: Tiff = Vec::new();
    while let Some(ofs) = next_ifd {
        let mut entries = HashMap::new();
        let mut unloaded = Vec::new();
        let buf = read_bytes(&mut file, ofs, ofs + size.ifd_header)?;
        let count = p.ifd_count(&buf)?;
        let buf = read_bytes(
            &mut file,
            ofs + size.ifd_header,
            ofs + size.ifd_header + size.ifd_body(count),
        )?;
        next_ifd = p.ifd_body(&buf, &mut entries, &mut unloaded)?;
        p.ifd_body(&buf, &mut entries, &mut unloaded)?;
        for (tag, c, dt, addr, len) in unloaded.into_iter() {
            let buf = read_bytes(&mut file, addr, addr + len)?;
            let data = p.entry(c, dt, &buf)?;
            entries.insert(tag, data);
        }
        directories.push(entries);
        println!("next_ifd:: {:?}", &next_ifd);
    }
    println!("parser:: {:?}", &p);
    Ok(directories)
}

fn read_bytes(file: &mut File, start: u64, end: u64) -> Result<Vec<u8>, Box<dyn error::Error>> {
    let len = (end - start) as usize;
    let mut buffer = vec![0; len];
    file.seek(SeekFrom::Start(start))?;
    let l = file.read(&mut buffer)?;
    if l == len {
        Ok(buffer)
    } else {
        Err(Box::new(TmpError::HogeError("hoge".to_string())))
    }
}

#[derive(Debug)]
enum TmpError {
    HogeError(String),
}

impl fmt::Display for TmpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TmpError::HogeError(s) => write!(f, "hogeerror: {}", s),
        }
    }
}

impl error::Error for TmpError {}
