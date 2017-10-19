use std::io::{Read, Result, Seek, SeekFrom};

pub enum FileType {
    Archive,
    Elf,
    Unknown,
}


pub fn filetype<T>(mut io: T) -> Result<FileType>
where
    T: Read + Seek,
{
    io.seek(SeekFrom::Start(0))?;
    let mut magic = [0; 8];
    io.read(&mut magic)?;
    io.seek(SeekFrom::Start(0))?;

    if magic[0..4] == [0x7F, 'E' as u8, 'L' as u8, 'F' as u8] {
        return Ok(FileType::Elf);
    }

    if magic
        == [
            '!' as u8,
            '<' as u8,
            'a' as u8,
            'r' as u8,
            'c' as u8,
            'h' as u8,
            '>' as u8,
            0x0A,
        ] {
        return Ok(FileType::Archive);
    }

    return Ok(FileType::Unknown);
}
