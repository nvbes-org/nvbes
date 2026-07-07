use tonic::Status;

pub(crate) struct AccessReviewScope {
    pub(super) include_members: bool,
    pub(super) include_roles: bool,
    pub(super) include_service_accounts: bool,
    pub(super) include_oauth_clients: bool,
}

impl AccessReviewScope {
    pub(super) fn from_contract_request(
        scope: &str,
        include_members: bool,
        include_roles: bool,
        include_service_accounts: bool,
        include_oauth_clients: bool,
    ) -> Result<Self, Status> {
        if include_members || include_roles || include_service_accounts || include_oauth_clients {
            Ok(Self {
                include_members,
                include_roles,
                include_service_accounts,
                include_oauth_clients,
            })
        } else {
            Self::parse(scope)
        }
    }

    pub(super) fn parse(value: &str) -> Result<Self, Status> {
        match value.trim() {
            "all" => Ok(Self {
                include_members: true,
                include_roles: true,
                include_service_accounts: true,
                include_oauth_clients: true,
            }),
            "members" => Ok(Self::only_members()),
            "roles" => Ok(Self {
                include_members: false,
                include_roles: true,
                include_service_accounts: false,
                include_oauth_clients: false,
            }),
            "service_accounts" => Ok(Self {
                include_members: false,
                include_roles: false,
                include_service_accounts: true,
                include_oauth_clients: false,
            }),
            "oauth_clients" => Ok(Self {
                include_members: false,
                include_roles: false,
                include_service_accounts: false,
                include_oauth_clients: true,
            }),
            _ => Err(Status::invalid_argument(
                "scope must be all, members, roles, service_accounts, or oauth_clients",
            )),
        }
    }

    pub(super) fn only_members() -> Self {
        Self {
            include_members: true,
            include_roles: false,
            include_service_accounts: false,
            include_oauth_clients: false,
        }
    }

    pub(super) fn as_contract_scope(&self) -> &'static str {
        match (
            self.include_members,
            self.include_roles,
            self.include_service_accounts,
            self.include_oauth_clients,
        ) {
            (true, true, true, true) => "all",
            (true, false, false, false) => "members",
            (false, true, false, false) => "roles",
            (false, false, true, false) => "service_accounts",
            (false, false, false, true) => "oauth_clients",
            _ => "custom",
        }
    }
}
