use std::{
    convert::TryInto,
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
    io::{Read, Result as IoResult, Seek, SeekFrom},
};

use crate::errors::ImageError;

pub const BOOT_SECTOR_SIZE: usize = 512;
pub const BOOT_SECTOR_SIGNATURE: &[u8; 2] = b"\x55\xAA";
pub const BOOT_SECTOR_SIGNATURE_OFFSET: usize = 510;

pub const CHS_SIZE: usize = 3;
pub const PARTITION_TABLE_OFFSET: usize = 446;
pub const PARTITION_TABLE_ENTRIES: usize = 4;
pub const PARTITION_ENTRY_SIZE: usize = 16;

#[derive(Debug)]
pub struct BootSector {
    pub partitions: [PartitionEntry; PARTITION_TABLE_ENTRIES],
    pub signature: [u8; 2],
}

impl BootSector {
    pub fn from_disk_image<R>(reader: &mut R, start_pos: u64) -> IoResult<Self>
    where
        R: Read + Seek,
    {
        let mut data: [u8; BOOT_SECTOR_SIZE] = [0; BOOT_SECTOR_SIZE];
        reader.seek(SeekFrom::Start(start_pos))?;
        reader.read_exact(&mut data)?;

        Ok(Self {
            partitions: [
                PartitionEntry::new(&data[PARTITION_TABLE_OFFSET..]),
                PartitionEntry::new(&data[PARTITION_TABLE_OFFSET + PARTITION_ENTRY_SIZE..]),
                PartitionEntry::new(&data[PARTITION_TABLE_OFFSET + 2 * PARTITION_ENTRY_SIZE..]),
                PartitionEntry::new(&data[PARTITION_TABLE_OFFSET + 3 * PARTITION_ENTRY_SIZE..]),
            ],
            signature: data[BOOT_SECTOR_SIGNATURE_OFFSET..BOOT_SECTOR_SIGNATURE_OFFSET + 2].try_into().unwrap(),
        })
    }
}

#[derive(Debug)]
pub struct CHSPosition {
    pub cylinder: u16,
    pub head: u8,
    pub sector: u8,
}

impl CHSPosition {
    pub fn new(data: &[u8]) -> Self {
        if data.len() < CHS_SIZE {
            panic!("CHS data is too small; expected at least {} bytes, got {}", CHS_SIZE, data.len());
        }

        Self {
            cylinder: (((data[1] & 0xc0) as u16) << 2) | data[2] as u16,
            head: data[0],
            sector: data[1] & 0x3f,
        }
    }
}

impl Display for CHSPosition {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}:{}:{}", self.cylinder, self.head, self.sector)
    }
}

#[derive(Debug)]
pub enum PartitionStatusFlag {
    Bootable,
    Unknown(u8),
}

impl PartitionStatusFlag {
    fn from_u8(value: u8) -> Vec<Self> {
        let mut result = Vec::new();
        if value & 0x80 != 0 {
            result.push(Self::Bootable);
        }

        for i in 0..7 {
            if value & (1 << i) != 0 {
                result.push(Self::Unknown(i));
            }
        }

        result
    }
}

impl Display for PartitionStatusFlag {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Bootable => f.write_str("bootable"),
            Self::Unknown(value) => write!(f, "unknown(bit {})", value),
        }
    }
}

#[derive(Debug)]
pub struct MBRPartitionType {
    pub code: u8,
    pub name: &'static str,
    pub is_extended: bool,
}

impl MBRPartitionType {
    const fn regular(code: u8, name: &'static str) -> Self {
        Self {
            code,
            name,
            is_extended: false,
        }
    }

    const fn extended(code: u8, name: &'static str) -> Self {
        Self {
            code,
            name,
            is_extended: true,
        }
    }
}

// See https://en.wikipedia.org/wiki/Partition_type for a list of MBR partition types.
// A few missing entries are gleaned from util-linux/include/pt-mbr-partnames.h (noted below).
pub const MBR_PARTITION_TYPES: [MBRPartitionType; 256] = [
    MBRPartitionType::regular(0x00, "Empty"),
    MBRPartitionType::regular(0x01, "FAT12"),
    MBRPartitionType::regular(0x02, "XENIX root"),
    MBRPartitionType::regular(0x03, "XENIX usr"),
    MBRPartitionType::regular(0x04, "FAT16 <32 MB"),
    MBRPartitionType::extended(0x05, "Extended"),
    MBRPartitionType::regular(0x06, "FAT16B"),
    MBRPartitionType::regular(0x07, "HPFS; NTFS; exFAT"),
    MBRPartitionType::regular(0x08, "AIX boot"),
    MBRPartitionType::regular(0x09, "AIX data"),
    MBRPartitionType::regular(0x0a, "OS/2 Boot Manager"),
    MBRPartitionType::regular(0x0b, "FAT32"),
    MBRPartitionType::regular(0x0c, "FAT32 LBA"),
    MBRPartitionType::regular(0x0d, "Unknown"),
    MBRPartitionType::regular(0x0e, "FAT16 LBA"),
    MBRPartitionType::extended(0x0f, "Extended LBA"),
    MBRPartitionType::regular(0x10, "OPUS"), // from util-linux
    MBRPartitionType::regular(0x11, "Hidden FAT12"),
    MBRPartitionType::regular(0x12, "Compaq diag; EISA config; NCR firmware; IBM rescue"),
    MBRPartitionType::regular(0x13, "Unknown"),
    MBRPartitionType::regular(0x14, "OS/2 hidden FAT16"),
    MBRPartitionType::extended(0x15, "OS/2 hidden extended CHS"),
    MBRPartitionType::regular(0x16, "OS/2 hidden FAT16B"),
    MBRPartitionType::regular(0x17, "OS/2 Hidden IFS; HPFS; NTFS; exFAT"),
    MBRPartitionType::regular(0x18, "AST SmartSleep"),
    MBRPartitionType::regular(0x19, "Willowtech Photon coS"),
    MBRPartitionType::regular(0x1a, "Unknown"),
    MBRPartitionType::regular(0x1b, "OS/2 hidden FAT32"),
    MBRPartitionType::regular(0x1c, "OS/2 hidden FAT32 LBA"),
    MBRPartitionType::regular(0x1d, "Unknown"),
    MBRPartitionType::regular(0x1e, "OS/2 hidden FAT16 LBA"),
    MBRPartitionType::extended(0x1f, "OS/2 hidden extended LBA"),
    MBRPartitionType::regular(0x20, "Windows Mobile update"),
    MBRPartitionType::regular(0x21, "Oxygen FSo2"),
    MBRPartitionType::regular(0x22, "Oxygen extended"),
    MBRPartitionType::regular(0x23, "Windows Mobile boot"),
    MBRPartitionType::regular(0x24, "NEC DOS"),
    MBRPartitionType::regular(0x25, "Unknown"),
    MBRPartitionType::regular(0x26, "Unknown"),
    MBRPartitionType::regular(0x27, "WinRE (hidden NTFS)"),
    MBRPartitionType::regular(0x28, "Unknown"),
    MBRPartitionType::regular(0x29, "Unknown"),
    MBRPartitionType::regular(0x2a, "AtheOS AthFS"),
    MBRPartitionType::regular(0x2b, "Syllable Secure"),
    MBRPartitionType::regular(0x2c, "Unknown"),
    MBRPartitionType::regular(0x2d, "Unknown"),
    MBRPartitionType::regular(0x2e, "Unknown"),
    MBRPartitionType::regular(0x2f, "Unknown"),
    MBRPartitionType::regular(0x30, "Unknown"),
    MBRPartitionType::regular(0x31, "Unknown"),
    MBRPartitionType::regular(0x32, "Unknown"),
    MBRPartitionType::regular(0x33, "Unknown"),
    MBRPartitionType::regular(0x34, "Unknown"),
    MBRPartitionType::regular(0x35, "JFS"),
    MBRPartitionType::regular(0x36, "Unknown"),
    MBRPartitionType::regular(0x37, "Unknown"),
    MBRPartitionType::regular(0x38, "THEOSv3.2"),
    MBRPartitionType::regular(0x39, "Plan 9; THEOSv4 spanned"),
    MBRPartitionType::regular(0x3a, "THEOSv4 4 GB"),
    MBRPartitionType::regular(0x3b, "THEOSv4 extended"),
    MBRPartitionType::regular(0x3c, "PartitionMagic recovery"),
    MBRPartitionType::regular(0x3d, "PartitionMagic hidden NetWare"),
    MBRPartitionType::regular(0x3e, "Unknown"),
    MBRPartitionType::regular(0x3f, "Unknown"),
    MBRPartitionType::regular(0x40, "Venix 80286; PICK R83"),
    MBRPartitionType::regular(0x41, "Minix; PPC PReP Boot"),
    MBRPartitionType::regular(0x42, "Secure File System; Old Linux swap"),
    MBRPartitionType::regular(0x43, "Old Linux native"),
    MBRPartitionType::regular(0x44, "Norton GoBack"),
    MBRPartitionType::regular(0x45, "Boot-US boot manager; Priam; EUMEL-ELAN (L2)"),
    MBRPartitionType::regular(0x46, "EUMEL-ELAN (L2)"),
    MBRPartitionType::regular(0x47, "EUMEL-ELAN (L2)"),
    MBRPartitionType::regular(0x48, "EUMEL-ELAN (L2); ERGOS (L3)"),
    MBRPartitionType::regular(0x49, "Unknown"),
    MBRPartitionType::regular(0x4a, "AdaOS Aquila; ALFS-THIN"),
    MBRPartitionType::regular(0x4b, "Unknown"),
    MBRPartitionType::regular(0x4c, "Oberon Aos (A2)"),
    MBRPartitionType::regular(0x4d, "QNX4.x primary"),
    MBRPartitionType::regular(0x4e, "QNX4.x secondary"),
    MBRPartitionType::regular(0x4f, "QNX4.x tertiary; Oberon boot"),
    MBRPartitionType::regular(0x50, "OnTrack Disk Manager; Oberon alternative; Lynx"),
    MBRPartitionType::regular(0x51, "OnTrack Disk Manager aux 1; Novell"),
    MBRPartitionType::regular(0x52, "CP/M-80; Microport SysV/AT"),
    MBRPartitionType::regular(0x53, "OnTrack Disk Manager aux 3"),
    MBRPartitionType::regular(0x54, "OnTrack Disk Manager dynamic driver overlay"),
    MBRPartitionType::regular(0x55, "EZ-Drive"),
    MBRPartitionType::regular(0x56, "AT&T DOS; EZ-Drive BIOS; Golden Bow VFeature"),
    MBRPartitionType::regular(0x57, "DrivePro; Novell VNDI"),
    MBRPartitionType::regular(0x58, "Unknown"),
    MBRPartitionType::regular(0x59, "yocOS yocFS"),
    MBRPartitionType::regular(0x5a, "Unknown"),
    MBRPartitionType::regular(0x5b, "Unknown"),
    MBRPartitionType::regular(0x5c, "Priam EDisk"),
    MBRPartitionType::regular(0x5d, "Unknown"),
    MBRPartitionType::regular(0x5e, "Unknown"),
    MBRPartitionType::regular(0x5f, "Unknown"),
    MBRPartitionType::regular(0x60, "Unknown"),
    MBRPartitionType::regular(0x61, "SpeedStor hidden FAT12"),
    MBRPartitionType::regular(0x62, "Unknown"),
    MBRPartitionType::regular(0x63, "SysV; GNU HURD; SpeedStor hidden read-only FAT12"),
    MBRPartitionType::regular(0x64, "Novell Netware 286; SpeedStor hidden FAT16; PC-ARMOUR"),
    MBRPartitionType::regular(0x65, "Novell Netware 386"),
    MBRPartitionType::regular(0x66, "Novell Netware Storage Management Services; Speedstor hidden read-only FAT16"),
    MBRPartitionType::regular(0x67, "Novell Netware Wolf Mountain"),
    MBRPartitionType::regular(0x68, "Novell Netware"),
    MBRPartitionType::regular(0x69, "Novell Netware 5"),
    MBRPartitionType::regular(0x6a, "Unknown"),
    MBRPartitionType::regular(0x6b, "Unknown"),
    MBRPartitionType::regular(0x6c, "DragonFly BSD slice"),
    MBRPartitionType::regular(0x6d, "Unknown"),
    MBRPartitionType::regular(0x6e, "Unknown"),
    MBRPartitionType::regular(0x6f, "Unknown"),
    MBRPartitionType::regular(0x70, "DiskSecure multiboot"),
    MBRPartitionType::regular(0x71, "Unknown"),
    MBRPartitionType::regular(0x72, "V7/x86; APTI alternative FAT12"),
    MBRPartitionType::regular(0x73, "Unknown"),
    MBRPartitionType::regular(0x74, "SpeedStor hidden FAT16B; V7/x86"),
    MBRPartitionType::regular(0x75, "PC/IX"),
    MBRPartitionType::regular(0x76, "SpeedStor hidden read-only FAT16B"),
    MBRPartitionType::regular(0x77, "Novell VNDI/M2FS/M2CS"),
    MBRPartitionType::regular(0x78, "XOSL bootloader"),
    MBRPartitionType::regular(0x79, "APTI alternative FAT16"),
    MBRPartitionType::regular(0x7a, "APTI alternative FAT16 LBA"),
    MBRPartitionType::regular(0x7b, "APTI alternative FAT16B"),
    MBRPartitionType::regular(0x7c, "APTI alternative FAT32 LBA"),
    MBRPartitionType::regular(0x7d, "APTI alternative FAT32"),
    MBRPartitionType::regular(0x7e, "PrimoCache L2"),
    MBRPartitionType::regular(0x7f, "User-defined"),
    MBRPartitionType::regular(0x80, "Minix old"),
    MBRPartitionType::regular(0x81, "Minix; Linux old"),
    MBRPartitionType::regular(0x82, "Linux swap; Solaris"),
    MBRPartitionType::regular(0x83, "Linux"),
    MBRPartitionType::regular(0x84, "OS/2 hidden; Intel hibernation"),
    MBRPartitionType::extended(0x85, "Linux extended"),
    MBRPartitionType::regular(0x86, "FAT16B volume set; Linux RAID"),
    MBRPartitionType::regular(0x87, "NTFS volume set"),
    MBRPartitionType::regular(0x88, "Linux plaintext"),
    MBRPartitionType::regular(0x89, "Unknown"),
    MBRPartitionType::regular(0x8a, "Unknown"),
    MBRPartitionType::regular(0x8b, "Unknown"),
    MBRPartitionType::regular(0x8c, "Unknown"),
    MBRPartitionType::regular(0x8d, "Unknown"),
    MBRPartitionType::regular(0x8e, "Linux LVM"),
    MBRPartitionType::regular(0x8f, "Unknown"),
    MBRPartitionType::regular(0x90, "FreeDOS hidden FAT16"),
    MBRPartitionType::extended(0x91, "FreeDOS hidden extended"),
    MBRPartitionType::regular(0x92, "FreeDOS hidden FAT16B"),
    MBRPartitionType::regular(0x93, "Amoeba; Linux hidden"),
    MBRPartitionType::regular(0x94, "Amoeba bad block table"),
    MBRPartitionType::regular(0x95, "EXOPC native"),
    MBRPartitionType::regular(0x96, "ISO-9660"),
    MBRPartitionType::regular(0x97, "FreeDOS FAT32"),
    MBRPartitionType::regular(0x98, "FreeDOS FAT32 LBA; Service"),
    MBRPartitionType::regular(0x99, "Unknown"),
    MBRPartitionType::regular(0x9a, "FreeDOS hidden FAT16B LBA"),
    MBRPartitionType::extended(0x9b, "FreeDOS hidden extended LBA"),
    MBRPartitionType::regular(0x9c, "Unknown"),
    MBRPartitionType::regular(0x9d, "Unknown"),
    MBRPartitionType::regular(0x9e, "ForthOS"),
    MBRPartitionType::regular(0x9f, "BSD/OS"),
    MBRPartitionType::regular(0xa0, "Hibernation"),
    MBRPartitionType::regular(0xa1, "Hibernation"),
    MBRPartitionType::regular(0xa2, "Hard Processor System preloader"),
    MBRPartitionType::regular(0xa3, "HP Volume Expansion"),
    MBRPartitionType::regular(0xa4, "HP Volume Expansion"),
    MBRPartitionType::regular(0xa5, "FreeBSD slice"),
    MBRPartitionType::regular(0xa6, "OpenBSD slice; HP Volume Expansion"),
    MBRPartitionType::regular(0xa7, "NeXTSTEP"),
    MBRPartitionType::regular(0xa8, "Darwin UFS"),
    MBRPartitionType::regular(0xa9, "NetBSD slice"),
    MBRPartitionType::regular(0xaa, "Olivetti FAT12"),
    MBRPartitionType::regular(0xab, "Darwin boot"),
    MBRPartitionType::regular(0xac, "Apple RAID"),
    MBRPartitionType::regular(0xad, "RiscOS ADFS"),
    MBRPartitionType::regular(0xae, "ShagOS"),
    MBRPartitionType::regular(0xaf, "HFS; HFS+"),
    MBRPartitionType::regular(0xb0, "BootStar dummy"),
    MBRPartitionType::regular(0xb1, "QNX Neutrino; HP Volume Expansion"),
    MBRPartitionType::regular(0xb2, "QNX Neutrino"),
    MBRPartitionType::regular(0xb3, "QNX Neutrino; HP Volume Expansion"),
    MBRPartitionType::regular(0xb4, "HP Volume Expansion"),
    MBRPartitionType::regular(0xb5, "Unknown"),
    MBRPartitionType::regular(0xb6, "FAT16B corrupted primary volume; HP Volume Expansion"),
    MBRPartitionType::regular(0xb7, "BSDI; HPFS/NTFS corrupted primary volume"),
    MBRPartitionType::regular(0xb8, "BSDI swap"),
    MBRPartitionType::regular(0xb9, "Unknown"),
    MBRPartitionType::regular(0xba, "Unknown"),
    MBRPartitionType::regular(0xbb, "Boot Wizard hidden; Acronis OEM Secure Zone; FAT32 corrupted primary volume"),
    MBRPartitionType::regular(
        0xbc,
        "Acronis Secure Zone; FAT32 LBA corrupted primary volume set; Paragon Backup Capsule",
    ),
    MBRPartitionType::regular(0xbd, "BonnyDOS/286"),
    MBRPartitionType::regular(0xbe, "Solaris boot"),
    MBRPartitionType::regular(0xbf, "Solaris"),
    MBRPartitionType::regular(0xc0, "DR-DOS secured FAT <32 MB"),
    MBRPartitionType::regular(0xc1, "DR-DOS secured FAT12"),
    MBRPartitionType::regular(0xc2, "Power Boot hidden Linux"),
    MBRPartitionType::regular(0xc3, "Power Boot hidden Linux swap"),
    MBRPartitionType::regular(0xc4, "DR-DOS secured FAT16"),
    MBRPartitionType::extended(0xc5, "DR-DOS secured extended"),
    MBRPartitionType::regular(0xc6, "DR-DOS secured FAT16B; FAT16B corrupted secondary volume"),
    MBRPartitionType::regular(0xc7, "Syrinx boot; HPFS/NTFS corrupted secondary volume"),
    MBRPartitionType::regular(0xc8, "Unknown"),
    MBRPartitionType::regular(0xc9, "Unknown"),
    MBRPartitionType::regular(0xca, "Unknown"),
    MBRPartitionType::regular(0xcb, "DR-DOS secured FAT32"),
    MBRPartitionType::regular(0xcc, "DR-DOS secured FAT32 LBA; FAT32 corrupted secondary volume"),
    MBRPartitionType::regular(0xcd, "CTOS memory dump"),
    MBRPartitionType::regular(0xce, "DR-DOS secured FAT16B LBA"),
    MBRPartitionType::regular(0xcf, "DR-DOS secured extended LBA"),
    MBRPartitionType::regular(0xd0, "Multiuser DOS secured FAT >32 MB"),
    MBRPartitionType::regular(0xd1, "Multiuser DOS secured FAT12"),
    MBRPartitionType::regular(0xd2, "Unknown"),
    MBRPartitionType::regular(0xd3, "Unknown"),
    MBRPartitionType::regular(0xd4, "Multiuser DOS secured FAT16"),
    MBRPartitionType::extended(0xd5, "Multiuser DOS secured extended CHS"),
    MBRPartitionType::regular(0xd6, "Multiuser DOS secured FAT16B"),
    MBRPartitionType::regular(0xd7, "Unknown"),
    MBRPartitionType::regular(0xd8, "CP/M-86"),
    MBRPartitionType::regular(0xd9, "Unknown"),
    MBRPartitionType::regular(0xda, "Non-filesystem data"),
    MBRPartitionType::regular(0xdb, "CP/M-86"),
    MBRPartitionType::regular(0xdc, "Unknown"),
    MBRPartitionType::regular(0xdd, "CTOS hidden memory dump"),
    MBRPartitionType::regular(0xde, "Dell FAT16 diagnostic/utility"),
    MBRPartitionType::regular(0xdf, "BootIt EMBRM; DG/UX"),
    MBRPartitionType::regular(0xe0, "ST AVFS"),
    MBRPartitionType::regular(0xe1, "SpeedStor FAT12 <16 MB"),
    MBRPartitionType::regular(0xe2, "Unknown"),
    MBRPartitionType::regular(0xe3, "SpeedStor read-only FAT12 <16 MB"),
    MBRPartitionType::regular(0xe4, "SpeedStor FAT16 <32 MB"),
    MBRPartitionType::regular(0xe5, "Tandy MS/DOS"),
    MBRPartitionType::regular(0xe6, "SpeedStor read-only FAT16 <32 MB"),
    MBRPartitionType::regular(0xe7, "Unknown"),
    MBRPartitionType::regular(0xe8, "Linux Unified Key Setup (LUKS)"),
    MBRPartitionType::regular(0xe9, "Unknown"),
    MBRPartitionType::extended(0xea, "Linux extended boot"), // util-linux
    MBRPartitionType::regular(0xeb, "BeOS/Haiku BFS"),
    MBRPartitionType::regular(0xec, "SkyOS SkyFS"),
    MBRPartitionType::regular(0xed, "Sprytix EDC loader"),
    MBRPartitionType::regular(0xee, "GPT"),
    MBRPartitionType::regular(0xef, "EFI system"),
    MBRPartitionType::regular(0xf0, "Linux/PA-RISC boot"),
    MBRPartitionType::regular(0xf1, "SpeedStor"), // util-linux
    MBRPartitionType::regular(0xf2, "Unisys FAT12/FAT16 secondary"),
    MBRPartitionType::regular(0xf3, "Unknown"),
    MBRPartitionType::regular(0xf4, "SpeedStor FAT16; Prologue single-volume NGF/TwinFS"),
    MBRPartitionType::regular(0xf5, "Prologue multi-volume NGF/TwinFS"),
    MBRPartitionType::regular(0xf6, "SpeedStor read-only FAT16B"),
    MBRPartitionType::regular(0xf7, "OSG EFAT; X1 solid state"),
    MBRPartitionType::regular(0xf8, "Arm EBBR system firmware"),
    MBRPartitionType::regular(0xf9, "Linux pCache EXT2/EXT3"),
    MBRPartitionType::regular(0xfa, "Unknown"),
    MBRPartitionType::regular(0xfb, "VMware VMFS"),
    MBRPartitionType::regular(0xfc, "VMware swap/VMKCORE"),
    MBRPartitionType::regular(0xfd, "Linux raid autodetect"),
    MBRPartitionType::regular(0xfe, "PS/2 recovery; Old Linux LVM"),
    MBRPartitionType::regular(0xff, "Xenix bad block table"),
];

impl Display for MBRPartitionType {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "0x{:02x} ({})", self.code, self.name)
    }
}

#[derive(Debug)]
pub struct PartitionEntry {
    pub status: Vec<PartitionStatusFlag>,
    pub chs_start: CHSPosition,
    pub partition_type: &'static MBRPartitionType,
    pub chs_end: CHSPosition,
    pub lba_start: u32,
    pub sector_count: u32,
}

impl PartitionEntry {
    pub fn new(data: &[u8]) -> Self {
        if data.len() < PARTITION_ENTRY_SIZE {
            panic!(
                "Partition entry data is too small; expected at least {} bytes, got {}",
                PARTITION_ENTRY_SIZE,
                data.len()
            );
        }

        Self {
            status: PartitionStatusFlag::from_u8(data[0]),
            chs_start: CHSPosition::new(&data[1..4]),
            partition_type: &MBR_PARTITION_TYPES[data[4] as usize],
            chs_end: CHSPosition::new(&data[5..8]),
            lba_start: u32::from_le_bytes(data[8..12].try_into().unwrap()),
            sector_count: u32::from_le_bytes(data[12..16].try_into().unwrap()),
        }
    }

    pub fn get_extended_boot_sector<R>(
        &self,
        reader: &mut R,
        my_boot_sector_start_pos: u64,
    ) -> Result<(BootSector, u64), Box<dyn Error>>
    where
        R: Read + Seek,
    {
        if !self.partition_type.is_extended {
            let mut extended_types = Vec::new();
            MBR_PARTITION_TYPES.iter().for_each(|t| {
                if t.is_extended {
                    extended_types.push(format!("{:02x}", t.code));
                }
            });
            return Err(ImageError::InvalidPartitionType {
                expected: extended_types.join("/"),
                actual: format!("{:02x}", self.partition_type.code).into(),
            }
            .into());
        }

        if self.lba_start == 0 {
            return Err(ImageError::InvalidPartitionEntry("Cannot handle CHS extended partitions".into()).into());
        }

        let start_pos = my_boot_sector_start_pos + self.lba_start as u64 * 512;
        Ok((BootSector::from_disk_image(reader, start_pos)?, start_pos))
    }

    pub fn is_extended(&self) -> bool {
        self.partition_type.is_extended
    }
}

impl Display for PartitionEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let status_str = self.status.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(" ");
        write!(
            f,
            "Partition Type: {}\nStatus: {}\nStart CHS: {}\nEnd CHS: {}\n\
            LBA Start: {} (0x{:x})\nSector Count: {} (0x{:x})",
            self.partition_type,
            status_str,
            self.chs_start,
            self.chs_end,
            self.lba_start,
            self.lba_start,
            self.sector_count,
            self.sector_count
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_mbr_partition_entries() {
        for i in 0..=255 {
            assert_eq!(MBR_PARTITION_TYPES[i as usize].code, i);
        }
    }
}
