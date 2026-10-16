//! Versão semântica mínima (port de `SemVer.cs`): 0.1.0, 0.1.0-alpha.5 (pré-release vem antes da final).

use std::cmp::Ordering;
use std::fmt;
use std::sync::OnceLock;

use regex::Regex;

#[derive(Debug, Clone)]
pub struct SemVer {
    core: [u32; 3],
    pre: Vec<String>,
}

impl SemVer {
    pub fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }

    /// Como `find`, mas sem versão no texto vira 0.0.0.
    pub fn parse(text: &str) -> SemVer {
        SemVer::find(text).unwrap_or(SemVer {
            core: [0, 0, 0],
            pre: Vec::new(),
        })
    }

    /// Primeira versão encontrada em qualquer ponto do texto ("v0.1.0-alpha.5", "Tako 0.1.0").
    pub fn find(text: &str) -> Option<SemVer> {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| {
            Regex::new(r"([0-9]+)\.([0-9]+)(?:\.([0-9]+))?(?:-([0-9A-Za-z.\-]+))?")
                .unwrap_or_else(|e| panic!("regex fixa: {e}"))
        });
        let c = re.captures(text)?;
        let num = |i: usize| c.get(i).map_or(Some(0), |m| m.as_str().parse().ok());
        Some(SemVer {
            core: [num(1)?, num(2)?, num(3)?],
            pre: c
                .get(4)
                .map(|m| m.as_str().split('.').map(str::to_owned).collect())
                .unwrap_or_default(),
        })
    }

    /// Versão deste build (a do workspace).
    pub fn current() -> SemVer {
        SemVer::parse(env!("CARGO_PKG_VERSION"))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        self.core.cmp(&other.core).then_with(|| {
            if self.pre.is_empty() || other.pre.is_empty() {
                // Sem pré-release é maior.
                return other.pre.len().cmp(&self.pre.len());
            }
            for (a, b) in self.pre.iter().zip(&other.pre) {
                let c = match (numeric(a), numeric(b)) {
                    (Some(x), Some(y)) => x.cmp(&y),
                    _ => a.as_str().cmp(b.as_str()),
                };
                if c != Ordering::Equal {
                    return c;
                }
            }
            self.pre.len().cmp(&other.pre.len())
        })
    }
}

/// Igual ao `int.TryParse(NumberStyles.None)`: só dígitos, dentro de 32 bits.
fn numeric(s: &str) -> Option<i32> {
    if s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse().ok()
    } else {
        None
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for SemVer {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for SemVer {}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c] = self.core;
        write!(f, "{a}.{b}.{c}")?;
        if !self.pre.is_empty() {
            write!(f, "-{}", self.pre.join("."))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        for (a, b, expected) in [
            ("0.1.0-alpha.5", "0.1.0-alpha.10", Ordering::Less),
            ("0.1.0-alpha.5", "0.1.0", Ordering::Less),
            ("0.1.0", "0.1.1-beta.1", Ordering::Less),
            ("1.2.0", "1.10.0", Ordering::Less),
            ("v0.1.0-alpha.5", "Tako 0.1.0-alpha.5", Ordering::Equal),
            ("0.1.0-alpha", "0.1.0-alpha.1", Ordering::Less),
            ("0.1.0-beta", "0.1.0-alpha.9", Ordering::Greater),
        ] {
            assert_eq!(
                SemVer::parse(a).cmp(&SemVer::parse(b)),
                expected,
                "{a} vs {b}"
            );
        }
    }

    #[test]
    fn finds_anywhere_and_defaults() {
        assert_eq!(SemVer::parse("Tako 1.2").to_string(), "1.2.0");
        assert_eq!(SemVer::parse("v0.1.0-alpha.5").to_string(), "0.1.0-alpha.5");
        assert_eq!(SemVer::parse("sem versão").to_string(), "0.0.0");
        assert!(SemVer::find("").is_none());
        assert!(SemVer::find("99999999999.1").is_none());
        assert!(SemVer::parse("0.1.0-alpha.5").is_prerelease());
        assert!(!SemVer::current().to_string().is_empty());
    }
}
