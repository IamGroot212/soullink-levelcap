use anyhow::{bail, Result};

/// Die 6 Pokémon-EXP-Wachstumsraten.
/// Quelle: https://bulbapedia.bulbagarden.net/wiki/Experience
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrowthRate {
    Erratic,
    Fast,
    MediumFast,
    MediumSlow,
    Slow,
    Fluctuating,
}

impl GrowthRate {
    /// Mapped die PokéAPI-Bezeichnungen (engl. plain) auf unsere Varianten.
    /// PokéAPI nennt Erratic "slow-then-very-fast" und Fluctuating "fast-then-very-slow";
    /// "erratic"/"fluctuating" akzeptieren wir zusätzlich für manuell editierte Daten.
    pub fn from_pokeapi_slug(slug: &str) -> Result<Self> {
        Ok(match slug {
            "slow-then-very-fast" | "erratic" => Self::Erratic,
            "fast" => Self::Fast,
            "medium" | "medium-fast" => Self::MediumFast,
            "medium-slow" => Self::MediumSlow,
            "slow" => Self::Slow,
            "fast-then-very-slow" | "fluctuating" => Self::Fluctuating,
            other => bail!("Unbekannte Wachstumsrate aus PokéAPI: {}", other),
        })
    }
}

/// Berechnet die minimale Gesamt-EXP, die für genau diesen Level nötig ist.
/// Formeln siehe Bulbapedia. `level == 1` ⇒ 0 EXP.
pub fn min_exp_for_level(level: u8, rate: GrowthRate) -> u32 {
    if level <= 1 {
        return 0;
    }
    let n = level as i64;
    let exp: i64 = match rate {
        GrowthRate::Fast => 4 * n.pow(3) / 5,
        GrowthRate::MediumFast => n.pow(3),
        GrowthRate::MediumSlow => (6 * n.pow(3)) / 5 - 15 * n.pow(2) + 100 * n - 140,
        GrowthRate::Slow => 5 * n.pow(3) / 4,
        GrowthRate::Erratic => {
            if n <= 50 {
                (n.pow(3) * (100 - n)) / 50
            } else if n <= 68 {
                (n.pow(3) * (150 - n)) / 100
            } else if n <= 98 {
                let inner = (1911 - 10 * n) / 3;
                (n.pow(3) * inner) / 500
            } else {
                (n.pow(3) * (160 - n)) / 100
            }
        }
        GrowthRate::Fluctuating => {
            if n <= 15 {
                let inner = (n + 1) / 3 + 24;
                n.pow(3) * inner / 50
            } else if n <= 36 {
                let inner = n + 14;
                n.pow(3) * inner / 50
            } else {
                let inner = n / 2 + 32;
                n.pow(3) * inner / 50
            }
        }
    };
    exp.max(0) as u32
}

/// Holt die Wachstumsrate für die gegebene Species-ID. Fällt auf MediumFast zurück, falls
/// die ID nicht in der generierten Tabelle ist (sicherer Default für die häufigste Gen-6-Rate).
///
/// TODO(R5): `data/species_growth.json` ist aktuell nur ein Stub mit Beispieldaten.
/// Vor v0.1.0 vollständig aus PokéAPI generieren — siehe scripts/fetch_growth_rates.py.
pub fn growth_rate_of(species: u16) -> GrowthRate {
    crate::species_data::SPECIES_GROWTH
        .get(&species)
        .copied()
        .unwrap_or(GrowthRate::MediumFast)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verifikation gegen bekannte Werte aus Bulbapedia.
    // https://bulbapedia.bulbagarden.net/wiki/Experience#Relation_to_level

    #[test]
    fn medium_fast_lvl_1_is_zero() {
        assert_eq!(min_exp_for_level(1, GrowthRate::MediumFast), 0);
    }

    #[test]
    fn medium_fast_matches_table() {
        // L^3
        assert_eq!(min_exp_for_level(50, GrowthRate::MediumFast), 125_000);
        assert_eq!(min_exp_for_level(100, GrowthRate::MediumFast), 1_000_000);
    }

    #[test]
    fn fast_matches_table() {
        // 4/5 * L^3 — bei L=100: 800_000
        assert_eq!(min_exp_for_level(100, GrowthRate::Fast), 800_000);
    }

    #[test]
    fn slow_matches_table() {
        // 5/4 * L^3 — bei L=100: 1_250_000
        assert_eq!(min_exp_for_level(100, GrowthRate::Slow), 1_250_000);
    }

    #[test]
    fn medium_slow_matches_table() {
        // bei L=100: 1_059_860
        assert_eq!(min_exp_for_level(100, GrowthRate::MediumSlow), 1_059_860);
    }

    #[test]
    fn erratic_matches_table_at_100() {
        // bei L=100: 600_000
        assert_eq!(min_exp_for_level(100, GrowthRate::Erratic), 600_000);
    }

    #[test]
    fn fluctuating_matches_table_at_100() {
        // bei L=100: 1_640_000
        assert_eq!(min_exp_for_level(100, GrowthRate::Fluctuating), 1_640_000);
    }

    #[test]
    fn slug_parser_accepts_pokeapi_variants() {
        assert_eq!(
            GrowthRate::from_pokeapi_slug("medium-fast").unwrap(),
            GrowthRate::MediumFast
        );
        assert_eq!(
            GrowthRate::from_pokeapi_slug("fluctuating").unwrap(),
            GrowthRate::Fluctuating
        );
        assert!(GrowthRate::from_pokeapi_slug("bogus").is_err());
    }
}
