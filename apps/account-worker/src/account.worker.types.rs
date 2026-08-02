use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClosureParticipant {
    Cloud,
    Billing,
    Identity,
    Account,
}

impl ClosureParticipant {
    pub fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "cloud" => Ok(Self::Cloud),
            "billing" => Ok(Self::Billing),
            "identity" => Ok(Self::Identity),
            "account" => Ok(Self::Account),
            _ => anyhow::bail!("Unknown Account closure participant: {value}"),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cloud => "cloud",
            Self::Billing => "billing",
            Self::Identity => "identity",
            Self::Account => "account",
        }
    }
}

pub struct ClaimedProjection {
    pub event_id: Uuid,
    pub payload: serde_json::Value,
}

pub struct ClaimedClosure {
    pub event_id: Uuid,
    pub saga_id: Uuid,
    pub principal_id: Uuid,
    pub avatar_object_key: Option<String>,
    pub payload: serde_json::Value,
    pub participant: ClosureParticipant,
    pub participant_attempts: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportParticipant {
    Cloud,
    Billing,
    Identity,
    Account,
}

impl ExportParticipant {
    pub fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "cloud" => Ok(Self::Cloud),
            "billing" => Ok(Self::Billing),
            "identity" => Ok(Self::Identity),
            "account" => Ok(Self::Account),
            _ => anyhow::bail!("Unknown Account export participant: {value}"),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cloud => "cloud",
            Self::Billing => "billing",
            Self::Identity => "identity",
            Self::Account => "account",
        }
    }
}

pub struct ClaimedExport {
    pub event_id: Uuid,
    pub export_id: Uuid,
    pub principal_id: Uuid,
    pub payload: serde_json::Value,
    pub participant: ExportParticipant,
    pub participant_attempts: i32,
}
