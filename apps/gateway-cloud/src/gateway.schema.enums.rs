use async_graphql::{Enum, Error, Result};

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum BillingProvider {
    Stripe,
    Mollie,
    Cb,
}

impl BillingProvider {
    pub fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "stripe" => Ok(Self::Stripe),
            "mollie" => Ok(Self::Mollie),
            "cb" => Ok(Self::Cb),
            _ => Err(Error::new(format!("unsupported billing provider: {value}"))),
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum BillingSubscriptionStatus {
    Trialing,
    Active,
    PastDue,
    Canceled,
    Incomplete,
}

impl BillingSubscriptionStatus {
    pub fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "trialing" => Ok(Self::Trialing),
            "active" => Ok(Self::Active),
            "past_due" => Ok(Self::PastDue),
            "canceled" => Ok(Self::Canceled),
            "incomplete" => Ok(Self::Incomplete),
            _ => Err(Error::new(format!("unsupported subscription status: {value}"))),
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum BillingInvoiceStatus {
    Draft,
    Open,
    Paid,
    Void,
    Uncollectible,
}

impl BillingInvoiceStatus {
    pub fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "draft" => Ok(Self::Draft),
            "open" => Ok(Self::Open),
            "paid" => Ok(Self::Paid),
            "void" => Ok(Self::Void),
            "uncollectible" => Ok(Self::Uncollectible),
            _ => Err(Error::new(format!("unsupported invoice status: {value}"))),
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum BillingProviderReferenceStatus {
    Active,
    Inactive,
    Failed,
}

impl BillingProviderReferenceStatus {
    pub fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "inactive" | "" => Ok(Self::Inactive),
            "failed" => Ok(Self::Failed),
            _ => Err(Error::new(format!("unsupported provider reference status: {value}"))),
        }
    }
}
