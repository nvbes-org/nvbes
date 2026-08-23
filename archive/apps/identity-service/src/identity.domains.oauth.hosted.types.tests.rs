use super::HostedLoginDecision;

#[test]
fn hosted_login_decisions_use_the_public_snake_case_discriminator() {
    let decisions = [
        HostedLoginDecision::LoginRequired {
            login_url: "/login".to_string(),
            state_id: "hosted_login".to_string(),
        },
        HostedLoginDecision::ConsentRequired {
            state_id: "hosted_consent".to_string(),
            client: Box::new(super::HostedClientDisplay {
                client_id: "account-web".to_string(),
                name: "Account Web".to_string(),
                logo_url: None,
                description: None,
                support_url: None,
                privacy_url: None,
                terms_url: None,
                brand_color: None,
                custom_css: None,
                help_text: None,
            }),
            scope: "openid".to_string(),
        },
        HostedLoginDecision::Redirect {
            redirect_url: "https://account.example.test/callback".to_string(),
        },
        HostedLoginDecision::ErrorPage {
            code: "invalid_request".to_string(),
            message: "Invalid request".to_string(),
        },
    ];

    let discriminators = decisions
        .into_iter()
        .map(|decision| {
            serde_json::to_value(decision).expect("decision should serialize")["kind"]
                .as_str()
                .expect("decision should have a string discriminator")
                .to_string()
        })
        .collect::<Vec<_>>();

    assert_eq!(
        discriminators,
        [
            "login_required",
            "consent_required",
            "redirect",
            "error_page"
        ]
    );
}
