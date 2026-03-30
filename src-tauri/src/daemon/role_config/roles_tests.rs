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
