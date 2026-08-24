use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recommendation {
    Allow,
    Challenge,
    Review,
    Deny,
}

impl Recommendation {
    pub const fn strictest(self, other: Self) -> Self {
        if self.rank() >= other.rank() {
            self
        } else {
            other
        }
    }

    const fn rank(self) -> u8 {
        match self {
            Self::Allow => 0,
            Self::Challenge => 1,
            Self::Review => 2,
            Self::Deny => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskBand {
    Low,
    Elevated,
    High,
    Critical,
}
