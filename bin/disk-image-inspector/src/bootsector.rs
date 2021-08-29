use std::{
    convert::TryInto,
    fmt::{Display, Formatter, Result as FmtResult},
};

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
    pub fn new(data: &[u8]) -> Self {
        if data.len() < BOOT_SECTOR_SIZE {
            panic!("Boot sector data is too small; expected at least {} bytes, got {}", BOOT_SECTOR_SIZE, data.len());
        }

        Self {
            partitions: [
                PartitionEntry::new(&data[PARTITION_TABLE_OFFSET..]),
                PartitionEntry::new(&data[PARTITION_TABLE_OFFSET + PARTITION_ENTRY_SIZE..]),
                PartitionEntry::new(&data[PARTITION_TABLE_OFFSET + 2 * PARTITION_ENTRY_SIZE..]),
                PartitionEntry::new(&data[PARTITION_TABLE_OFFSET + 3 * PARTITION_ENTRY_SIZE..]),
            ],
            signature: data[BOOT_SECTOR_SIGNATURE_OFFSET..BOOT_SECTOR_SIGNATURE_OFFSET + 2].try_into().unwrap(),
        }
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
}

impl MBRPartitionType {
    const fn new(code: u8, name: &'static str) -> Self {
        Self { code, name }
    }
}

// See https://en.wikipedia.org/wiki/Partition_type for a list of MBR partition types.
// A few missing entries are gleaned from util-linux/include/pt-mbr-partnames.h (noted below).
pub const MBR_PARTITION_TYPES: [MBRPartitionType; 256] = [
    MBRPartitionType::new(0x00, "Empty"),
    MBRPartitionType::new(0x01, "FAT12"),
    MBRPartitionType::new(0x02, "XENIX root"),
    MBRPartitionType::new(0x03, "XENIX usr"),
    MBRPartitionType::new(0x04, "FAT16 <32 MB"),
    MBRPartitionType::new(0x05, "Extended"),
    MBRPartitionType::new(0x06, "FAT16B"),
    MBRPartitionType::new(0x07, "HPFS; NTFS; exFAT"),
    MBRPartitionType::new(0x08, "AIX boot"),
    MBRPartitionType::new(0x09, "AIX data"),
    MBRPartitionType::new(0x0a, "OS/2 Boot Manager"),
    MBRPartitionType::new(0x0b, "FAT32"),
    MBRPartitionType::new(0x0c, "FAT32 LBA"),
    MBRPartitionType::new(0x0d, "Unknown"),
    MBRPartitionType::new(0x0e, "FAT16 LBA"),
    MBRPartitionType::new(0x0f, "Extended LBA"),
    MBRPartitionType::new(0x10, "OPUS"), // from util-linux
    MBRPartitionType::new(0x11, "Hidden FAT12"),
    MBRPartitionType::new(0x12, "Compaq diag; EISA config; NCR firmware; IBM rescue"),
    MBRPartitionType::new(0x13, "Unknown"),
    MBRPartitionType::new(0x14, "OS/2 hidden FAT16"),
    MBRPartitionType::new(0x15, "OS/2 hidden extended CHS"),
    MBRPartitionType::new(0x16, "OS/2 hidden FAT16B"),
    MBRPartitionType::new(0x17, "OS/2 Hidden IFS; HPFS; NTFS; exFAT"),
    MBRPartitionType::new(0x18, "AST SmartSleep"),
    MBRPartitionType::new(0x19, "Willowtech Photon coS"),
    MBRPartitionType::new(0x1a, "Unknown"),
    MBRPartitionType::new(0x1b, "OS/2 hidden FAT32"),
    MBRPartitionType::new(0x1c, "OS/2 hidden FAT32 LBA"),
    MBRPartitionType::new(0x1d, "Unknown"),
    MBRPartitionType::new(0x1e, "OS/2 hidden FAT16 LBA"),
    MBRPartitionType::new(0x1f, "OS/2 hidden extended LBA"),
    MBRPartitionType::new(0x20, "Windows Mobile update"),
    MBRPartitionType::new(0x21, "Oxygen FSo2"),
    MBRPartitionType::new(0x22, "Oxygen extended"),
    MBRPartitionType::new(0x23, "Windows Mobile boot"),
    MBRPartitionType::new(0x24, "NEC DOS"),
    MBRPartitionType::new(0x25, "Unknown"),
    MBRPartitionType::new(0x26, "Unknown"),
    MBRPartitionType::new(0x27, "WinRE (hidden NTFS)"),
    MBRPartitionType::new(0x28, "Unknown"),
    MBRPartitionType::new(0x29, "Unknown"),
    MBRPartitionType::new(0x2a, "AtheOS AthFS"),
    MBRPartitionType::new(0x2b, "Syllable Secure"),
    MBRPartitionType::new(0x2c, "Unknown"),
    MBRPartitionType::new(0x2d, "Unknown"),
    MBRPartitionType::new(0x2e, "Unknown"),
    MBRPartitionType::new(0x2f, "Unknown"),
    MBRPartitionType::new(0x30, "Unknown"),
    MBRPartitionType::new(0x31, "Unknown"),
    MBRPartitionType::new(0x32, "Unknown"),
    MBRPartitionType::new(0x33, "Unknown"),
    MBRPartitionType::new(0x34, "Unknown"),
    MBRPartitionType::new(0x35, "JFS"),
    MBRPartitionType::new(0x36, "Unknown"),
    MBRPartitionType::new(0x37, "Unknown"),
    MBRPartitionType::new(0x38, "THEOSv3.2"),
    MBRPartitionType::new(0x39, "Plan 9; THEOSv4 spanned"),
    MBRPartitionType::new(0x3a, "THEOSv4 4 GB"),
    MBRPartitionType::new(0x3b, "THEOSv4 extended"),
    MBRPartitionType::new(0x3c, "PartitionMagic recovery"),
    MBRPartitionType::new(0x3d, "PartitionMagic hidden NetWare"),
    MBRPartitionType::new(0x3e, "Unknown"),
    MBRPartitionType::new(0x3f, "Unknown"),
    MBRPartitionType::new(0x40, "Venix 80286; PICK R83"),
    MBRPartitionType::new(0x41, "Minix; PPC PReP Boot"),
    MBRPartitionType::new(0x42, "Secure File System; Old Linux swap"),
    MBRPartitionType::new(0x43, "Old Linux native"),
    MBRPartitionType::new(0x44, "Norton GoBack"),
    MBRPartitionType::new(0x45, "Boot-US boot manager; Priam; EUMEL-ELAN (L2)"),
    MBRPartitionType::new(0x46, "EUMEL-ELAN (L2)"),
    MBRPartitionType::new(0x47, "EUMEL-ELAN (L2)"),
    MBRPartitionType::new(0x48, "EUMEL-ELAN (L2); ERGOS (L3)"),
    MBRPartitionType::new(0x49, "Unknown"),
    MBRPartitionType::new(0x4a, "AdaOS Aquila; ALFS-THIN"),
    MBRPartitionType::new(0x4b, "Unknown"),
    MBRPartitionType::new(0x4c, "Oberon Aos (A2)"),
    MBRPartitionType::new(0x4d, "QNX4.x primary"),
    MBRPartitionType::new(0x4e, "QNX4.x secondary"),
    MBRPartitionType::new(0x4f, "QNX4.x tertiary; Oberon boot"),
    MBRPartitionType::new(0x50, "OnTrack Disk Manager; Oberon alternative; Lynx"),
    MBRPartitionType::new(0x51, "OnTrack Disk Manager aux 1; Novell"),
    MBRPartitionType::new(0x52, "CP/M-80; Microport SysV/AT"),
    MBRPartitionType::new(0x53, "OnTrack Disk Manager aux 3"),
    MBRPartitionType::new(0x54, "OnTrack Disk Manager dynamic driver overlay"),
    MBRPartitionType::new(0x55, "EZ-Drive"),
    MBRPartitionType::new(0x56, "AT&T DOS; EZ-Drive BIOS; Golden Bow VFeature"),
    MBRPartitionType::new(0x57, "DrivePro; Novell VNDI"),
    MBRPartitionType::new(0x58, "Unknown"),
    MBRPartitionType::new(0x59, "yocOS yocFS"),
    MBRPartitionType::new(0x5a, "Unknown"),
    MBRPartitionType::new(0x5b, "Unknown"),
    MBRPartitionType::new(0x5c, "Priam EDisk"),
    MBRPartitionType::new(0x5d, "Unknown"),
    MBRPartitionType::new(0x5e, "Unknown"),
    MBRPartitionType::new(0x5f, "Unknown"),
    MBRPartitionType::new(0x60, "Unknown"),
    MBRPartitionType::new(0x61, "SpeedStor hidden FAT12"),
    MBRPartitionType::new(0x62, "Unknown"),
    MBRPartitionType::new(0x63, "SysV; GNU HURD; SpeedStor hidden read-only FAT12"),
    MBRPartitionType::new(0x64, "Novell Netware 286; SpeedStor hidden FAT16; PC-ARMOUR"),
    MBRPartitionType::new(0x65, "Novell Netware 386"),
    MBRPartitionType::new(0x66, "Novell Netware Storage Management Services; Speedstor hidden read-only FAT16"),
    MBRPartitionType::new(0x67, "Novell Netware Wolf Mountain"),
    MBRPartitionType::new(0x68, "Novell Netware"),
    MBRPartitionType::new(0x69, "Novell Netware 5"),
    MBRPartitionType::new(0x6a, "Unknown"),
    MBRPartitionType::new(0x6b, "Unknown"),
    MBRPartitionType::new(0x6c, "DragonFly BSD slice"),
    MBRPartitionType::new(0x6d, "Unknown"),
    MBRPartitionType::new(0x6e, "Unknown"),
    MBRPartitionType::new(0x6f, "Unknown"),
    MBRPartitionType::new(0x70, "DiskSecure multiboot"),
    MBRPartitionType::new(0x71, "Unknown"),
    MBRPartitionType::new(0x72, "V7/x86; APTI alternative FAT12"),
    MBRPartitionType::new(0x73, "Unknown"),
    MBRPartitionType::new(0x74, "SpeedStor hidden FAT16B; V7/x86"),
    MBRPartitionType::new(0x75, "PC/IX"),
    MBRPartitionType::new(0x76, "SpeedStor hidden read-only FAT16B"),
    MBRPartitionType::new(0x77, "Novell VNDI/M2FS/M2CS"),
    MBRPartitionType::new(0x78, "XOSL bootloader"),
    MBRPartitionType::new(0x79, "APTI alternative FAT16"),
    MBRPartitionType::new(0x7a, "APTI alternative FAT16 LBA"),
    MBRPartitionType::new(0x7b, "APTI alternative FAT16B"),
    MBRPartitionType::new(0x7c, "APTI alternative FAT32 LBA"),
    MBRPartitionType::new(0x7d, "APTI alternative FAT32"),
    MBRPartitionType::new(0x7e, "PrimoCache L2"),
    MBRPartitionType::new(0x7f, "User-defined"),
    MBRPartitionType::new(0x80, "Minix old"),
    MBRPartitionType::new(0x81, "Minix; Linux old"),
    MBRPartitionType::new(0x82, "Linux swap; Solaris"),
    MBRPartitionType::new(0x83, "Linux"),
    MBRPartitionType::new(0x84, "OS/2 hidden; Intel hibernation"),
    MBRPartitionType::new(0x85, "Linux extended"),
    MBRPartitionType::new(0x86, "FAT16B volume set; Linux RAID"),
    MBRPartitionType::new(0x87, "NTFS volume set"),
    MBRPartitionType::new(0x88, "Linux plaintext"),
    MBRPartitionType::new(0x89, "Unknown"),
    MBRPartitionType::new(0x8a, "Unknown"),
    MBRPartitionType::new(0x8b, "Unknown"),
    MBRPartitionType::new(0x8c, "Unknown"),
    MBRPartitionType::new(0x8d, "Unknown"),
    MBRPartitionType::new(0x8e, "Linux LVM"),
    MBRPartitionType::new(0x8f, "Unknown"),
    MBRPartitionType::new(0x90, "FreeDOS hidden FAT16"),
    MBRPartitionType::new(0x91, "FreeDOS hidden extended"),
    MBRPartitionType::new(0x92, "FreeDOS hidden FAT16B"),
    MBRPartitionType::new(0x93, "Amoeba; Linux hidden"),
    MBRPartitionType::new(0x94, "Amoeba bad block table"),
    MBRPartitionType::new(0x95, "EXOPC native"),
    MBRPartitionType::new(0x96, "ISO-9660"),
    MBRPartitionType::new(0x97, "FreeDOS FAT32"),
    MBRPartitionType::new(0x98, "FreeDOS FAT32 LBA; Service"),
    MBRPartitionType::new(0x99, "Unknown"),
    MBRPartitionType::new(0x9a, "FreeDOS hidden FAT16B LBA"),
    MBRPartitionType::new(0x9b, "FreeDOS hidden extended LBA"),
    MBRPartitionType::new(0x9c, "Unknown"),
    MBRPartitionType::new(0x9d, "Unknown"),
    MBRPartitionType::new(0x9e, "ForthOS"),
    MBRPartitionType::new(0x9f, "BSD/OS"),
    MBRPartitionType::new(0xa0, "Hibernation"),
    MBRPartitionType::new(0xa1, "Hibernation"),
    MBRPartitionType::new(0xa2, "Hard Processor System preloader"),
    MBRPartitionType::new(0xa3, "HP Volume Expansion"),
    MBRPartitionType::new(0xa4, "HP Volume Expansion"),
    MBRPartitionType::new(0xa5, "FreeBSD slice"),
    MBRPartitionType::new(0xa6, "OpenBSD slice; HP Volume Expansion"),
    MBRPartitionType::new(0xa7, "NeXTSTEP"),
    MBRPartitionType::new(0xa8, "Darwin UFS"),
    MBRPartitionType::new(0xa9, "NetBSD slice"),
    MBRPartitionType::new(0xaa, "Olivetti FAT12"),
    MBRPartitionType::new(0xab, "Darwin boot"),
    MBRPartitionType::new(0xac, "Apple RAID"),
    MBRPartitionType::new(0xad, "RiscOS ADFS"),
    MBRPartitionType::new(0xae, "ShagOS"),
    MBRPartitionType::new(0xaf, "HFS; HFS+"),
    MBRPartitionType::new(0xb0, "BootStar dummy"),
    MBRPartitionType::new(0xb1, "QNX Neutrino; HP Volume Expansion"),
    MBRPartitionType::new(0xb2, "QNX Neutrino"),
    MBRPartitionType::new(0xb3, "QNX Neutrino; HP Volume Expansion"),
    MBRPartitionType::new(0xb4, "HP Volume Expansion"),
    MBRPartitionType::new(0xb5, "Unknown"),
    MBRPartitionType::new(0xb6, "FAT16B corrupted primary volume; HP Volume Expansion"),
    MBRPartitionType::new(0xb7, "BSDI; HPFS/NTFS corrupted primary volume"),
    MBRPartitionType::new(0xb8, "BSDI swap"),
    MBRPartitionType::new(0xb9, "Unknown"),
    MBRPartitionType::new(0xba, "Unknown"),
    MBRPartitionType::new(0xbb, "Boot Wizard hidden; Acronis OEM Secure Zone; FAT32 corrupted primary volume"),
    MBRPartitionType::new(0xbc, "Acronis Secure Zone; FAT32 LBA corrupted primary volume set; Paragon Backup Capsule"),
    MBRPartitionType::new(0xbd, "BonnyDOS/286"),
    MBRPartitionType::new(0xbe, "Solaris boot"),
    MBRPartitionType::new(0xbf, "Solaris"),
    MBRPartitionType::new(0xc0, "DR-DOS secured FAT <32 MB"),
    MBRPartitionType::new(0xc1, "DR-DOS secured FAT12"),
    MBRPartitionType::new(0xc2, "Power Boot hidden Linux"),
    MBRPartitionType::new(0xc3, "Power Boot hidden Linux swap"),
    MBRPartitionType::new(0xc4, "DR-DOS secured FAT16"),
    MBRPartitionType::new(0xc5, "DR-DOS secured extended"),
    MBRPartitionType::new(0xc6, "DR-DOS secured FAT16B; FAT16B corrupted secondary volume"),
    MBRPartitionType::new(0xc7, "Syrinx boot; HPFS/NTFS corrupted secondary volume"),
    MBRPartitionType::new(0xc8, "Unknown"),
    MBRPartitionType::new(0xc9, "Unknown"),
    MBRPartitionType::new(0xca, "Unknown"),
    MBRPartitionType::new(0xcb, "DR-DOS secured FAT32"),
    MBRPartitionType::new(0xcc, "DR-DOS secured FAT32 LBA; FAT32 corrupted secondary volume"),
    MBRPartitionType::new(0xcd, "CTOS memory dump"),
    MBRPartitionType::new(0xce, "DR-DOS secured FAT16B LBA"),
    MBRPartitionType::new(0xcf, "DR-DOS secured extended LBA"),
    MBRPartitionType::new(0xd0, "Multiuser DOS secured FAT >32 MB"),
    MBRPartitionType::new(0xd1, "Multiuser DOS secured FAT12"),
    MBRPartitionType::new(0xd2, "Unknown"),
    MBRPartitionType::new(0xd3, "Unknown"),
    MBRPartitionType::new(0xd4, "Multiuser DOS secured FAT16"),
    MBRPartitionType::new(0xd5, "Multiuser DOS secured extended CHS"),
    MBRPartitionType::new(0xd6, "Multiuser DOS secured FAT16B"),
    MBRPartitionType::new(0xd7, "Unknown"),
    MBRPartitionType::new(0xd8, "CP/M-86"),
    MBRPartitionType::new(0xd9, "Unknown"),
    MBRPartitionType::new(0xda, "Non-filesystem data"),
    MBRPartitionType::new(0xdb, "CP/M-86"),
    MBRPartitionType::new(0xdc, "Unknown"),
    MBRPartitionType::new(0xdd, "CTOS hidden memory dump"),
    MBRPartitionType::new(0xde, "Dell FAT16 diagnostic/utility"),
    MBRPartitionType::new(0xdf, "BootIt EMBRM; DG/UX"),
    MBRPartitionType::new(0xe0, "ST AVFS"),
    MBRPartitionType::new(0xe1, "SpeedStor FAT12 <16 MB"),
    MBRPartitionType::new(0xe2, "Unknown"),
    MBRPartitionType::new(0xe3, "SpeedStor read-only FAT12 <16 MB"),
    MBRPartitionType::new(0xe4, "SpeedStor FAT16 <32 MB"),
    MBRPartitionType::new(0xe5, "Tandy MS/DOS"),
    MBRPartitionType::new(0xe6, "SpeedStor read-only FAT16 <32 MB"),
    MBRPartitionType::new(0xe7, "Unknown"),
    MBRPartitionType::new(0xe8, "Linux Unified Key Setup (LUKS)"),
    MBRPartitionType::new(0xe9, "Unknown"),
    MBRPartitionType::new(0xea, "Linux extended boot"), // util-linux
    MBRPartitionType::new(0xeb, "BeOS/Haiku BFS"),
    MBRPartitionType::new(0xec, "SkyOS SkyFS"),
    MBRPartitionType::new(0xed, "Sprytix EDC loader"),
    MBRPartitionType::new(0xee, "GPT"),
    MBRPartitionType::new(0xef, "EFI system"),
    MBRPartitionType::new(0xf0, "Linux/PA-RISC boot"),
    MBRPartitionType::new(0xf1, "SpeedStor"), // util-linux
    MBRPartitionType::new(0xf2, "Unisys FAT12/FAT16 secondary"),
    MBRPartitionType::new(0xf3, "Unknown"),
    MBRPartitionType::new(0xf4, "SpeedStor FAT16; Prologue single-volume NGF/TwinFS"),
    MBRPartitionType::new(0xf5, "Prologue multi-volume NGF/TwinFS"),
    MBRPartitionType::new(0xf6, "SpeedStor read-only FAT16B"),
    MBRPartitionType::new(0xf7, "OSG EFAT; X1 solid state"),
    MBRPartitionType::new(0xf8, "Arm EBBR system firmware"),
    MBRPartitionType::new(0xf9, "Linux pCache EXT2/EXT3"),
    MBRPartitionType::new(0xfa, "Unknown"),
    MBRPartitionType::new(0xfb, "VMware VMFS"),
    MBRPartitionType::new(0xfc, "VMware swap/VMKCORE"),
    MBRPartitionType::new(0xfd, "Linux raid autodetect"),
    MBRPartitionType::new(0xfe, "PS/2 recovery; Old Linux LVM"),
    MBRPartitionType::new(0xff, "Xenix bad block table"),
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
