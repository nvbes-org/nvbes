use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Aal {
    #[serde(rename = "aal1")]
    #[default]
    Aal1 = 1,
    #[serde(rename = "aal2")]
    Aal2 = 2,
    #[serde(rename = "aal3")]
    Aal3 = 3,
}

impl Aal {
    pub fn as_str(&self) -> &'static str {
        match self {
            Aal::Aal1 => "aal1",
            Aal::Aal2 => "aal2",
            Aal::Aal3 => "aal3",
        }
    }
}

impl FromStr for Aal {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "aal3" => Ok(Aal::Aal3),
            "aal2" => Ok(Aal::Aal2),
            "aal1" | "" => Ok(Aal::Aal1),
            _ => Err(()),
        }
    }
}

pub fn max_aal(left: Aal, right: Aal) -> Aal {
    if left >= right { left } else { right }
}

#[path = "auth.helpers.rs"]
pub mod helpers;

pub use helpers::*;
