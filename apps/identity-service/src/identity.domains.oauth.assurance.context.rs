use chrono::Utc;
use nvbes_core::auth::Aal;

use super::{AssuranceContext, SessionAssuranceState};

pub(super) fn build_assurance_context(
    required: Aal,
    session_state: Option<&SessionAssuranceState>,
) -> AssuranceContext {
    let achieved = achieved_assurance_level(session_state);
    let has_step_up = has_valid_step_up(session_state);
    let mut amr = session_state
        .map(|state| state.amr.clone())
        .unwrap_or_else(|| vec!["pwd".to_string()]);

    if amr.is_empty() {
        amr.push("pwd".to_string());
    }
    if has_step_up && !amr.iter().any(|value| value == "otp") {
        amr.push("otp".to_string());
    }

    let auth_time = session_state
        .and_then(|state| state.step_up_verified_at)
        .or_else(|| session_state.map(|state| state.created_at))
        .unwrap_or_else(Utc::now)
        .timestamp();

    AssuranceContext {
        sufficient: achieved >= required,
        acr: achieved.as_str().to_string(),
        amr,
        auth_time,
    }
}

fn achieved_assurance_level(session_state: Option<&SessionAssuranceState>) -> Aal {
    let mut achieved = session_state
        .and_then(|state| {
            state
                .acr
                .as_deref()
                .and_then(|value| value.parse::<Aal>().ok())
        })
        .unwrap_or(Aal::Aal1);

    if has_valid_step_up(session_state) {
        achieved = std::cmp::max(achieved, Aal::Aal2);
    }

    achieved
}

fn has_valid_step_up(session_state: Option<&SessionAssuranceState>) -> bool {
    session_state.is_some_and(|state| {
        state
            .step_up_expires_at
            .is_some_and(|value| value > Utc::now())
    })
}
