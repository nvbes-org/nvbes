use super::db::SecuritySummaryRow;
use super::types::{
    EnterpriseSecurityPostureControl, EnterpriseSecurityPostureScore, EnterpriseSecurityStatus,
};

const MAX_ADMIN_SESSION_TTL_HOURS: i64 = 8;

pub fn security_posture(
    summary: &SecuritySummaryRow,
    auth_session_ttl_hours: i64,
) -> EnterpriseSecurityPostureScore {
    let controls = vec![
        control(
            "admin_mfa",
            "Admin MFA",
            "Owner and admin accounts must use phishing-resistant MFA or TOTP.",
            if summary.admin_without_mfa_count == 0 {
                EnterpriseSecurityStatus::Complete
            } else {
                EnterpriseSecurityStatus::Critical
            },
            25,
            if summary.admin_without_mfa_count == 0 {
                25
            } else {
                0
            },
            "Activer MFA obligatoire pour tous les admins et bloquer les exceptions.",
            "Identity",
            format!("{} admins without MFA", summary.admin_without_mfa_count),
            "Review admins",
            "/users",
        ),
        control(
            "session_ttl",
            "Session TTL",
            "Tenant sessions should expire quickly enough for privileged contexts.",
            if auth_session_ttl_hours <= MAX_ADMIN_SESSION_TTL_HOURS {
                EnterpriseSecurityStatus::Complete
            } else {
                EnterpriseSecurityStatus::Attention
            },
            15,
            if auth_session_ttl_hours <= MAX_ADMIN_SESSION_TTL_HOURS {
                15
            } else {
                5
            },
            "Reduire le TTL des sessions admin a 8 heures avec re-auth step-up.",
            "Identity",
            format!("{auth_session_ttl_hours}h configured"),
            "Open policies",
            "/policies",
        ),
        control(
            "verified_domain",
            "Verified domain",
            "The tenant primary email domain is DNS verified and protected from spoofing.",
            if summary.verified_domain_count > 0 {
                EnterpriseSecurityStatus::Complete
            } else {
                EnterpriseSecurityStatus::Critical
            },
            20,
            if summary.verified_domain_count > 0 {
                20
            } else {
                0
            },
            "Verifier le domaine principal par DNS avant tout auto-provisioning.",
            "Tenant admin",
            format!("{} verified domains", summary.verified_domain_count),
            "Open settings",
            "/settings",
        ),
        control(
            "sso",
            "SSO",
            "Enterprise sign-in should be centralized through a managed IdP.",
            if summary.sso_provider_count > 0 {
                EnterpriseSecurityStatus::Complete
            } else {
                EnterpriseSecurityStatus::Attention
            },
            20,
            if summary.sso_provider_count > 0 {
                20
            } else {
                8
            },
            "Configurer SSO SAML/OIDC puis forcer les admins sur le fournisseur verifie.",
            "Identity",
            format!("{} active identity providers", summary.sso_provider_count),
            "Configure SSO",
            "/settings",
        ),
        control(
            "old_secrets",
            "Old secrets",
            "OAuth clients and service accounts should not keep stale secret versions active.",
            if summary.stale_secret_count == 0 {
                EnterpriseSecurityStatus::Complete
            } else {
                EnterpriseSecurityStatus::Attention
            },
            20,
            if summary.stale_secret_count == 0 {
                20
            } else {
                10
            },
            "Revoquer les anciens secrets apres la fenetre de rotation approuvee.",
            "Developers",
            format!("{} stale active secrets", summary.stale_secret_count),
            "Review secrets",
            "/developers",
        ),
    ];

    let max_score = controls.iter().map(|control| control.weight).sum::<i64>();
    let completed_weight = controls
        .iter()
        .map(|control| control.completed_weight)
        .sum::<i64>();
    let score = if max_score == 0 {
        0
    } else {
        ((completed_weight * 100) as f64 / max_score as f64).round() as i64
    };
    let completed_controls = controls
        .iter()
        .filter(|control| control.status == EnterpriseSecurityStatus::Complete)
        .count() as i64;
    let status = if score >= 80 {
        EnterpriseSecurityStatus::Complete
    } else if score >= 50 {
        EnterpriseSecurityStatus::Attention
    } else {
        EnterpriseSecurityStatus::Critical
    };

    EnterpriseSecurityPostureScore {
        score,
        max_score,
        completed_weight,
        status,
        completed_controls,
        total_controls: controls.len() as i64,
        controls,
    }
}

fn control(
    id: &str,
    label: &str,
    description: &str,
    status: EnterpriseSecurityStatus,
    weight: i64,
    completed_weight: i64,
    recommendation: &str,
    owner: &str,
    evidence: String,
    action_label: &str,
    action_path: &str,
) -> EnterpriseSecurityPostureControl {
    EnterpriseSecurityPostureControl {
        id: id.to_string(),
        label: label.to_string(),
        description: description.to_string(),
        status,
        weight,
        completed_weight,
        recommendation: recommendation.to_string(),
        owner: owner.to_string(),
        evidence,
        action_label: action_label.to_string(),
        action_path: action_path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posture_scores_complete_controls() {
        let score = security_posture(
            &SecuritySummaryRow {
                mfa_factor_count: 5,
                passkey_count: 2,
                high_risk_event_count: 0,
                admin_without_mfa_count: 0,
                verified_domain_count: 1,
                sso_provider_count: 1,
                stale_secret_count: 0,
            },
            8,
        );

        assert_eq!(score.score, 100);
        assert_eq!(score.status, EnterpriseSecurityStatus::Complete);
        assert_eq!(score.completed_controls, 5);
        assert_eq!(score.controls[0].action_path, "/users");
    }

    #[test]
    fn posture_prioritizes_missing_admin_mfa_and_domain() {
        let score = security_posture(
            &SecuritySummaryRow {
                mfa_factor_count: 0,
                passkey_count: 0,
                high_risk_event_count: 0,
                admin_without_mfa_count: 2,
                verified_domain_count: 0,
                sso_provider_count: 0,
                stale_secret_count: 1,
            },
            24,
        );

        assert_eq!(score.score, 23);
        assert_eq!(score.status, EnterpriseSecurityStatus::Critical);
        assert_eq!(score.completed_controls, 0);
        assert_eq!(score.controls[4].action_path, "/developers");
    }
}
