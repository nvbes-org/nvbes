use super::AuditEventView;

pub fn events_to_csv(events: &[AuditEventView]) -> String {
    let mut csv = String::from(
        "id,workspace_id,created_at,actor_user_id,actor_principal_id,actor_email,action,target_type,target_id,ip,user_agent,metadata,previous_event_hash,event_hash\n",
    );

    for event in events {
        let columns = [
            event.id.to_string(),
            event.workspace_id.to_string(),
            event.created_at.to_rfc3339(),
            event
                .actor_user_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            event
                .actor_principal_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            event.actor_email.clone().unwrap_or_default(),
            event.action.clone(),
            event.target_type.clone(),
            event.target_id.map(|id| id.to_string()).unwrap_or_default(),
            event.ip.clone().unwrap_or_default(),
            event.user_agent.clone().unwrap_or_default(),
            event.metadata.to_string(),
            event.previous_event_hash.clone().unwrap_or_default(),
            event.event_hash.clone(),
        ];

        csv.push_str(
            &columns
                .iter()
                .map(|column| csv_escape(column))
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');
    }

    csv
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}
