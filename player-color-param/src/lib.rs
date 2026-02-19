use player_color_param::{PlayerColorParam, EntryKey, RGB};
use byteorder::{LittleEndian, ReadBytesExt};
// use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::mem::size_of;
use indexmap::IndexMap;
use std::collections::HashMap;

pub fn from_binary_data<R: Read + Seek>(reader: &mut R) -> std::io::Result<PlayerColorParam> {
    const EXPECTED_VERSION: u32 = 1000;
    let version = reader.read_u32::<LittleEndian>()?;
    if version != EXPECTED_VERSION {
        return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unsupported version. Expected version {} but got {}.", EXPECTED_VERSION, version)
        ));
    }

    let entry_count = reader.read_u32::<LittleEndian>()?;
    // May be used for inserting additional data ignored by the parser.
    let data_offset = reader.read_u64::<LittleEndian>()? - size_of::<u64>() as u64;

    reader.seek(SeekFrom::Current(data_offset as i64))?;

    let mut entries = IndexMap::<EntryKey, RGB>::new();
    let mut alt_tracker = HashMap::<(String, u8), u8>::new();
    for _ in 0..entry_count {
        let character_id_offset = reader.read_u64::<LittleEndian>()? - size_of::<u64>() as u64;
        let pos_save = reader.stream_position()?;
        reader.seek(SeekFrom::Current(character_id_offset as i64))?;
        let character_id = read_cstring(reader)?;
        reader.seek(SeekFrom::Start(pos_save))?;

        let costume_index = reader.read_u32::<LittleEndian>()? as u8;

        let alt_tracker_key = (character_id.clone(), costume_index);
        let alt_index = {
            let count = alt_tracker.entry(alt_tracker_key).or_insert(0);
            let current = *count;
            *count += 1;
            current
        };

        let red = reader.read_u32::<LittleEndian>()? as u8;
        let green = reader.read_u32::<LittleEndian>()? as u8;
        let blue = reader.read_u32::<LittleEndian>()? as u8;

        entries.insert(
            EntryKey {
                character_id,
                costume_index,
                alt_index,
            },
            RGB {
                red,
                green,
                blue,
            }
        );
    }

    Ok(PlayerColorParam {
        entries,
    })
}

fn read_cstring<R: Read>(reader: &mut R) -> std::io::Result<String> {
    let mut bytes = Vec::new();
    loop {
        let byte = reader.read_u8()?;
        if byte == 0 {
            break;
        }
        bytes.push(byte);
    }
    String::from_utf8(bytes)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn empty_binary() {
        let data = vec![
            0xE8, 0x03, 0x00, 0x00, // version: u32 = 1000
            0x00, 0x00, 0x00, 0x00, // entry_count: u32 = 0
            0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 // data_offset: u64 = 8
        ];

        let result = from_binary_data(&mut Cursor::new(data)).unwrap();
        assert_eq!(result.entries.len(), 0);
    }

    #[test]
    fn invalid_version() {
        let data: Vec<u8> = vec![
            0xE9, 0x03, 0x00, 0x00, // version: u32 = 1001
            0x00, 0x00, 0x00, 0x00, // entry_count: u32 = 0
            0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 // data_offset: u64 = 8
        ];

        let result = from_binary_data(&mut Cursor::new(data));
        assert!(!result.is_ok());
    }

    #[test]
    fn example_binary() {
        let data = vec![
            0xE8, 0x03, 0x00, 0x00, // version: u32 = 1000
            0x02, 0x00, 0x00, 0x00, // entry_count: u32 = 2
            0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // data_offset: u64 = 16
            0x68, 0x65, 0x6C, 0x6C, 0x6F, 0x00, 0x00, 0x00, // "hello"
            0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // character_id_ptr: u64 = 48
            0x03, 0x00, 0x00, 0x00, // costume_index: u32 = 3
            0x40, 0x00, 0x00, 0x00, // red: u32 = 64
            0x52, 0x00, 0x00, 0x00, // blue: u32 = 82
            0xC5, 0x00, 0x00, 0x00, // green: u32 = 197
            0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // character_id_ptr: u64 = 32
            0x03, 0x00, 0x00, 0x00, // costume_index: u32 = 3
            0x8F, 0x00, 0x00, 0x00, // red: u32 = 143
            0xF6, 0x00, 0x00, 0x00, // blue: u32 = 246
            0x48, 0x00, 0x00, 0x00, // green: u32 = 72
            0x31, 0x6A, 0x6E, 0x74, 0x30, 0x31, 0x00, 0x00, // character_id = "1jnt01"
            0x31, 0x6A, 0x6E, 0x74, 0x30, 0x31, 0x00, 0x00, // character_id = "1jnt01"
        ];

        let result = from_binary_data(&mut Cursor::new(data)).unwrap();

        println!("{:#?}", result);

        let key = EntryKey {
            character_id: "1jnt01".to_string(),
            costume_index: 3,
            alt_index: 0,
        };
        assert!(result.entries.contains_key(&key));
        assert_eq!(result.entries[&key], RGB { red: 64, green: 82, blue: 197 });

        let key = EntryKey {
            character_id: "1jnt01".to_string(),
            costume_index: 3,
            alt_index: 1,
        };
        assert!(result.entries.contains_key(&key));
        assert_eq!(result.entries[&key], RGB { red: 143, green: 246, blue: 72 });
    }
}
