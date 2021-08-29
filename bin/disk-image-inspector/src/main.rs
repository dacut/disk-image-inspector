use env_logger;
use getopts::Options;
use std::{
    env,
    error::Error,
    fs::File,
    io::{stderr, stdout, BufReader, Read, Write},
    process::exit,
};

mod bootsector;
use bootsector::{BootSector, BOOT_SECTOR_SIGNATURE, BOOT_SECTOR_SIZE};
mod errors;
use errors::ImageError;

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
    let image_unbuffered = match File::open(image_filename) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Unable to open {} for reading: {}", image_filename, e);
            return Err(e.into());
        }
    };

    let mut image = BufReader::new(image_unbuffered);
    let mut mbr = [0u8; BOOT_SECTOR_SIZE];

    match image.read_exact(&mut mbr) {
        Err(e) => {
            eprintln!("Failed to read master boot record ({} bytes) from {}: {}", BOOT_SECTOR_SIZE, image_filename, e);
            return Err(e.into());
        }
        Ok(()) => (),
    }

    let boot_sector = BootSector::new(&mbr);
    if &boot_sector.signature != BOOT_SECTOR_SIGNATURE {
        eprintln!(
            "Image does not start with a boot sector: expected [0x{:02x}, 0x{:02x}], got [0x{:02x}, 0x{:02x}]",
            BOOT_SECTOR_SIGNATURE[0], BOOT_SECTOR_SIGNATURE[1], boot_sector.signature[0], boot_sector.signature[1],
        );
        return Err(ImageError::InvalidSignature.into());
    }

    for (i, ref partition) in boot_sector.partitions.iter().enumerate() {
        println!("Partition {}:\n    {}", i + 1, format!("{}", partition).replace("\n", "\n    "));
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
