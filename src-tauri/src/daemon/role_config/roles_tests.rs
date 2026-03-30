use super::{output_schema, ROLE_CODER};

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
    let prompt = ROLE_CODER.base_instructions;
    assert!(prompt.contains("send_to = \"lead\" is the default"));
    assert!(prompt.contains("may send_to = \"user\" only when the user explicitly names your role"));
}

#[test]
fn prompt_documents_get_status_structured_response() {
    let prompt = ROLE_CODER.base_instructions;
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
