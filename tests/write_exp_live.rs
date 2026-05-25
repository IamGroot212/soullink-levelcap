//! Live-Test: PartyPokemon::write_exp ändert RAM und liest geänderten Wert zurück.
//! Stellt den Original-EXP nach dem Test wieder her.

use soullink_levelcap::emulator::CitraProcess;
use soullink_levelcap::game::{PartyPokemon, PARTY_SIZE};
use soullink_levelcap::memory::DefaultProcessMemory;

#[test]
#[ignore = "braucht laufenden Citra-Prozess mit ORAS"]
fn write_exp_roundtrip_in_ram() {
    let citra = CitraProcess::find().expect("Citra läuft nicht");
    let mem = DefaultProcessMemory::open(citra.pid).expect("memory open");

    // Suche ersten gültigen Party-Slot
    let mut target: Option<PartyPokemon> = None;
    for slot in 0..PARTY_SIZE {
        if let Some(p) = PartyPokemon::read(&mem, &citra, slot).unwrap() {
            target = Some(p);
            break;
        }
    }
    let original = target.expect("kein gültiges Party-Pokemon gefunden");
    eprintln!(
        "Ausgangs-State: Slot {} species={} lvl={} exp={}",
        original.slot, original.species, original.level, original.exp
    );

    // Schreibe einen offset-EXP (= original + 12345)
    let test_exp = original.exp + 12345;
    eprintln!("Schreibe EXP={}", test_exp);
    original.write_exp(&mem, &citra, test_exp).expect("write");

    // Read-back
    let after = PartyPokemon::read(&mem, &citra, original.slot)
        .expect("read")
        .expect("slot noch besetzt");
    eprintln!(
        "Nach Schreib: species={} lvl={} exp={}",
        after.species, after.level, after.exp
    );
    assert_eq!(
        after.species, original.species,
        "Species hat sich geändert!"
    );
    assert_eq!(after.exp, test_exp, "EXP wurde nicht korrekt geschrieben");

    // Restore
    eprintln!("Restore EXP={}", original.exp);
    original
        .write_exp(&mem, &citra, original.exp)
        .expect("restore");

    let restored = PartyPokemon::read(&mem, &citra, original.slot)
        .expect("read")
        .expect("slot noch besetzt");
    assert_eq!(restored.exp, original.exp, "Restore fehlgeschlagen");
    eprintln!("Restored: exp={}", restored.exp);
}
