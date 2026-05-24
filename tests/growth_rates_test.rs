//! Integration-Tests gegen Bulbapedia-Referenzwerte für `min_exp_for_level`.

use soullink_levelcap::game::growth_rates::{min_exp_for_level, GrowthRate};

#[test]
fn lvl_1_is_zero_for_all_rates() {
    for rate in [
        GrowthRate::Erratic,
        GrowthRate::Fast,
        GrowthRate::MediumFast,
        GrowthRate::MediumSlow,
        GrowthRate::Slow,
        GrowthRate::Fluctuating,
    ] {
        assert_eq!(min_exp_for_level(1, rate), 0, "rate {:?}", rate);
    }
}

/// Quelle: https://bulbapedia.bulbagarden.net/wiki/Experience#Relation_to_level
#[test]
fn lvl_100_matches_canonical_totals() {
    assert_eq!(min_exp_for_level(100, GrowthRate::Erratic), 600_000);
    assert_eq!(min_exp_for_level(100, GrowthRate::Fast), 800_000);
    assert_eq!(min_exp_for_level(100, GrowthRate::MediumFast), 1_000_000);
    assert_eq!(min_exp_for_level(100, GrowthRate::MediumSlow), 1_059_860);
    assert_eq!(min_exp_for_level(100, GrowthRate::Slow), 1_250_000);
    assert_eq!(min_exp_for_level(100, GrowthRate::Fluctuating), 1_640_000);
}

#[test]
fn medium_fast_known_values() {
    assert_eq!(min_exp_for_level(2, GrowthRate::MediumFast), 8);
    assert_eq!(min_exp_for_level(10, GrowthRate::MediumFast), 1000);
    assert_eq!(min_exp_for_level(50, GrowthRate::MediumFast), 125_000);
}
