use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

/// Mapping `Anzahl gewonnener Hoenn-Orden` → `maximaler Level-Cap`.
///
/// In ORAS gibt es 8 Liga-Orden, also sind gültige Keys `0..=8`.
#[derive(Debug, Clone)]
pub struct CapTable {
    caps: BTreeMap<u8, u8>,
}

impl CapTable {
    /// Parst eine `caps.txt`-Datei vom Format:
    /// ```text
    /// # Kommentar
    /// 0=15
    /// 1=19
    /// ```
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("caps.txt nicht gefunden: {}", path.display()))?;
        Self::parse(&content)
    }

    pub fn parse(content: &str) -> Result<Self> {
        let mut caps = BTreeMap::new();

        for (i, raw) in content.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let (k, v) = line
                .split_once('=')
                .with_context(|| format!("Zeile {} ungültig (kein '='): {}", i + 1, line))?;

            let badges: u8 = k
                .trim()
                .parse()
                .with_context(|| format!("Zeile {}: Orden-Anzahl nicht parsebar: {}", i + 1, k))?;
            let cap: u8 = v
                .trim()
                .parse()
                .with_context(|| format!("Zeile {}: Level-Cap nicht parsebar: {}", i + 1, v))?;

            if badges > 8 {
                bail!(
                    "Zeile {}: Orden-Anzahl {} > 8 (ORAS hat nur 8 Hoenn-Orden)",
                    i + 1,
                    badges
                );
            }
            if cap == 0 || cap > 100 {
                bail!("Zeile {}: Level-Cap {} außerhalb 1..=100", i + 1, cap);
            }

            if caps.insert(badges, cap).is_some() {
                bail!("Zeile {}: Orden-Anzahl {} doppelt definiert", i + 1, badges);
            }
        }

        if !caps.contains_key(&0) {
            bail!("caps.txt muss mindestens '0=<cap>' enthalten");
        }

        Ok(Self { caps })
    }

    /// Liefert den Cap für die gegebene Orden-Anzahl. Wirft, wenn die Stufe in
    /// `caps.txt` fehlt — bewusst harte Semantik (kein impliziter Fallback).
    pub fn cap_for(&self, badges: u8) -> Result<u8> {
        self.caps.get(&badges).copied().with_context(|| {
            format!(
                "Kein Cap für {} Orden in caps.txt definiert. Definierte Stufen: {:?}",
                badges,
                self.caps.keys().collect::<Vec<_>>()
            )
        })
    }

    pub fn defined_badge_counts(&self) -> impl Iterator<Item = u8> + '_ {
        self.caps.keys().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_valid_file() {
        let table = CapTable::parse("0=15").unwrap();
        assert_eq!(table.cap_for(0).unwrap(), 15);
    }

    #[test]
    fn parses_full_default_caps() {
        let content = "\
# Kommentar
0=15
1=19
2=24
3=29
4=31
5=33
6=36
7=42
8=48
";
        let table = CapTable::parse(content).unwrap();
        assert_eq!(table.cap_for(0).unwrap(), 15);
        assert_eq!(table.cap_for(3).unwrap(), 29);
        assert_eq!(table.cap_for(8).unwrap(), 48);
    }

    #[test]
    fn ignores_blank_lines_and_comments() {
        let content = "\n\n# header\n0=10\n\n  # inline-ish\n1=20\n";
        let table = CapTable::parse(content).unwrap();
        assert_eq!(table.cap_for(0).unwrap(), 10);
        assert_eq!(table.cap_for(1).unwrap(), 20);
    }

    #[test]
    fn rejects_missing_zero_badge_entry() {
        let err = CapTable::parse("1=19").unwrap_err();
        assert!(err.to_string().contains("0=<cap>"));
    }

    #[test]
    fn rejects_badge_count_above_eight() {
        let err = CapTable::parse("0=10\n9=99").unwrap_err();
        assert!(err.to_string().contains("> 8"));
    }

    #[test]
    fn rejects_level_above_hundred() {
        let err = CapTable::parse("0=101").unwrap_err();
        assert!(err.to_string().contains("außerhalb"));
    }

    #[test]
    fn rejects_zero_level_cap() {
        let err = CapTable::parse("0=0").unwrap_err();
        assert!(err.to_string().contains("außerhalb"));
    }

    #[test]
    fn rejects_duplicate_badge_count() {
        let err = CapTable::parse("0=15\n0=20").unwrap_err();
        assert!(err.to_string().contains("doppelt"));
    }

    #[test]
    fn rejects_line_without_equals_sign() {
        let err = CapTable::parse("0=15\nblubb").unwrap_err();
        assert!(err.to_string().contains("kein '='"));
    }

    #[test]
    fn cap_for_unknown_badges_returns_error() {
        let table = CapTable::parse("0=15\n8=48").unwrap();
        let err = table.cap_for(3).unwrap_err();
        assert!(err.to_string().contains("3 Orden"));
    }
}
