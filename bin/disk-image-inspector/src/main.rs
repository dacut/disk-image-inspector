use env_logger;
use getopts::Options;
use std::{
    env,
    error::Error,
    fs::File,
    io::{stderr, stdout, Read, Seek, Write},
    process::exit,
};

mod bootsector;
use bootsector::{BootSector, BOOT_SECTOR_SIGNATURE, BOOT_SECTOR_SIZE};
mod errors;
use errors::ImageError;
mod gpt;
use gpt::{GptHeader, GptPartitionEntry, MBR_GPT_PARTITION_TYPE};

fn main() {
    env_logger::init();
    let args: Vec<String> = env::args().collect();
    let program: String = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("h", "help", "show this usage information");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}", e);
            print_usage(&program, &opts, &mut stderr());
            exit(2);
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, &opts, &mut stdout());
        exit(0);
    }

    if matches.free.len() == 0 {
        eprintln!("Error: image-filename not specified");
        print_usage(&program, &opts, &mut stderr());
        exit(2);
    } else if matches.free.len() > 1 {
        eprintln!("Error: unknown argument {}", matches.free[2]);
        print_usage(&program, &opts, &mut stderr());
        exit(2);
    }

    let image_filename = matches.free[0].clone();

    match run(&image_filename) {
        Ok(()) => (),
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    }
}

fn run(image_filename: &str) -> Result<(), Box<dyn Error>> {
    let mut image = match File::open(image_filename) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Unable to open {} for reading: {}", image_filename, e);
            return Err(e.into());
        }
    };

    let boot_sector = match BootSector::new(&mut image, 0) {
        Err(e) => {
            eprintln!("Failed to read master boot record ({} bytes) from {}: {}", BOOT_SECTOR_SIZE, image_filename, e);
            return Err(e.into());
        }
        Ok(bs) => bs,
    };

    if &boot_sector.signature != BOOT_SECTOR_SIGNATURE {
        eprintln!(
            "Image does not start with a boot sector: expected [0x{:02x}, 0x{:02x}], got [0x{:02x}, 0x{:02x}]",
            BOOT_SECTOR_SIGNATURE[0], BOOT_SECTOR_SIGNATURE[1], boot_sector.signature[0], boot_sector.signature[1],
        );
        return Err(ImageError::InvalidSignature.into());
    }

    if let Err(e) = print_mbr_partition_table(&mut image, &boot_sector, 0) {
        eprintln!("Failed to get partition table: {}", e);
        return Err(e.into());
    }

    let gpt_partition = &boot_sector.partitions[0];
    if gpt_partition.partition_type.code == MBR_GPT_PARTITION_TYPE {
        if let Err(e) = print_gpt_partition_table(&mut image, gpt_partition.lba_start as u64 * 512) {
            eprintln!("Failed to get GPT partition table: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

fn print_mbr_partition_table<R: Read + Seek>(
    reader: &mut R,
    boot_sector: &BootSector,
    start_pos: u64,
) -> Result<(), Box<dyn Error>> {
    for (i, ref partition) in boot_sector.partitions.iter().enumerate() {
        if partition.partition_type.code > 0 {
            println!("MBR Partition {}:\n    {}", i + 1, format!("{}", partition).replace("\n", "\n    "));
        }
    }

    for partition in boot_sector.partitions.iter() {
        if partition.is_extended() {
            let (new_boot_sector, new_start_pos) = partition.get_extended_boot_sector(reader, start_pos)?;
            print_mbr_partition_table(reader, &new_boot_sector, new_start_pos)?;
        }
    }

    Ok(())
}

fn print_gpt_partition_table<R: Read + Seek>(reader: &mut R, header_pos: u64) -> Result<(), Box<dyn Error>> {
    let gpt_header = GptHeader::new(reader, header_pos)?;
    let gpt_entry_table_pos = gpt_header.partition_table_lba as u64 * 512;

    println!("GPT header:\n    {}", gpt_header.to_string().replace("\n", "\n    "));

    for i in 0..gpt_header.partition_count {
        let partition =
            GptPartitionEntry::new(reader, gpt_entry_table_pos + gpt_header.partition_entry_size as u64 * i as u64)?;

        if partition.partition_type.as_u128() != 0u128 {
            println!("GPT Partition {}:\n    {}", i + 1, format!("{}", partition).replace("\n", "\n    "));
        }
    }

    Ok(())
}

fn print_usage<W: Write>(program: &str, opts: &Options, writer: &mut W) {
    let brief = format!(
        "Inspect a disk image and show boot sector and partition information.\n\
    Usage: {} [options] <image-filename>",
        program
    );
    let _ = write!(writer, "{}", opts.usage(&brief));
}
