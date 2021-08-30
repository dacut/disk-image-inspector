use phf::{phf_map, Map};
use std::{
    convert::TryInto,
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
    io::{Read, Seek, SeekFrom},
};
use uuid::Uuid;

use crate::errors::ImageError;

pub const GPT_HEADER_SIGNATURE: [u8; 8] = [0x45, 0x46, 0x49, 0x20, 0x50, 0x41, 0x52, 0x54];
pub const GPT_REVISION_1_0: u32 = 0x00010000;
pub const GPT_HEADER_1_0_SIZE: u32 = 92;
pub const MBR_GPT_PARTITION_TYPE: u8 = 0xee;

#[derive(Debug)]
pub struct GptHeader {
    pub signature: [u8; 8],
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved1: [u8; 4],
    pub current_lba: u64,
    pub backup_lba: u64,
    pub first_usable_lba: u64,
    pub last_usable_lba: u64,
    pub disk_guid: Uuid,
    pub partition_table_lba: u64,
    pub partition_count: u32,
    pub partition_entry_size: u32,
    pub partition_entry_array_crc32: u32,
}

impl GptHeader {
    pub fn new<R: Read + Seek>(reader: &mut R, offset: u64) -> Result<Self, Box<dyn Error>> {
        let mut header_bytes: [u8; GPT_HEADER_1_0_SIZE as usize] = [0; GPT_HEADER_1_0_SIZE as usize];

        reader.seek(SeekFrom::Start(offset))?;
        reader.read_exact(&mut header_bytes)?;

        let signature: [u8; 8] = header_bytes[0..8].try_into().unwrap();
        if signature != GPT_HEADER_SIGNATURE {
            return Err(ImageError::InvalidGptHeaderSignature(signature.to_vec()).into());
        }

        let revision = u32::from_le_bytes(header_bytes[8..12].try_into().unwrap());
        if revision < GPT_REVISION_1_0 {
            return Err(ImageError::InvalidGptHeaderRevision(revision).into());
        }

        let header_size = u32::from_le_bytes(header_bytes[12..16].try_into().unwrap());
        if header_size != GPT_HEADER_1_0_SIZE {
            return Err(ImageError::InvalidGptHeaderSize(header_size).into());
        }

        let crc32 = u32::from_le_bytes(header_bytes[16..20].try_into().unwrap());
        let reserved1 = header_bytes[20..24].try_into().unwrap();

        let current_lba = u64::from_le_bytes(header_bytes[24..32].try_into().unwrap());
        let backup_lba = u64::from_le_bytes(header_bytes[32..40].try_into().unwrap());
        let first_usable_lba = u64::from_le_bytes(header_bytes[40..48].try_into().unwrap());
        let last_usable_lba = u64::from_le_bytes(header_bytes[48..56].try_into().unwrap());
        let disk_guid = read_mixed_endian_uuid(header_bytes[56..72].try_into().unwrap());
        let partition_table_lba = u64::from_le_bytes(header_bytes[72..80].try_into().unwrap());
        let partition_count = u32::from_le_bytes(header_bytes[80..84].try_into().unwrap());
        let partition_entry_size = u32::from_le_bytes(header_bytes[84..88].try_into().unwrap());
        let partition_entry_array_crc32 = u32::from_le_bytes(header_bytes[88..92].try_into().unwrap());

        Ok(Self {
            signature,
            revision,
            header_size,
            crc32,
            reserved1,
            current_lba,
            backup_lba,
            first_usable_lba,
            last_usable_lba,
            disk_guid,
            partition_table_lba,
            partition_count,
            partition_entry_size,
            partition_entry_array_crc32,
        })
    }
}

impl Display for GptHeader {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "Signature: {}\nRevision: 0x{:04x}\nHeader size: {}\nCRC32: 0x{:04x}\nCurrent LBA: {}\nBackup LBA: {}\n\
             First usable LBA: {}\nLast usable LBA: {}\nDisk GUID: {}\nPartition table LBA: {}\nPartition count: {}\n\
             Partition entry size: {}\nPartition table CRC32: {:04x}",
            hex::encode(&self.signature),
            self.revision,
            self.header_size,
            self.crc32,
            self.current_lba,
            self.backup_lba,
            self.first_usable_lba,
            self.last_usable_lba,
            self.disk_guid,
            self.partition_table_lba,
            self.partition_count,
            self.partition_entry_size,
            self.partition_entry_array_crc32,
        )
    }
}

// See https://en.wikipedia.org/wiki/GUID_Partition_Table for partition types.
pub const GPT_PARTITION_TYPES: Map<u128, &'static str> = phf_map! {
    0x00000000000000000000000000000000u128 => "Empty",
    0x024dee4133e711d39d690008c781f39fu128 => "MBR partition scheme",
    0xc12a7328f81f11d2ba4b00a0c93ec93bu128 => "EFI system",
    0x2168614864496e6f744e656564454649u128 => "BIOS boot",
    0xd3bfe2de3daf11dfba40e3a556d89593u128 => "Intel Fast Flash",
    0xf4019732066e4e128273346c5641494fu128 => "Sony boot",
    0xbfbfafe7a34f448a9a5b6213eb736c22u128 => "Lenovo boot",
    0xe3c9e3160b5c4db8817df92df00215aeu128 => "Microsoft reserved",
    0xebd0a0a2b9e5443387c068b6b72699c7u128 => "Windows basic data",
    0x5808c8aa7e8f42e085d2e1e90434cfb3u128 => "Windows Logical Disk Manager metadata",
    0xaf9b60a014314f62bc683311714a69adu128 => "Windows Logical Disk Manager data",
    0xde94bba406d14d40a16abfd50179d6acu128 => "Windows recovery environment",
    0x37affc90ef7d4e9691c32d7ae055b174u128 => "IBM General Parallel File System",
    0xe75caf8ff6804ceeafa3b001e56efc2du128 => "Windows Storage Spaces",
    0x558d43c5a1ac43c0aac8d1472b2923d1u128 => "Windows Storage replica",
    0x75894c1e3aeb11d3b7c17b03a0000000u128 => "HP-UX data",
    0xe2a1e72832e311d6a6827b03a0000000u128 => "HP-UX service",
    0x0fc63daf848347728e793d69d8477de4u128 => "Linux data",
    0xa19d880f05fc4d3ba006743f0f84911eu128 => "Linux RAID",
    0x44479540f29741b29af7d131d5f0458au128 => "Linux root (x86)",
    0x4f68bce3e8cd4db196e7fbcaf984b709u128 => "Linux root (x86-64)",
    0x69dad7102ce44e3cb16c21a1d49abed3u128 => "Linux root (ARM 32)",
    0xb921b0451df041c3af444c6f280d3faeu128 => "Linux root (ARM 64)",
    0xbc13c2ff59e64262a352b275fd6f7172u128 => "Linux /boot",
    0x0657fd6da4ab43c484e50933c84b4f4fu128 => "Linux swap",
    0xe6d6d379f50744c2a23c238f2a3df928u128 => "Linux LVM",
    0x933ac7e12eb44f13b8440e14e2aef915u128 => "Linux /home",
    0x3b8f842520e04f3b907f1a25a76f98e8u128 => "Linux /srv",
    0x7ffec5c92d0049b789413ea10a5586b7u128 => "Linux dm-crypt",
    0xca7d7ccb63ed4c53861c1742536059ccu128 => "Linux LUKS",
    0x8da63339000760c0c436083ac8230908u128 => "Linux reserved",
    0x83bd6b9d7f4111dcbe0b001560b84f0fu128 => "FreeBSD boot",
    0x516e7cb46ecf11d68ff800022d09712bu128 => "FreeBSD disklabel",
    0x516e7cb56ecf11d68ff800022d09712bu128 => "FreeBSD swap",
    0x516e7cb66ecf11d68ff800022d09712bu128 => "FreeBSD UFS",
    0x516e7cb86ecf11d68ff800022d09712bu128 => "FreeBSD Vinum volume manager",
    0x516e7cba6ecf11d68ff800022d09712bu128 => "FreeBSD ZFS",
    0x74ba7dd9a68911e1bd0400e081286acfu128 => "FreeBSD nandfs",
    0x48465300000011aaaa1100306543ecacu128 => "Apple HFS+",
    0x7c3457ef000011aaaa1100306543ecacu128 => "Apple APFS container/APFS FileVault container",
    0x55465300000011aaaa1100306543ecacu128 => "Apple UFS container",
    0x52414944000011aaaa1100306543ecacu128 => "Apple RAID",
    0x524149445f4f11aaaa1100306543ecacu128 => "Apple RAID (offline)",
    0x426f6f74000011aaaa1100306543ecacu128 => "Apple recovery",
    0x4c6162656c0011aaaa1100306543ecacu128 => "Apple label",
    0x5265636f766511aaaa1100306543ecacu128 => "AppleTV recovery",
    0x53746f72616711aaaa1100306543ecacu128 => "Apple Core Storage container/HFS+ FileVault container",
    0x6a82cb451dd211b299a6080020736631u128 => "Solaris boot",
    0x6a85cf4d1dd211b299a6080020736631u128 => "Solaris root",
    0x6a87c46f1dd211b299a6080020736631u128 => "Solaris swap",
    0x6a8b642b1dd211b299a6080020736631u128 => "Solaris backup",
    0x6a898cc31dd211b299a6080020736631u128 => "Solaris /usr",
    0x6a8ef2e91dd211b299a6080020736631u128 => "Solaris /var",
    0x6a90ba391dd211b299a6080020736631u128 => "Solaris /home",
    0x6a9283a51dd211b299a6080020736631u128 => "Solaris alternate sector",
    0x6a945a3b1dd211b299a6080020736631u128 => "Solaris reserved",
    0x6a9630d11dd211b299a6080020736631u128 => "Solaris reserved",
    0x6a9807671dd211b299a6080020736631u128 => "Solaris reserved",
    0x6a96237f1dd211b299a6080020736631u128 => "Solaris reserved",
    0x6a8d2ac71dd211b299a6080020736631u128 => "Solaris reserved",
    0x49f48d32b10e11dcb99b0019d1879648u128 => "NetBSD swap",
    0x49f48d5ab10e11dcb99b0019d1879648u128 => "NetBSD FFS",
    0x49f48d82b10e11dcb99b0019d1879648u128 => "NetBSD LFS",
    0x49f48daab10e11dcb99b0019d1879648u128 => "NetBSD RAID",
    0x2db519c4b10f11dcb99b0019d1879648u128 => "NetBSD concatenated",
    0x2db519ecb10f11dcb99b0019d1879648u128 => "NetBSD encrypted",
    0xfe3a2a5d4f3241a7b725accc3285a309u128 => "ChromeOS kernel",
    0x3cb8e2023b7e47dd8a3c7ff2a13cfcecu128 => "ChromeOS root",
    0x2e0a753d9e4843b08337b15192cb1b5eu128 => "ChromeOS reserved",
    0x5dfbf5f428484bacaa5e0d9a20b745a6u128 => "CoreOS /usr",
    0x3884dd4185824404b9a8e9b84f2df50eu128 => "CoreOS resizeable root",
    0xc95dc21adf0e43408d7b26cbfa9a03e0u128 => "CoreOS reserved",
    0xbe9067b9ea494f15b4f6f36f8c9e1818u128 => "CoreOS root RAID",
    0x424653313ba310f1802a4861696b7521u128 => "Haiku BFS",
    0x85d5e45e237c11e1b4b3e89a8f7fc3a7u128 => "MidnightBSD boot",
    0x85d5e45a237c11e1b4b3e89a8f7fc3a7u128 => "MidnightBSD data",
    0x85d5e45b237c11e1b4b3e89a8f7fc3a7u128 => "MidnightBSD swap",
    0x0394ef8b237e11e1b4b3e89a8f7fc3a7u128 => "MidnightBSD UFS",
    0x85d5e45c237c11e1b4b3e89a8f7fc3a7u128 => "MidnightBSD Vinum volume manager",
    0x85d5e45d237c11e1b4b3e89a8f7fc3a7u128 => "MidnightBSD ZFS",
    0x45b0969e9b034f30b4c6b4b80ceff106u128 => "Ceph journal",
    0x45b0969e9b034f30b4c65ec00ceff106u128 => "Ceph dm-crypt journal",
    0x4fbd7e299d2541b8afd0062c0ceff05du128 => "Ceph OSD",
    0x4fbd7e299d2541b8afd05ec00ceff05du128 => "Ceph dm-crypt OSD",
    0x89c57f982fe54dc089c1f3ad0ceff2beu128 => "Ceph disk in creation",
    0x89c57f982fe54dc089c15ec00ceff2beu128 => "Ceph dm-crypt disk in creation",
    0xcafecafe9b034f30b4c6b4b80ceff106u128 => "Ceph block",
    0x30cd0809c2b2499c88792d6b78529876u128 => "Ceph block DB",
    0x5ce17fce40874169b7ff056cc58473f9u128 => "Ceph block write-ahead log",
    0xfb3aabf9d25f47ccbf5e721d1816496bu128 => "Ceph dm-crypt lockbox",
    0x4fbd7e298ae04982bf9d5a8d867af560u128 => "Ceph multipath OSD",
    0x45b0969e8ae04982bf9d5a8d867af560u128 => "Ceph multipath journal",
    0xcafecafe8ae04982bf9d5a8d867af560u128 => "Ceph multipath block",
    0x7f4a666a16f347a28445152ef4d03f6cu128 => "Ceph multipath block",
    0xec6d6385e34645dcbe91da2a7c8b3261u128 => "Ceph multipath block DB",
    0x01b41e1b002a453c9f1788793989ff8fu128 => "Ceph multipath block write-ahead log",
    0xcafecafe9b034f30b4c65ec00ceff106u128 => "Ceph dm-crypt block",
    0x93b0052d02d94d8aa43b33a3ee4dfbc3u128 => "Ceph dm-crypt block DB",
    0x306e86834fe24330b7c000a917c16966u128 => "Ceph dm-crypt block write-ahead log",
    0x45b0969e9b034f30b4c635865ceff106u128 => "Ceph dm-crypt LUKS journal",
    0xcafecafe9b034f30b4c635865ceff106u128 => "Ceph dm-crypt LUKS block",
    0x166418dac4694022adf4b30afd37f176u128 => "Ceph dm-crypt LUKS block DB",
    0x86a32090364740b9bbbd38d8c573aa86u128 => "Ceph dm-crypt LUKS block write-ahead log",
    0x4fbd7e299d2541b8afd035865ceff05du128 => "Ceph dm-crypt LUKS OSD",
    0x824cc7a036a811e3890a952519ad3f61u128 => "OpenBSD data",
    0xcef5a9ad73bc460189f3cdeeeee321a1u128 => "QNX Power-Safe",
    0xc91818f9802547af89d2f030d7000c2cu128 => "Plan9",
    0x9d27538040ad11dbbf97000c2911d1b8u128 => "VMware ESX vmkcore",
    0xaa31e02a400f11db9590000c2911d1b8u128 => "VMware ESX VMFS",
    0x9198effc31c011db8f78000c2911d1b8u128 => "VMware ESX reserved",
    0x2568845d23324675bc398fa5a4748d15u128 => "Android x86 bootloader",
    0x114eaffe15524022b26e9b053604cf84u128 => "Android x86 bootloader2",
    0x49a4d17f93a345c1a0def50b2ebe2599u128 => "Android x86 boot",
    0x4177c7229e924aab864443502bfd5506u128 => "Android x86 recovery",
    0xef32a33ba409486c91419ffb711f6266u128 => "Android x86 misc",
    0x20ac26be20b711e384c56cfdb94711e9u128 => "Android x86 metadata",
    0x38f428e6d326425d91406e0ea133647cu128 => "Android x86 system",
    0xa893ef21e428470a9e550668fd91a2d9u128 => "Android x86 cache",
    0xdc76dda95ac1491caf42a82591580c0du128 => "Android x86 data",
    0xebc597d020534b158b64e0aac75f4db1u128 => "Android x86 persistent",
    0xc5a0aeec13ea11e5a1b1001e67ca0c3cu128 => "Android x86 vendor",
    0xbd59408b4514490dbf129878d963f378u128 => "Android x86 config",
    0x8f68cc74c5e548dabe91a0c8c15e9c80u128 => "Android x86 factory",
    0x9fdaa6ef4b3f40d2ba8dbff16bfb887bu128 => "Android x86 factory alt",
    0x767941d0208511e3ad3b6cfdb94711e9u128 => "Android x86 fastboot/tertiary",
    0xac6d7924eb714df8b48de267b27148ffu128 => "Android x86 OEM",
    0x19a710a2b3ca11e4b02610604b889dcfu128 => "Android ARM metadata",
    0x193d1ea4b3ca11e4b07510604b889dcfu128 => "Android ARM EXT",
    0x7412f7d5a1564b1381dc867174929325u128 => "Open Network Install Environment boot",
    0xd4e6e2cd446946f3b5cb1bff57afc149u128 => "Open Network Install Environment config",
    0x9e1a2d38c6124316aa268b49521e5a8bu128 => "PowerPC PReP boot",
    0x734e5afef61a11e6bc6492361f002671u128 => "Atari TOS data",
    0x8c8f8effac954770814a21994f2dbc8fu128 => "VeraCrypt",
    0x90b6ff38b98f4358a21f48f35b4a8ad3u128 => "ArcaOS type 1",
    0x7c5222bd8f5d40879c00bf9843c7b58cu128 => "Storage Performance Development Kit block",
    0x4778ed65bf4245fa9c5b287a1dc4aab1u128 => "Barebox state",
    0xb6fa30da92d24a9a96f1871ec6486200u128 => "SoftRAID status",
    0x2e31346519b9463f81268a7993773801u128 => "SoftRAID scratch",
    0xfa709c7e65b14593bfd5e71d61de9b02u128 => "SoftRAID volume",
    0xbbba6df5f46f4a898f598765b2727503u128 => "SoftRAID cache",
};

pub struct GptPartitionEntry {
    pub partition_type: Uuid,
    pub unique_partition_guid: Uuid,
    pub starting_lba: u64,
    pub ending_lba: u64,
    pub attributes: u64,
    pub name: [u8; 72],
}

fn read_mixed_endian_uuid(data: &[u8]) -> Uuid {
    let part1 = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let part2 = u16::from_le_bytes(data[4..6].try_into().unwrap());
    let part3 = u16::from_le_bytes(data[6..8].try_into().unwrap());
    let part4 = &data[8..16];

    Uuid::from_fields(part1, part2, part3, part4).unwrap()
}

impl GptPartitionEntry {
    pub fn new<R: Read + Seek>(reader: &mut R, offset: u64) -> Result<Self, Box<dyn Error>> {
        reader.seek(SeekFrom::Start(offset))?;
        let mut partition_entry_bytes: [u8; 128] = [0; 128];
        reader.read_exact(&mut partition_entry_bytes)?;

        let partition_type = read_mixed_endian_uuid(&partition_entry_bytes[0..16]);
        let unique_partition_guid = read_mixed_endian_uuid(&partition_entry_bytes[16..32]);
        let starting_lba = u64::from_le_bytes(partition_entry_bytes[32..40].try_into().unwrap());
        let ending_lba = u64::from_le_bytes(partition_entry_bytes[40..48].try_into().unwrap());
        let attributes = u64::from_le_bytes(partition_entry_bytes[48..56].try_into().unwrap());
        let name = partition_entry_bytes[56..128].try_into().unwrap();

        Ok(Self {
            partition_type,
            unique_partition_guid,
            starting_lba,
            ending_lba,
            attributes,
            name,
        })
    }
}

impl Display for GptPartitionEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let partition_type_name = GPT_PARTITION_TYPES.get(&self.partition_type.as_u128()).unwrap_or(&"Unknown");
        write!(
            f,
            "Partition Type: {} ({})\nPartition GUID: {}\nStarting LBA: {}\nEnding LBA: {}\nAttributes: {}\nName: {}",
            self.partition_type,
            partition_type_name,
            self.unique_partition_guid,
            self.starting_lba,
            self.ending_lba,
            self.attributes,
            String::from_utf8_lossy(&self.name),
        )
    }
}
