use serde_json::json;

#[test]
fn stream_event_content_block_delta_structure() {
    let raw = json!({
        "type": "stream_event",
        "event": {
            "type": "content_block_delta",
            "delta": {
                "type": "text_delta",
                "text": "Hello world"
            }
        }
    });
    let event_type = raw["type"].as_str().unwrap();
    assert_eq!(event_type, "stream_event");
    let inner_type = raw["event"]["type"].as_str().unwrap();
    assert_eq!(inner_type, "content_block_delta");
    let delta_text = raw["event"]["delta"]["text"].as_str().unwrap();
    assert_eq!(delta_text, "Hello world");
}

#[test]
fn stream_event_content_block_start_structure() {
    let raw = json!({
        "type": "stream_event",
        "event": {
            "type": "content_block_start",
            "content_block": {"type": "text"}
        }
    });
    let block_type = raw["event"]["content_block"]["type"].as_str().unwrap();
    assert_eq!(block_type, "text");
}
