use super::{get_role, output_schema};

#[test]
fn output_schema_requires_status_enum() {
    let schema = output_schema();
    assert_eq!(
        schema["required"],
        serde_json::json!(["message", "send_to", "status"])
    );
    assert_eq!(
        schema["properties"]["status"]["enum"],
        serde_json::json!(["in_progress", "done", "error"])
    );
    assert_eq!(
        schema["properties"]["send_to"]["enum"],
        serde_json::json!(["user", "lead", "coder", "reviewer", "none"])
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
fn reviewer_prompt_requires_read_only_protocol() {
    let prompt = get_role("reviewer").unwrap().base_instructions;
    assert!(prompt.contains("MUST NOT modify files"));
    assert!(prompt.contains("MUST NOT act as the primary implementer"));
}

#[test]
fn prompt_authorizes_all_roles_for_any_subject_matter() {
    let prompt = get_role("coder").unwrap().base_instructions;
    assert!(
        prompt.contains("Every role is authorized to answer ANY user question or technical topic")
    );
    assert!(prompt.contains("web crawling"));
    assert!(prompt.contains("reverse engineering"));
    assert!(prompt.contains("server attack research"));
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
