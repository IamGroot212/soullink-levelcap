//! Integration-Test gegen die mitgelieferte `caps.example.txt`.

use std::path::PathBuf;

use soullink_levelcap::caps::CapTable;

fn example_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("caps.example.txt")
}

#[test]
fn caps_example_loads_and_covers_all_badge_counts() {
    let table = CapTable::load(&example_path()).expect("caps.example.txt muss parsen");
    for badges in 0u8..=8 {
        let cap = table
            .cap_for(badges)
            .unwrap_or_else(|_| panic!("caps.example.txt deckt {badges} Orden nicht ab"));
        assert!((1..=100).contains(&cap), "Cap {cap} außerhalb 1..=100");
    }
}

#[test]
fn caps_example_is_monotonic() {
    // Die Default-Werte sollten mit steigender Orden-Anzahl nicht sinken.
    let table = CapTable::load(&example_path()).unwrap();
    let mut prev = 0u8;
    for badges in 0u8..=8 {
        let cap = table.cap_for(badges).unwrap();
        assert!(
            cap >= prev,
            "Cap fällt zwischen {} und {} Orden ({} → {})",
            badges - 1,
            badges,
            prev,
            cap
        );
        prev = cap;
    }
}
