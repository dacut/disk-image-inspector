use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use codepage_437::{FromCp437, CP437_WINGDINGS};
use log::{debug, warn};
use phf::{phf_map, Map};
use std::{
    convert::TryInto,
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
    io::{Read, Seek, SeekFrom},
};

use crate::errors::ImageError;

pub const BOOT_SECTOR_SIZE: usize = 512;

pub const FAT_MEDIA_TYPES: Map<u8, &'static str> = phf_map! {
    0xf0u8 => "3.5\" Floppy (1.44 MB) / 3.5\" Floppy (2.88 MB)",
    0xf8u8 => "Hard disk",
    0xf9u8 => "3.5\" Floppy (720 kB) / 5.25\" Floppy (1.2 MB)",
    0xfau8 => "3.5\" Floppy (320 kB) / 5.25\" Floppy (320 kB)",
    0xfbu8 => "3.5\" Floppy (640 kB) / 5.25\" Floppy (640 kB)",
    0xfcu8 => "5.25\" Floppy (180 kB)",
    0xfdu8 => "5.25\" Floppy (360 kB)",
    0xfeu8 => "5.25\" Floppy (160 kB)",
    0xffu8 => "5.25\" Floppy (320 kB)",
};

pub const FAT_ATTRIBUTE_READ_ONLY: u8 = 0x01u8;
pub const FAT_ATTRIBUTE_HIDDEN: u8 = 0x02u8;
pub const FAT_ATTRIBUTE_SYSTEM: u8 = 0x04u8;
pub const FAT_ATTRIBUTE_VOLUME_LABEL: u8 = 0x08u8;
pub const FAT_ATTRIBUTE_DIRECTORY: u8 = 0x10u8;
pub const FAT_ATTRIBUTE_ARCHIVE: u8 = 0x20u8;
pub const FAT_ATTRIBUTE_DEVICE: u8 = 0x40u8;
pub const FAT_ATTRIBUTE_LONG_FILENAME: u8 = 0x0fu8;

const FAT_DIRECTORY_ENTRY_SIZE: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FatType {
    Fat12,
    Fat16,
    Fat32,
}

impl Display for FatType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Fat12 => f.write_str("FAT12"),
            Self::Fat16 => f.write_str("FAT16"),
            Self::Fat32 => f.write_str("FAT32"),
        }
    }
}

#[derive(Debug)]
pub struct FatPartition<R: Read + Seek> {
    pub reader: R,
    pub offset: u64,
    pub fat_type: FatType,
    pub boot_sector: FatBootSector,
    pub fat_tables: Vec<Vec<u32>>,
}

impl<R: Read + Seek> FatPartition<R> {
    pub fn from_partition_image(mut reader: R, offset: u64) -> Result<Self, Box<dyn Error + 'static>> {
        let boot_sector = FatBootSector::from_partition_image(&mut reader, offset)?;
        let fat_type = match boot_sector.extra {
            FatBootSectorExtra::Fat12(_) => FatType::Fat12,
            FatBootSectorExtra::Fat16(_) => FatType::Fat16,
            FatBootSectorExtra::Fat32(_) => FatType::Fat32,
        };

        reader.seek(SeekFrom::Start(
            offset + boot_sector.reserved_sectors as u64 * boot_sector.bytes_per_sector as u64,
        ))?;

        let mut fat_tables = Vec::with_capacity(boot_sector.number_of_fats as usize);
        let fat_table_size = boot_sector.sectors_per_fat as usize * boot_sector.bytes_per_sector as usize;
        for i in 0..boot_sector.number_of_fats as usize {
            reader.seek(SeekFrom::Start(offset + boot_sector.get_fat_table_offset(i)))?;
            let mut fat_table_bytes = vec![0; fat_table_size];

            reader.read_exact(&mut fat_table_bytes)?;

            match fat_type {
                FatType::Fat12 => {
                    // FAT12 has to be handled differently. Each entry spans 1.5 bytes, so we read 3 bytes at a time
                    // and write 2 FAT entries.
                    let mut fat_table = Vec::with_capacity(fat_table_size / 3);
                    for i in (0..fat_table_size).step_by(3) {
                        let first = fat_table_bytes[i] as u32 | (fat_table_bytes[i + 1] as u32 & 0x0f) << 8;
                        let second =
                            ((fat_table_bytes[i + 1] as u32 & 0xf0) >> 4) | ((fat_table_bytes[i + 2] as u32) << 4);

                        fat_table.push(first);
                        fat_table.push(second);
                    }

                    fat_tables.push(fat_table);
                }
                FatType::Fat16 => {
                    // 16-bit LE entries.
                    let mut fat_table = Vec::with_capacity(fat_table_size / 2);
                    for i in (0..fat_table_size).step_by(2) {
                        fat_table.push(u16::from_le_bytes(fat_table_bytes[i..i + 2].try_into().unwrap()) as u32);
                    }

                    fat_tables.push(fat_table);
                }
                FatType::Fat32 => {
                    // 32-bit LE entries.
                    let mut fat_table = Vec::with_capacity(fat_table_size / 2);
                    for i in (0..fat_table_size).step_by(4) {
                        fat_table.push(u32::from_le_bytes(fat_table_bytes[i..i + 2].try_into().unwrap()));
                    }

                    fat_tables.push(fat_table);
                }
            }
        }

        Ok(Self {
            reader,
            offset,
            fat_type,
            fat_tables,
            boot_sector,
        })
    }

    pub fn get_root_directory_entries(&mut self) -> Result<Vec<FatDirectoryEntry>, Box<dyn Error + 'static>> {
        let mut directory_entries = Vec::with_capacity(self.boot_sector.root_directory_entries as usize);
        self.reader.seek(SeekFrom::Start(self.offset + self.boot_sector.get_root_directory_offset()))?;

        for _ in 0..self.boot_sector.root_directory_entries {
            let mut directory_entry_bytes: [u8; FAT_DIRECTORY_ENTRY_SIZE] = [0; FAT_DIRECTORY_ENTRY_SIZE];
            self.reader.read_exact(&mut directory_entry_bytes)?;
            let directory_entry = FatDirectoryEntry::from_data(&directory_entry_bytes, self.fat_type);
            directory_entries.push(directory_entry);
        }

        Ok(directory_entries)
    }

    pub fn get_directory_at_cluster(
        &mut self,
        mut cluster: u32,
    ) -> Result<Vec<FatDirectoryEntry>, Box<dyn Error + 'static>> {
        debug!("Retrieving directory at cluster {}", cluster);
        let bytes_per_cluster = self.boot_sector.get_bytes_per_cluster();
        let mut directory_entries = Vec::with_capacity(512);

        loop {
            let cluster_offset = self.boot_sector.get_cluster_offset(cluster) + self.offset;
            debug!("Current cluster is {} at offset {:x}", cluster, cluster_offset);
            self.reader.seek(SeekFrom::Start(cluster_offset))?;
            let mut directory_entry_bytes = vec![0; bytes_per_cluster];
            self.reader.read_exact(&mut directory_entry_bytes)?;

            for i in (0..bytes_per_cluster).step_by(FAT_DIRECTORY_ENTRY_SIZE) {
                let directory_entry = FatDirectoryEntry::from_data(
                    &directory_entry_bytes[i..i + FAT_DIRECTORY_ENTRY_SIZE],
                    self.fat_type,
                );
                directory_entries.push(directory_entry);
            }

            cluster = self.fat_tables[0][cluster as usize];
            if cluster == 0
                || cluster == 1
                || (self.fat_type == FatType::Fat12 && cluster >= 0xff8)
                || (self.fat_type == FatType::Fat16 && cluster >= 0xfff8)
                || (self.fat_type == FatType::Fat32 && cluster & 0x0fff_ffff >= 0x0fff_fff8)
            {
                break;
            }
        }

        Ok(directory_entries)
    }
}

#[derive(Debug)]
pub struct FatBootSector {
    pub jump_instruction: [u8; 3],
    pub oem_name: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub number_of_fats: u8,
    pub root_directory_entries: u16,
    pub sectors_in_filesystem: u32,
    pub media_descriptor: u8,
    pub sectors_per_fat: u32,
    pub sectors_per_track: u16,
    pub number_of_heads: u16,
    pub hidden_sectors: u32,
    pub signature: [u8; 2],
    pub extra: FatBootSectorExtra,
}

impl FatBootSector {
    pub fn from_partition_image<R>(reader: &mut R, start_pos: u64) -> Result<Self, Box<dyn Error + 'static>>
    where
        R: Read + Seek,
    {
        let mut data: [u8; BOOT_SECTOR_SIZE] = [0; BOOT_SECTOR_SIZE];
        reader.seek(SeekFrom::Start(start_pos))?;
        reader.read_exact(&mut data)?;

        // Read the BIOS Parameter Block. The format, alas, varies grossly by FAT type, which we have to deduce via
        // heuristics.
        let signature: [u8; 2] = data[510..512].try_into().unwrap();
        if signature != [0x55, 0xAA] {
            return Err(ImageError::InvalidSignature(signature).into());
        }

        let jump_instruction: [u8; 3] = [data[0], data[1], data[2]];
        let oem_name: [u8; 8] = data[0x03..0x0b].try_into().unwrap();
        let bytes_per_sector: u16 = u16::from_le_bytes(data[0x0b..0x0d].try_into().unwrap());
        let sectors_per_cluster: u8 = data[0x0d];
        let reserved_sectors: u16 = u16::from_le_bytes(data[0x0e..0x10].try_into().unwrap());
        let number_of_fats: u8 = data[0x10];
        let root_directory_entries: u16 = u16::from_le_bytes(data[0x11..0x13].try_into().unwrap());

        let sectors_in_filesystem_fat12_fat16 = u16::from_le_bytes(data[0x13..0x15].try_into().unwrap());
        let sectors_in_filesystem: u32 = match sectors_in_filesystem_fat12_fat16 {
            0 => {
                // FAT32; use the 4-byte value at 0x20.
                u32::from_le_bytes(data[0x20..0x24].try_into().unwrap())
            }
            value => value as u32,
        };

        let media_descriptor: u8 = data[0x15];
        let sectors_per_fat: u32 = match u16::from_le_bytes(data[0x16..0x18].try_into().unwrap()) {
            0 => {
                // FAT32; use the 4-byte value 0x24.
                u32::from_le_bytes(data[0x24..0x28].try_into().unwrap())
            }
            value => value as u32,
        };
        let sectors_per_track: u16 = u16::from_le_bytes(data[0x18..0x1a].try_into().unwrap());
        let number_of_heads: u16 = u16::from_le_bytes(data[0x1a..0x1c].try_into().unwrap());
        let hidden_sectors: u32 = u32::from_le_bytes(data[0x1c..0x20].try_into().unwrap());

        let data_sectors = sectors_in_filesystem
            - reserved_sectors as u32
            - (number_of_fats as u32 * sectors_per_fat)
            - (root_directory_entries as u32 * FAT_DIRECTORY_ENTRY_SIZE as u32 / bytes_per_sector as u32);

        let data_clusters = data_sectors / sectors_per_cluster as u32;

        // This logic is from https://www.win.tue.nl/~aeb/linux/fs/fat/fat-1.html
        let extra = match data_clusters {
            _ if data_clusters < 4085 => FatBootSectorExtra::Fat12(Fat12BootExtra {}),
            _ if data_clusters >= 4085 && data_clusters < 65525 => {
                // FAT 16
                let logical_drive_number: u8 = data[0x24];
                let flags: u8 = data[0x25];
                let extended_signature: u8 = data[0x26];
                let serial_number = match extended_signature {
                    0x28 | 0x29 => Some(u32::from_le_bytes(data[39..43].try_into().unwrap())),
                    _ => None,
                };
                let volume_label = match extended_signature {
                    0x29 => Some(data[0x2b..0x36].try_into().unwrap()),
                    _ => None,
                };
                let file_system_type = match extended_signature {
                    0x29 => Some(data[0x36..0x3e].try_into().unwrap()),
                    _ => None,
                };

                FatBootSectorExtra::Fat16(Fat16BootExtra {
                    logical_drive_number,
                    flags,
                    extended_signature,
                    serial_number,
                    volume_label,
                    file_system_type,
                })
            }
            _ => {
                // FAT 32
                let mirror_flags = u16::from_le_bytes(data[0x28..0x2a].try_into().unwrap());
                let filesystem_version = u16::from_le_bytes(data[0x2a..0x2c].try_into().unwrap());
                let root_directory_cluster = u32::from_le_bytes(data[0x2c..0x30].try_into().unwrap());
                let fsinfo_sector = u16::from_le_bytes(data[0x30..0x32].try_into().unwrap());
                let backup_boot_sector = u16::from_le_bytes(data[0x32..0x34].try_into().unwrap());
                let reserved: [u8; 12] = data[0x34..0x40].try_into().unwrap();
                let logical_drive_number: u8 = data[0x40];
                let reserved2: u8 = data[0x41];
                let extended_signature: u8 = data[0x37];
                let serial_number = match extended_signature {
                    0x28 | 0x29 => Some(u32::from_le_bytes(data[67..71].try_into().unwrap())),
                    _ => None,
                };
                let volume_label = match extended_signature {
                    0x29 => Some(data[71..82].try_into().unwrap()),
                    _ => None,
                };
                let file_system_type = match extended_signature {
                    0x29 => Some(data[82..90].try_into().unwrap()),
                    _ => None,
                };

                FatBootSectorExtra::Fat32(Fat32BootExtra {
                    mirror_flags,
                    filesystem_version,
                    root_directory_cluster,
                    fsinfo_sector,
                    backup_boot_sector,
                    reserved,
                    logical_drive_number,
                    reserved2,
                    extended_signature,
                    serial_number,
                    volume_label,
                    file_system_type,
                })
            }
        };

        Ok(Self {
            jump_instruction,
            oem_name,
            bytes_per_sector,
            sectors_per_cluster,
            reserved_sectors,
            hidden_sectors,
            number_of_fats,
            root_directory_entries,
            sectors_in_filesystem,
            media_descriptor,
            sectors_per_fat,
            sectors_per_track,
            number_of_heads,
            signature,
            extra,
        })
    }

    pub fn get_root_directory_offset(&self) -> u64 {
        self.get_fat_table_offset(self.number_of_fats as usize)
    }

    pub fn get_data_start_offset(&self) -> u64 {
        self.get_root_directory_offset() + (self.root_directory_entries as u64 * FAT_DIRECTORY_ENTRY_SIZE as u64)
    }

    pub fn get_cluster_offset(&self, cluster: u32) -> u64 {
        let result = self.get_data_start_offset()
            + self.sectors_per_cluster as u64 * self.bytes_per_sector as u64 * (cluster - 2) as u64;
        debug!(
            "get_cluster_offset: data_start=0x{:x}, sectors_per_cluster={}, bytes_per_sector={}, cluster={}, result=0x{:x}",
            self.get_data_start_offset(), self.sectors_per_cluster, self.bytes_per_sector, cluster, result
        );
        result
    }

    pub fn get_bytes_per_cluster(&self) -> usize {
        self.sectors_per_cluster as usize * self.bytes_per_sector as usize
    }

    pub fn get_fat_table_offset(&self, fat_index: usize) -> u64 {
        let result = (self.reserved_sectors as u64 + fat_index as u64 * self.sectors_per_fat as u64)
            * self.bytes_per_sector as u64;
        debug!("FAT table {} is at 0x{:x}", fat_index, result);
        result
    }
}

impl Display for FatBootSector {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let media_descriptor_string = FAT_MEDIA_TYPES.get(&self.media_descriptor).unwrap_or(&"Unknown");
        write!(
            f,
            "OEM name: {}\nBytes per sector: {}\nSectors per cluster: {}\nReserved sectors: {}\nNumber of FATs: {}\n\
             Max root entries: {}\nFilesystem sectors: {}\nMedia descriptor: 0x{:02x} ({})\nSectors per FAT: {}\n\
             Sectors per track: {}\nNumber of heads: {}\nHidden sectors: {}\n",
            String::from_utf8_lossy(&self.oem_name),
            self.bytes_per_sector,
            self.sectors_per_cluster,
            self.reserved_sectors,
            self.number_of_fats,
            self.root_directory_entries,
            self.sectors_in_filesystem,
            self.media_descriptor,
            media_descriptor_string,
            self.sectors_per_fat,
            self.sectors_per_track,
            self.number_of_heads,
            self.hidden_sectors
        )?;

        match &self.extra {
            FatBootSectorExtra::Fat12(extra) => extra.fmt(f),
            FatBootSectorExtra::Fat16(extra) => extra.fmt(f),
            FatBootSectorExtra::Fat32(extra) => extra.fmt(f),
        }
    }
}

#[derive(Debug)]
pub struct Fat12BootExtra {}

impl Display for Fat12BootExtra {
    fn fmt(&self, _: &mut Formatter<'_>) -> FmtResult {
        Ok(())
    }
}

#[derive(Debug)]
pub struct Fat16BootExtra {
    pub logical_drive_number: u8,
    pub flags: u8,
    pub extended_signature: u8,
    pub serial_number: Option<u32>,
    pub volume_label: Option<[u8; 11]>,
    pub file_system_type: Option<[u8; 8]>,
}

impl Display for Fat16BootExtra {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "Logical drive number: {}\n\
             Flags: 0x{:02x}\n\
             Extended signature: 0x{:02x}",
            self.logical_drive_number, self.flags, self.extended_signature,
        )?;

        if let Some(serial_number) = self.serial_number {
            write!(f, "\nSerial number: {:08x}", serial_number)?;
        }

        if let Some(volume_label) = self.volume_label {
            write!(f, "\nVolume label: {}", String::from_cp437(volume_label, &CP437_WINGDINGS))?;
        }

        if let Some(file_system_type) = self.file_system_type {
            write!(f, "\nFile system type: {}", String::from_cp437(file_system_type, &CP437_WINGDINGS))?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Fat32BootExtra {
    pub mirror_flags: u16,
    pub filesystem_version: u16,
    pub root_directory_cluster: u32,
    pub fsinfo_sector: u16,
    pub backup_boot_sector: u16,
    pub reserved: [u8; 12],
    pub logical_drive_number: u8,
    pub reserved2: u8,
    pub extended_signature: u8,
    pub serial_number: Option<u32>,
    pub volume_label: Option<[u8; 11]>,
    pub file_system_type: Option<[u8; 8]>,
}

impl Display for Fat32BootExtra {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "Mirror flags: 0x{:04x}\n\
             Filesystem version: 0x{:04x}\n\
             Root directory cluster: {}\n\
             Filesystem info sector: {}\n\
             Backup boot sector: {}\n\
             Logical drive number: {}\n\
             Reserved2: 0x{:02x}\n\
             Extended signature: 0x{:02x}",
            self.mirror_flags,
            self.filesystem_version,
            self.root_directory_cluster,
            self.fsinfo_sector,
            self.backup_boot_sector,
            self.logical_drive_number,
            self.reserved2,
            self.extended_signature,
        )?;

        if let Some(serial_number) = self.serial_number {
            write!(f, "\nSerial number: {:08x}", serial_number)?;
        }

        if let Some(volume_label) = self.volume_label {
            write!(f, "\nVolume label: {}", String::from_cp437(volume_label, &CP437_WINGDINGS))?;
        }

        if let Some(file_system_type) = self.file_system_type {
            write!(f, "\nFile system type: {}", String::from_cp437(file_system_type, &CP437_WINGDINGS))?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum FatBootSectorExtra {
    Fat12(Fat12BootExtra),
    Fat16(Fat16BootExtra),
    Fat32(Fat32BootExtra),
}

impl Display for FatBootSectorExtra {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Fat12(extra) => extra.fmt(f),
            Self::Fat16(extra) => extra.fmt(f),
            Self::Fat32(extra) => extra.fmt(f),
        }
    }
}

#[derive(Debug)]
pub struct FatDirectoryEntry {
    pub filename: [u8; 8],
    pub extension: [u8; 3],
    pub attributes: u8,
    pub reserved: u8,
    pub creation_timestamp: Option<NaiveDateTime>,
    pub last_access_date: Option<NaiveDate>,
    pub extended_attributes_cluster: Option<u32>,
    pub last_modification_timestamp: Option<NaiveDateTime>,
    pub first_cluster: u32,
    pub file_size: u32,
}

impl FatDirectoryEntry {
    pub fn from_data(data: &[u8], fat_type: FatType) -> Self {
        let filename = data[0..8].try_into().unwrap();
        let extension = data[8..11].try_into().unwrap();
        let attributes = data[11];
        let reserved = data[12];

        let creation_time = fat_fine_time_to_chrono_naive_time(data[13..16].try_into().unwrap());
        let creation_date = fat_date_to_chrono_naive_date(data[16..18].try_into().unwrap());
        let creation_timestamp = match creation_date {
            Some(cd) => match creation_time {
                Some(ct) => Some(NaiveDateTime::new(cd, ct)),
                None => {
                    warn!("Got a valid creation date but invalid creation time: {:02x?}", &data[13..16]);
                    None
                }
            },
            None => None,
        };
        let last_access_date = fat_date_to_chrono_naive_date(data[18..20].try_into().unwrap());

        let (extended_attributes_cluster, cluster_start_high) = match fat_type {
            FatType::Fat12 | FatType::Fat16 => {
                (Some(u16::from_le_bytes(data[20..22].try_into().unwrap()) as u32), 0u32)
            }
            FatType::Fat32 => (None, (u16::from_le_bytes(data[20..22].try_into().unwrap()) as u32) << 16),
        };

        let last_modification_time = fat_time_to_chrono_naive_time(data[22..24].try_into().unwrap());
        let last_modification_date = fat_date_to_chrono_naive_date(data[24..26].try_into().unwrap());
        let last_modification_timestamp = match last_modification_date {
            Some(lmd) => match last_modification_time {
                Some(lmt) => Some(NaiveDateTime::new(lmd, lmt)),
                None => {
                    warn!(
                        "Got a last modification date, but no last modification time: time_data={:02x?}",
                        &data[22..24]
                    );
                    None
                }
            },
            None => None,
        };
        let first_cluster = u16::from_le_bytes(data[26..28].try_into().unwrap()) as u32 | cluster_start_high;
        let file_size = u32::from_le_bytes(data[28..32].try_into().unwrap());

        Self {
            filename,
            extension,
            attributes,
            reserved,
            creation_timestamp,
            last_access_date,
            extended_attributes_cluster,
            last_modification_timestamp,
            first_cluster,
            file_size,
        }
    }

    pub fn get_attribute_flags(&self) -> String {
        if self.attributes == FAT_ATTRIBUTE_LONG_FILENAME {
            "<LFN>".to_string()
        } else {
            format!(
                "{}{}{}{}{}{}{}",
                if self.attributes & FAT_ATTRIBUTE_DEVICE != 0 { "!" } else { " " },
                if self.attributes & FAT_ATTRIBUTE_ARCHIVE != 0 { "A" } else { " " },
                if self.attributes & FAT_ATTRIBUTE_DIRECTORY != 0 { "D" } else { " " },
                if self.attributes & FAT_ATTRIBUTE_VOLUME_LABEL != 0 { "V" } else { " " },
                if self.attributes & FAT_ATTRIBUTE_SYSTEM != 0 { "S" } else { " " },
                if self.attributes & FAT_ATTRIBUTE_HIDDEN != 0 { "H" } else { " " },
                if self.attributes & FAT_ATTRIBUTE_READ_ONLY != 0 { "R" } else { " " }
            )
        }
    }

    pub fn is_valid(&self) -> bool {
        self.filename[0] != 0 && self.extension[0] != 0
    }

    pub fn is_directory(&self) -> bool {
        self.is_valid() && self.attributes & FAT_ATTRIBUTE_DIRECTORY != 0
    }

    pub fn get_directory_entries<R: Read + Seek>(
        &self,
        fp: &mut FatPartition<R>,
    ) -> Result<Vec<FatDirectoryEntry>, Box<dyn Error + 'static>> {
        fp.get_directory_at_cluster(self.first_cluster)
    }

    pub fn get_filename(&self) -> Option<String> {
        if self.filename[0] == 0 {
            return None;
        }

        let mut basename = Vec::with_capacity(12);

        match self.filename[0] {
            0x05 => {
                basename.push(0xe5);
                basename.extend_from_slice(&self.filename[1..8]);
            }
            0xe5 => {
                basename.push('?' as u8);
                basename.extend_from_slice(&self.filename[1..8]);
            }
            _ => basename.extend_from_slice(&self.filename[0..8]),
        };

        let basename = String::from_cp437(basename, &CP437_WINGDINGS).trim_end_matches(' ').to_string();
        let ext = String::from_cp437(self.extension, &CP437_WINGDINGS).trim_end_matches(' ').to_string();

        if ext.is_empty() {
            Some(basename)
        } else {
            Some(format!("{}.{}", basename, ext))
        }
    }
}

impl Display for FatDirectoryEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let lmt = match self.last_modification_timestamp {
            Some(lmt) => lmt.to_string(),
            None => "    ".into(),
        };
        write!(f, "{:-12} {} {}", self.get_filename().unwrap_or("".to_string()), self.get_attribute_flags(), lmt)
    }
}

fn fat_date_to_chrono_naive_date(data: [u8; 2]) -> Option<NaiveDate> {
    let ymd = u16::from_le_bytes(data);
    let year = ((ymd & 0xfe00) >> 9) + 1980;
    let month = (ymd & 0x01e0) >> 5;
    let day = ymd & 0x001f;

    NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)
}

fn fat_fine_time_to_chrono_naive_time(data: [u8; 3]) -> Option<NaiveTime> {
    let centi_millis = data[0] as u32;
    let hms = u16::from_le_bytes(data[1..3].try_into().unwrap()) as u32;
    let hour = (hms & 0xf800) >> 11 as u32;
    let minute = (hms & 0x07e0) >> 5 as u32;

    let seconds = centi_millis / 100;
    let milliseconds = 10 * (centi_millis % 100);

    NaiveTime::from_hms_milli_opt(hour, minute, seconds, milliseconds)
}

fn fat_time_to_chrono_naive_time(data: [u8; 2]) -> Option<NaiveTime> {
    let hms = u16::from_le_bytes(data) as u32;
    let hour = (hms & 0xf800) >> 11 as u32;
    let minute = (hms & 0x07e0) >> 5 as u32;
    let seconds = (hms & 0x001f) as u32 * 2;

    NaiveTime::from_hms_opt(hour, minute, seconds)
}
