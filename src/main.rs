use std::io;
use std::fs::File;

mod error;

use error::*;

fn main() -> ReadResult<()> {
    let mut gob_file = File::open(r"C:\Games\Steam\steamapps\common\Dark Forces\Game\DARK.GOB")?;

    for entry in gob::Catalog::read(&mut gob_file)? {
        let offset = entry.offset();
        let length = entry.length();
        let name = entry.name();
        println!("{:8x}:{:8x}: {:?}", offset, length, name);
        if name.as_deref() == Ok("TEXT.MSG") {
            use io::Read as _;
            let mut data = String::new();
            let len = entry.data(&mut gob_file).read_to_string(&mut data)?;
            println!("- ({} bytes) {}", len, data);
        }
    }
    Ok(())
}

mod gob;

fn read_u32(input: impl io::Read) -> io::Result<u32> {
    Ok(u32::from_le_bytes(read_buf(input, [0u8; 4])?))
}

fn read_buf<T: AsMut<[u8]>>(mut input: impl io::Read, mut buffer: T) -> io::Result<T> {
    input.read_exact(buffer.as_mut())?;
    Ok(buffer)
}