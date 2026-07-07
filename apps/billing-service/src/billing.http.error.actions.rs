use crate::http::error::AppError;

impl From<nvbes_billing::db::ProviderRoutingRuleLookupError> for AppError {
    fn from(err: nvbes_billing::db::ProviderRoutingRuleLookupError) -> Self {
        match err {
            nvbes_billing::db::ProviderRoutingRuleLookupError::Database(err) => err.into(),
            nvbes_billing::db::ProviderRoutingRuleLookupError::InvalidProvider(_) => {
                Self::internal(
                    "billing_provider_routing_rule_invalid",
                    "Billing provider routing rule contains an unsupported provider.",
                )
            }
        }
    }
}

impl From<nvbes_billing::checkout_provider::CheckoutProviderError> for AppError {
    fn from(err: nvbes_billing::checkout_provider::CheckoutProviderError) -> Self {
        match err {
            nvbes_billing::checkout_provider::CheckoutProviderError::InvalidPlan => {
                Self::bad_request("invalid_plan", "Unsupported billing plan.")
            }
            nvbes_billing::checkout_provider::CheckoutProviderError::PlanNotFound => {
                Self::not_found("plan_not_found", "Plan not found.")
            }
            nvbes_billing::checkout_provider::CheckoutProviderError::MissingProviderPriceMapping => {
                Self::conflict(
                    "missing_provider_price_mapping",
                    "No active provider price mapping exists for this plan.",
                )
            }
            nvbes_billing::checkout_provider::CheckoutProviderError::Database(err) => err.into(),
            nvbes_billing::checkout_provider::CheckoutProviderError::Stripe(err) => err.into(),
            nvbes_billing::checkout_provider::CheckoutProviderError::Mollie(err) => err.into(),
            nvbes_billing::checkout_provider::CheckoutProviderError::ProviderNotImplemented => {
                Self::conflict(
                    "provider_not_implemented",
                    "Billing provider is not implemented for checkout.",
                )
            }
        }
    }
}

impl From<nvbes_billing::checkout_sessions::BillingCheckoutSessionError> for AppError {
    fn from(err: nvbes_billing::checkout_sessions::BillingCheckoutSessionError) -> Self {
        match err {
            nvbes_billing::checkout_sessions::BillingCheckoutSessionError::WorkspaceNotFound => {
                Self::not_found("workspace_not_found", "Workspace not found.")
            }
            nvbes_billing::checkout_sessions::BillingCheckoutSessionError::BillingLocked => {
                Self::forbidden(
                    "billing_locked",
                    "Billing actions are temporarily blocked for this workspace.",
                )
            }
            nvbes_billing::checkout_sessions::BillingCheckoutSessionError::InvalidRedirect {
                field,
                env_name,
                source,
            } => match source {
                nvbes_billing::BillingRedirectUrlError::InvalidAbsoluteUrl => Self::bad_request(
                    "invalid_billing_return_url",
                    format!("{field} must be a valid absolute URL."),
                ),
                nvbes_billing::BillingRedirectUrlError::InvalidOrigin => Self::bad_request(
                    "invalid_billing_return_url",
                    format!(
                        "{field} must stay on the configured application origin. Set {env_name} to an allowed URL."
                    ),
                ),
            },
            nvbes_billing::checkout_sessions::BillingCheckoutSessionError::FraudPolicy(error) => {
                Self::internal("billing_fraud_policy_invalid", error.to_string())
            }
            nvbes_billing::checkout_sessions::BillingCheckoutSessionError::ManualReviewHold => {
                Self::conflict(
                    "billing_checkout_manual_review",
                    "This checkout requires manual review before it can continue.",
                )
            }
            nvbes_billing::checkout_sessions::BillingCheckoutSessionError::FraudBlocked => {
                Self::forbidden(
                    "billing_checkout_fraud_blocked",
                    "This checkout is blocked by the billing risk policy.",
                )
            }
            nvbes_billing::checkout_sessions::BillingCheckoutSessionError::RoutingBlocked(
                error,
            ) => Self::conflict(
                error.as_str(),
                "No compliant billing provider is available for this checkout policy.",
            ),
            nvbes_billing::checkout_sessions::BillingCheckoutSessionError::RoutingRule(err) => {
                err.into()
            }
            nvbes_billing::checkout_sessions::BillingCheckoutSessionError::CheckoutProvider(
                err,
            ) => err.into(),
            nvbes_billing::checkout_sessions::BillingCheckoutSessionError::Database(err) => {
                err.into()
            }
        }
    }
}

impl From<nvbes_billing::portal_sessions::PortalSessionError> for AppError {
    fn from(err: nvbes_billing::portal_sessions::PortalSessionError) -> Self {
        match err {
            nvbes_billing::portal_sessions::PortalSessionError::UnknownBillingProvider => {
                Self::conflict(
                    "unknown_billing_provider",
                    "Billing provider is not supported.",
                )
            }
            nvbes_billing::portal_sessions::PortalSessionError::ProviderPortalUnavailable => {
                Self::conflict(
                    "provider_portal_unavailable",
                    "Billing portal is not available for the current billing provider.",
                )
            }
            nvbes_billing::portal_sessions::PortalSessionError::MissingBillingCustomer => {
                Self::conflict(
                    "missing_billing_customer",
                    "Create a checkout session before opening the billing portal.",
                )
            }
            nvbes_billing::portal_sessions::PortalSessionError::Redirect(error) => match error {
                nvbes_billing::BillingRedirectUrlError::InvalidAbsoluteUrl => Self::bad_request(
                    "invalid_billing_return_url",
                    "return_url must be a valid absolute URL.",
                ),
                nvbes_billing::BillingRedirectUrlError::InvalidOrigin => Self::bad_request(
                    "invalid_billing_return_url",
                    "return_url must stay on the configured application origin. Set NVBES_BILLING_PORTAL_RETURN_URL to an allowed URL.",
                ),
            },
            nvbes_billing::portal_sessions::PortalSessionError::Stripe(err) => err.into(),
        }
    }
}

impl From<nvbes_billing::portal_actions::BillingPortalActionError> for AppError {
    fn from(err: nvbes_billing::portal_actions::BillingPortalActionError) -> Self {
        match err {
            nvbes_billing::portal_actions::BillingPortalActionError::WorkspaceNotFound => {
                Self::not_found("workspace_not_found", "Workspace not found.")
            }
            nvbes_billing::portal_actions::BillingPortalActionError::BillingLocked => {
                Self::forbidden(
                    "billing_locked",
                    "Billing actions are temporarily blocked for this workspace.",
                )
            }
            nvbes_billing::portal_actions::BillingPortalActionError::Portal(err) => err.into(),
            nvbes_billing::portal_actions::BillingPortalActionError::Database(err) => err.into(),
        }
    }
}

impl From<nvbes_billing::stripe::StripeProviderError> for AppError {
    fn from(err: nvbes_billing::stripe::StripeProviderError) -> Self {
        match err {
            nvbes_billing::stripe::StripeProviderError::NotConfigured => Self::conflict(
                "stripe_not_configured",
                "NVBES_STRIPE_SECRET_KEY must be configured before billing actions.",
            ),
            nvbes_billing::stripe::StripeProviderError::RequestRejected {
                status: 400,
                message,
            } => Self::bad_request("stripe_request_rejected", message),
            nvbes_billing::stripe::StripeProviderError::RequestRejected { message, .. } => {
                Self::conflict("stripe_request_rejected", message)
            }
            nvbes_billing::stripe::StripeProviderError::RequestFailed(message) => {
                Self::internal("stripe_request_failed", message)
            }
            nvbes_billing::stripe::StripeProviderError::ResponseFailed(message) => {
                Self::internal("stripe_response_failed", message)
            }
            nvbes_billing::stripe::StripeProviderError::ResponseInvalid(message) => {
                Self::internal("stripe_response_invalid", message)
            }
        }
    }
}

impl From<nvbes_billing::mollie::MollieProviderError> for AppError {
    fn from(err: nvbes_billing::mollie::MollieProviderError) -> Self {
        if err.is_bad_request() {
            Self::bad_request(err.code(), err.message())
        } else if matches!(
            err,
            nvbes_billing::mollie::MollieProviderError::NotConfigured
        ) {
            Self::conflict(err.code(), err.message())
        } else {
            Self::internal(err.code(), err.message())
        }
    }
}

impl From<nvbes_billing::stripe_webhook_intake::BillingWebhookIntakeError> for AppError {
    fn from(err: nvbes_billing::stripe_webhook_intake::BillingWebhookIntakeError) -> Self {
        match err {
            nvbes_billing::stripe_webhook_intake::BillingWebhookIntakeError::MissingStripeWebhookSecret => {
                Self::conflict(
                    "stripe_webhook_secret_missing",
                    "Stripe webhook intake is not configured.",
                )
            }
            nvbes_billing::stripe_webhook_intake::BillingWebhookIntakeError::StripeSignature(_) => {
                Self::bad_request("stripe_webhook_signature_invalid", err.to_string())
            }
            nvbes_billing::stripe_webhook_intake::BillingWebhookIntakeError::InvalidPayload => {
                Self::bad_request("billing_webhook_payload_invalid", err.to_string())
            }
            nvbes_billing::stripe_webhook_intake::BillingWebhookIntakeError::Database(error) => {
                error.into()
            }
            nvbes_billing::stripe_webhook_intake::BillingWebhookIntakeError::Queue(error) => {
                Self::internal("billing_webhook_enqueue_failed", error.to_string())
            }
        }
    }
}
