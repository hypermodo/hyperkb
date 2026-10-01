use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;

/// Hybrid Logical Clock (HLC) combining physical wall time with a logical counter.
/// Guarantees monotonic ordering across distributed nodes and resilient to NTP clock jumps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hlc {
    pub physical: i64, // Milliseconds since UNIX epoch
    pub logical: u32,  // Monotonic sequence within the same millisecond
    pub node_id: u16,  // Node discriminator to break ties deterministically
}

impl Hlc {
    pub fn new(node_id: u16) -> Self {
        Self {
            physical: Utc::now().timestamp_millis(),
            logical: 0,
            node_id,
        }
    }

    pub fn from_parts(physical: i64, logical: u32, node_id: u16) -> Self {
        Self {
            physical,
            logical,
            node_id,
        }
    }

    /// Advance this clock for a local mutation.
    pub fn tick(&mut self) -> Self {
        let now = Utc::now().timestamp_millis();
        if now > self.physical {
            self.physical = now;
            self.logical = 0;
        } else {
            self.logical += 1;
        }
        *self
    }

    /// Update this clock after receiving an HLC from a remote node (e.g. SaaS sync).
    pub fn update(&mut self, remote: &Hlc) -> Self {
        let now = Utc::now().timestamp_millis();
        if now > self.physical && now > remote.physical {
            self.physical = now;
            self.logical = 0;
        } else if self.physical == remote.physical {
            self.logical = self.logical.max(remote.logical) + 1;
        } else if self.physical > remote.physical {
            self.logical += 1;
        } else {
            self.physical = remote.physical;
            self.logical = remote.logical + 1;
        }
        *self
    }

    /// Encodes HLC as a sortable, fixed-width string for SQLite indexing.
    pub fn to_sortable_string(&self) -> String {
        format!("{:016x}-{:08x}-{:04x}", self.physical, self.logical, self.node_id)
    }

    /// Parses the sortable string back into an HLC.
    pub fn from_sortable_string(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return None;
        }
        let physical = i64::from_str_radix(parts[0], 16).ok()?;
        let logical = u32::from_str_radix(parts[1], 16).ok()?;
        let node_id = u16::from_str_radix(parts[2], 16).ok()?;
        Some(Self {
            physical,
            logical,
            node_id,
        })
    }
}

impl PartialOrd for Hlc {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Hlc {
    fn cmp(&self, other: &Self) -> Ordering {
        self.physical
            .cmp(&other.physical)
            .then_with(|| self.logical.cmp(&other.logical))
            .then_with(|| self.node_id.cmp(&other.node_id))
    }
}

impl fmt::Display for Hlc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}:{:04x}",
            DateTime::from_timestamp_millis(self.physical)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| self.physical.to_string()),
            self.logical,
            self.node_id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hlc_monotonicity() {
        let mut clock = Hlc::new(1);
        let t1 = clock.tick();
        let t2 = clock.tick();
        assert!(t2 > t1);
    }

    #[test]
    fn test_hlc_string_roundtrip() {
        let hlc = Hlc::from_parts(1727720000000, 42, 7);
        let serialized = hlc.to_sortable_string();
        let parsed = Hlc::from_sortable_string(&serialized).expect("should parse");
        assert_eq!(hlc, parsed);
    }
}

