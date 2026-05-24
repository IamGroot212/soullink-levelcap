//! End-to-end-Test gegen einen laufenden Citra-Prozess.
//!
//! Standardmäßig per `#[ignore]` deaktiviert — manuell laufen lassen mit:
//! ```bash
//! cargo test --test memory_layout_test -- --ignored --nocapture
//! ```
//!
//! Voraussetzungen:
//! - Citra läuft mit Pokémon Alpha Saphir geladen
//! - Memory-Offsets in `src/game/{badges,party}.rs` sind bereits aus R1-R4 verifiziert.

use soullink_levelcap::emulator::CitraProcess;
use soullink_levelcap::game::{read_badge_count, PartyPokemon, PARTY_SIZE};
use soullink_levelcap::memory::DefaultProcessMemory;

#[test]
#[ignore = "braucht laufenden Citra-Prozess mit ORAS"]
fn finds_citra_and_reads_plausible_party() {
    let citra = CitraProcess::find().expect("Citra läuft nicht");
    let mem = DefaultProcessMemory::open(citra.pid).expect("Citra-Memory nicht öffenbar");

    let badges = read_badge_count(&mem, &citra).expect("Badge-Byte nicht lesbar");
    assert!(badges <= 8, "Badge-Count {badges} > 8 — Offset falsch?");
    eprintln!("[test] Orden: {badges}");

    let mut any_alive = false;
    for slot in 0..PARTY_SIZE {
        if let Some(pkmn) = PartyPokemon::read(&mem, &citra, slot).expect("Party-Slot nicht lesbar")
        {
            any_alive = true;
            assert!(pkmn.species > 0 && pkmn.species < 722, "Species-ID {} außerhalb Gen1-6", pkmn.species);
            assert!(pkmn.level >= 1 && pkmn.level <= 100, "Level {} unplausibel", pkmn.level);
            eprintln!("[test] Slot {}: species={} lvl={} exp={}", slot, pkmn.species, pkmn.level, pkmn.exp);
        }
    }
    assert!(any_alive, "Party ist leer — Spieler hat noch kein Pokémon?");
}
