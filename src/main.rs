use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use soullink_levelcap::{caps::CapTable, daemon};

#[derive(Parser)]
#[command(
    version,
    about = "Soullink Level-Cap Daemon für Pokemon Alpha Saphir auf Citra"
)]
struct Args {
    /// Pfad zu caps.txt
    #[arg(short, long, default_value = "caps.txt")]
    caps_file: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let caps = CapTable::load(&args.caps_file)?;
    println!(
        "[soullink-levelcap v{}] gestartet",
        env!("CARGO_PKG_VERSION")
    );
    println!("[INFO] Caps geladen aus: {}", args.caps_file.display());
    for badges in caps.defined_badge_counts() {
        if let Ok(cap) = caps.cap_for(badges) {
            println!("[INFO]   {} Orden → Lvl {}", badges, cap);
        }
    }
    daemon::run(caps)
}
