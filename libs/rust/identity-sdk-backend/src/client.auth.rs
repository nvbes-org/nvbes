use super::{http, trace, IdentityClient};
use crate::types::*;
use crate::SdkError;

impl IdentityClient {
    /// Enregistre un nouvel utilisateur dans le flux Identity actuel.
    pub async fn register(&self, input: RegisterInput) -> Result<RegisterResult, SdkError> {
        self.send_json(self.http.post(self.endpoint("/auth/register")).json(&input))
            .await
    }

    /// Login email/password via le flux challenge identifier -> password.
    pub async fn login(&self, input: LoginInput) -> Result<LoginResult, SdkError> {
        let identifier = self.start_login(&input.email).await?;
        match self
            .submit_login_password(&identifier.state_token, &input.password)
            .await?
        {
            LoginPasswordResult::Success(result) => Ok(result),
            LoginPasswordResult::MfaRequired(challenge) => Err(SdkError::MfaRequired(format!(
                "MFA challenge required with methods: {}",
                challenge.available_methods.unwrap_or_default().join(", ")
            ))),
        }
    }

    pub async fn start_login(&self, email: &str) -> Result<IdentifierResult, SdkError> {
        self.send_json(
            self.http
                .post(self.endpoint("/auth/challenge/identifier"))
                .json(&serde_json::json!({ "email": email })),
        )
        .await
    }

    pub async fn submit_login_password(
        &self,
        state_token: &str,
        password: &str,
    ) -> Result<LoginPasswordResult, SdkError> {
        let request =
            self.http
                .post(self.endpoint("/auth/challenge/pwd"))
                .json(&serde_json::json!({
                    "state_token": state_token,
                    "password": password,
                }));
        let response = trace::with_fresh_trace_headers(request).send().await?;

        if http::is_accepted(response.status()) {
            return Ok(LoginPasswordResult::MfaRequired(response.json().await?));
        }

        if !response.status().is_success() {
            return Err(http::auth_error(response).await);
        }

        Ok(LoginPasswordResult::Success(response.json().await?))
    }

    pub async fn submit_login_mfa(
        &self,
        input: MfaChallengeInput,
    ) -> Result<LoginResult, SdkError> {
        self.send_json(
            self.http
                .post(self.endpoint("/auth/challenge/mfa"))
                .json(&input),
        )
        .await
    }

    /// Récupère le contexte utilisateur courant via Bearer ou cookie de session.
    pub async fn get_me(&self, access_token: Option<&str>) -> Result<MeResult, SdkError> {
        let mut request = self.http.get(self.endpoint("/auth/me"));
        if let Some(token) = access_token {
            request = request.bearer_auth(token);
        }

        self.send_json(request).await
    }

    /// Compatibilité avec les anciens appelants qui ne lisaient que le `user`.
    pub async fn get_user_info(&self, access_token: &str) -> Result<UserView, SdkError> {
        Ok(self.get_me(Some(access_token)).await?.user)
    }

    pub async fn list_workspaces(
        &self,
        access_token: Option<&str>,
    ) -> Result<Vec<WorkspaceView>, SdkError> {
        let mut request = self.http.get(self.endpoint("/workspaces"));
        if let Some(token) = access_token {
            request = request.bearer_auth(token);
        }

        let result: WorkspacesResult = self.send_json(request).await?;
        Ok(result.workspaces)
    }

    /// Déconnecte la session courante.
    pub async fn logout(&self, session_token: Option<&str>) -> Result<(), SdkError> {
        let mut request = self.http.post(self.endpoint("/auth/logout"));

        if let Some(token) = session_token {
            request = request.bearer_auth(token);
        }

        self.send_empty(request).await
    }
}
