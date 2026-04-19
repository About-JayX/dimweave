use super::*;

const SAMPLE_BODY: &str = r#"{
  "data": [
    {
      "type": "model",
      "id": "claude-opus-4-7",
      "display_name": "Claude Opus 4.7",
      "capabilities": {
        "effort": {
          "low": { "supported": true },
          "medium": { "supported": true },
          "high": { "supported": true },
          "max": { "supported": true }
        }
      }
    },
    {
      "type": "model",
      "id": "claude-sonnet-4-6",
      "display_name": "Claude Sonnet 4.6",
      "capabilities": {
        "effort": {
          "low": { "supported": true },
          "medium": { "supported": true },
          "high": { "supported": true },
          "max": { "supported": false }
        }
      }
    },
    {
      "type": "model",
      "id": "claude-3-5-sonnet-latest",
      "display_name": "Legacy",
      "capabilities": {}
    }
  ]
}"#;

#[test]
fn parses_models_response_extracts_supported_efforts() {
    let models = parse_models(SAMPLE_BODY).unwrap();
    let opus = models.iter().find(|m| m.slug == "claude-opus-4-7").unwrap();
    assert_eq!(opus.display_name, "Claude Opus 4.7");
    assert_eq!(opus.supported_efforts, vec!["low", "medium", "high", "max"]);
}

#[test]
fn parses_models_response_filters_max_not_supported() {
    let models = parse_models(SAMPLE_BODY).unwrap();
    let sonnet = models.iter().find(|m| m.slug == "claude-sonnet-4-6").unwrap();
    assert_eq!(sonnet.supported_efforts, vec!["low", "medium", "high"]);
}

#[test]
fn drops_legacy_claude_3_models() {
    let models = parse_models(SAMPLE_BODY).unwrap();
    assert!(models.iter().all(|m| !m.slug.starts_with("claude-3")));
}

#[test]
fn drops_legacy_claude_2_models() {
    let body = r#"{"data":[{"type":"model","id":"claude-2.1","display_name":"Legacy","capabilities":{}}]}"#;
    let models = parse_models(body).unwrap();
    assert!(models.is_empty());
}

#[test]
fn returns_empty_effort_list_when_capability_missing() {
    let body = r#"{"data":[{"type":"model","id":"claude-opus-4-5","display_name":"X","capabilities":{}}]}"#;
    let models = parse_models(body).unwrap();
    assert_eq!(models[0].supported_efforts, Vec::<String>::new());
}

#[test]
fn malformed_json_returns_err() {
    let err = parse_models("not json").unwrap_err();
    assert!(err.starts_with("parse models json"));
}

#[test]
fn xhigh_effort_picked_up_when_api_adds_it() {
    let body = r#"{"data":[{"type":"model","id":"claude-opus-4-7","display_name":"Opus 4.7","capabilities":{"effort":{"low":{"supported":true},"xhigh":{"supported":true},"max":{"supported":true}}}}]}"#;
    let models = parse_models(body).unwrap();
    assert_eq!(models[0].supported_efforts, vec!["low", "xhigh", "max"]);
}
