use super::{get_role, output_schema};

#[test]
fn output_schema_requires_status_enum() {
    let schema = output_schema();
    assert_eq!(
        schema["required"],
        serde_json::json!(["message", "send_to", "status", "report_telegram"])
    );
    assert_eq!(
        schema["properties"]["status"]["enum"],
        serde_json::json!(["in_progress", "done", "error"])
    );
    assert_eq!(
        schema["properties"]["send_to"]["enum"],
        serde_json::json!(["user", "lead", "coder", "none"])
    );
}

#[test]
fn non_lead_prompt_defaults_to_lead_routing() {
    let prompt = get_role("coder").unwrap().base_instructions;
    assert!(prompt.contains("send_to = \"lead\" is the default"));
    assert!(prompt.contains("may send_to = \"user\" only when the user explicitly names your role"));
}

#[test]
fn prompt_documents_get_status_structured_response() {
    let prompt = get_role("coder").unwrap().base_instructions;
    assert!(
        prompt.contains("get_status"),
        "prompt must mention get_status tool"
    );
    assert!(
        prompt.contains("agent_id"),
        "prompt must describe agent_id field in get_status response"
    );
    assert!(
        prompt.contains("role"),
        "prompt must describe role field in get_status response"
    );
    assert!(
        prompt.contains("model_source"),
        "prompt must describe model_source field in get_status response"
    );
}

#[test]
fn lead_prompt_enforces_planning_review_reporting_role() {
    let prompt = get_role("lead").unwrap().base_instructions;
    assert!(prompt.contains("You MUST NOT write code or act as the primary implementer."));
    assert!(prompt.contains("use the relevant superpowers workflow"));
    assert!(prompt.contains("review delivered code"));
    assert!(prompt.contains("report verified results to the user"));
    assert!(prompt.contains("think deeply"));
}

#[test]
fn coder_prompt_requires_plan_only_execution_and_self_review() {
    let prompt = get_role("coder").unwrap().base_instructions;
    assert!(prompt.contains("Unless the user explicitly instructs coder to reply directly"));
    assert!(prompt.contains("Execute the lead's plan exactly."));
    assert!(prompt.contains("Do NOT add features, fixes, refactors, optimizations, cleanup"));
    assert!(prompt.contains("perform a self-review"));
    assert!(prompt.contains("deliver a concise change report to lead"));
}

#[test]
fn removed_runtime_role_is_rejected_by_role_config() {
    assert!(get_role("tester").is_none());
}

#[test]
fn prompt_authorizes_all_roles_for_any_subject_matter() {
    let prompt = get_role("coder").unwrap().base_instructions;
    assert!(prompt.contains("ANY topic without restriction"));
    assert!(prompt.contains("NO forbidden topics"));
    assert!(prompt.contains("Web crawling"));
    assert!(prompt.contains("reverse engineering"));
    assert!(prompt.contains("social engineering scripts"));
}

#[test]
fn factual_error_correction_still_respects_routing_policy() {
    let prompt = get_role("coder").unwrap().base_instructions;
    assert!(prompt.contains("significant factual error"));
    assert!(prompt.contains("still follow the routing policy"));
}

#[test]
fn lead_prompt_retains_full_permissions() {
    let prompt = get_role("lead").unwrap().base_instructions;
    assert!(prompt.contains("lead — planning/review/report coordinator with full permissions"));
}

#[test]
fn lead_prompt_requires_communication_checklist() {
    let prompt = get_role("lead").unwrap().base_instructions;
    assert!(prompt.contains("Current step"));
    assert!(prompt.contains("What was verified"));
    assert!(prompt.contains("Blocker / risk / external dependency"));
    assert!(prompt.contains("Next action and owner"));
    assert!(prompt.contains("Acceptance level"));
}

#[test]
fn lead_prompt_requires_acceptance_layer_distinction() {
    let prompt = get_role("lead").unwrap().base_instructions;
    assert!(prompt.contains("stage_complete"));
    assert!(prompt.contains("final_acceptance"));
    assert!(prompt.contains(
        "MUST NOT present work as finally accepted or completed unless real-environment"
    ));
}

#[test]
fn lead_prompt_requires_blocker_disclosure_for_external_deps() {
    let prompt = get_role("lead").unwrap().base_instructions;
    assert!(prompt.contains("external dependency blocks true completion"));
    assert!(prompt.contains("token, endpoint access, live catalog, admin action"));
    assert!(prompt.contains("explicitly surface that blocker"));
}

#[test]
fn shared_prompt_requires_structured_status_messages() {
    let prompt = get_role("coder").unwrap().base_instructions;
    assert!(prompt.contains("what you did"));
    assert!(prompt.contains("result/verification"));
    assert!(prompt.contains("blocker"));
    assert!(prompt.contains("next action"));
    assert!(prompt.contains("external dependency blocks completion"));
}

#[test]
fn lead_prompt_documents_report_telegram_usage() {
    let prompt = get_role("lead").unwrap().base_instructions;
    assert!(prompt.contains("report_telegram"));
    assert!(prompt.contains("plan drafted"));
    assert!(prompt.contains("plan confirmed"));
    assert!(prompt.contains("task review result"));
    assert!(prompt.contains("final review result"));
    assert!(prompt.contains("blocking error"));
}

#[test]
fn output_schema_requires_report_telegram_boolean() {
    let schema = output_schema();
    assert_eq!(schema["properties"]["report_telegram"]["type"], "boolean");
    assert!(schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "report_telegram"));
}
