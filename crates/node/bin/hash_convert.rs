use iroh_blobs::Hash;
use std::str::FromStr;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <hash>", args[0]);
        std::process::exit(1);
    }

    let hash_str = &args[1];
    
    // Try to parse the hash from either hex or base32
    let hash = match Hash::from_str(hash_str) {
        Ok(h) => h,
        Err(_) => {
            eprintln!("Error: Invalid hash format. Please provide a valid hex or base32 hash.");
            std::process::exit(1);
        }
    };

    // Print both representations
    println!("Hash representations:");
    println!("  Base32-NoPadding:    {}", hash.to_string());
    println!("  Hex: {}", hash.to_hex());
} 