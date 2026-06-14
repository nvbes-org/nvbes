pub fn can_manage_members(role: &str, grants: &[String]) -> bool {
    role == "owner" || (role == "admin" && grants.iter().any(|grant| grant == "members"))
}

pub fn is_last_owner_removal(owner_count: i64, current_role: &str, next_role: &str) -> bool {
    owner_count <= 1 && current_role == "owner" && next_role != "owner"
}
