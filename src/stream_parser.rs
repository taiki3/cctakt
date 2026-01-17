//! Stream JSON parser for Claude Code non-interactive mode output
//!
//! Parses the JSONL (newline-delimited JSON) output from `claude -p --output-format stream-json`.
//!
//! # Output Format
//!
//! Claude Code's stream-json output consists of multiple event types:
//! - `system`: Session initialization, model info
//! - `assistant`: Messages from the assistant
//! - `result`: Final result when session ends

use serde::{Deserialize, Serialize};

/// Top-level stream event from Claude Code
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StreamEvent {
    /// System events (init, model info)
    System {
        subtype: String,
        #[serde(default)]
        session_id: Option<String>,
        #[serde(default)]
        model: Option<String>,
    },

    /// Assistant message
    Assistant {
        message: AssistantMessage,
        #[serde(default)]
        session_id: Option<String>,
    },

    /// User message (echo back)
    User {
        message: UserMessage,
        #[serde(default)]
        session_id: Option<String>,
    },

    /// Final result
    Result {
        subtype: String,
        session_id: String,
        #[serde(default)]
        result: Option<String>,
        #[serde(default)]
        cost_usd: Option<f64>,
        #[serde(default)]
        duration_ms: Option<u64>,
        #[serde(default)]
        duration_api_ms: Option<u64>,
        #[serde(default)]
        is_error: Option<bool>,
        #[serde(default)]
        num_turns: Option<u32>,
    },
}

/// Assistant message content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub content: Vec<ContentBlock>,
    #[serde(default)]
    pub stop_reason: Option<String>,
}

/// User message content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub content: Vec<ContentBlock>,
}

/// Content block in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content
    Text {
        text: String,
    },

    /// Tool use request
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },

    /// Tool result
    ToolResult {
        tool_use_id: String,
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        is_error: Option<bool>,
    },
}

/// Parse a single JSONL line
pub fn parse_line(line: &str) -> Option<StreamEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

/// Check if the event indicates completion
pub fn is_completed(event: &StreamEvent) -> bool {
    matches!(event, StreamEvent::Result { subtype, .. } if subtype == "success" || subtype == "error")
}

/// Check if the event indicates an error
pub fn is_error(event: &StreamEvent) -> bool {
    match event {
        StreamEvent::Result { is_error: Some(true), .. } => true,
        StreamEvent::Result { subtype, .. } if subtype == "error" => true,
        _ => false,
    }
}

/// Extract text content from an assistant message
pub fn extract_text(message: &AssistantMessage) -> String {
    message
        .content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Stream parser state machine
#[derive(Debug, Default)]
pub struct StreamParser {
    /// Session ID once received
    pub session_id: Option<String>,
    /// All received events
    pub events: Vec<StreamEvent>,
    /// Buffer for incomplete lines
    buffer: String,
    /// Whether the session has completed
    pub completed: bool,
    /// Error message if session ended with error
    pub error: Option<String>,
    /// Final result text
    pub result: Option<String>,
}

impl StreamParser {
    /// Create a new parser
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed data to the parser
    ///
    /// Returns newly parsed events
    pub fn feed(&mut self, data: &str) -> Vec<StreamEvent> {
        self.buffer.push_str(data);
        let mut events = Vec::new();

        // Process complete lines
        while let Some(newline_pos) = self.buffer.find('\n') {
            let line = self.buffer[..newline_pos].to_string();
            self.buffer = self.buffer[newline_pos + 1..].to_string();

            if let Some(event) = parse_line(&line) {
                // Update session state
                match &event {
                    StreamEvent::System { session_id: Some(id), .. } => {
                        self.session_id = Some(id.clone());
                    }
                    StreamEvent::Result { result, is_error: Some(true), .. } => {
                        self.completed = true;
                        self.error = result.clone();
                    }
                    StreamEvent::Result { result, subtype, .. } if subtype == "success" => {
                        self.completed = true;
                        self.result = result.clone();
                    }
                    StreamEvent::Result { result, subtype, .. } if subtype == "error" => {
                        self.completed = true;
                        self.error = result.clone();
                    }
                    _ => {}
                }

                self.events.push(event.clone());
                events.push(event);
            }
        }

        events
    }

    /// Get the last assistant message text
    pub fn last_assistant_text(&self) -> Option<String> {
        self.events
            .iter()
            .rev()
            .find_map(|event| match event {
                StreamEvent::Assistant { message, .. } => Some(extract_text(message)),
                _ => None,
            })
    }

    /// Get all tool uses from the session
    pub fn tool_uses(&self) -> Vec<(&str, &str)> {
        self.events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::Assistant { message, .. } => Some(message),
                _ => None,
            })
            .flat_map(|msg| &msg.content)
            .filter_map(|block| match block {
                ContentBlock::ToolUse { id, name, .. } => Some((id.as_str(), name.as_str())),
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== parse_line tests ====================

    #[test]
    fn test_parse_system_event() {
        let line = r#"{"type":"system","subtype":"init","session_id":"abc123"}"#;
        let event = parse_line(line).unwrap();
        match event {
            StreamEvent::System { subtype, session_id, .. } => {
                assert_eq!(subtype, "init");
                assert_eq!(session_id, Some("abc123".to_string()));
            }
            _ => panic!("Expected System event"),
        }
    }

    #[test]
    fn test_parse_assistant_event() {
        let line = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Hello!"}]}}"#;
        let event = parse_line(line).unwrap();
        match event {
            StreamEvent::Assistant { message, .. } => {
                assert_eq!(message.role, "assistant");
                assert_eq!(message.content.len(), 1);
                match &message.content[0] {
                    ContentBlock::Text { text } => assert_eq!(text, "Hello!"),
                    _ => panic!("Expected Text block"),
                }
            }
            _ => panic!("Expected Assistant event"),
        }
    }

    #[test]
    fn test_parse_result_success() {
        let line = r#"{"type":"result","subtype":"success","session_id":"abc123","result":"Done","cost_usd":0.01}"#;
        let event = parse_line(line).unwrap();
        match event {
            StreamEvent::Result { subtype, session_id, result, cost_usd, .. } => {
                assert_eq!(subtype, "success");
                assert_eq!(session_id, "abc123");
                assert_eq!(result, Some("Done".to_string()));
                assert_eq!(cost_usd, Some(0.01));
            }
            _ => panic!("Expected Result event"),
        }
    }

    #[test]
    fn test_parse_result_error() {
        let line = r#"{"type":"result","subtype":"error","session_id":"abc123","is_error":true,"result":"Failed"}"#;
        let event = parse_line(line).unwrap();
        assert!(is_error(&event));
        assert!(is_completed(&event));
    }

    #[test]
    fn test_parse_empty_line() {
        assert!(parse_line("").is_none());
        assert!(parse_line("   ").is_none());
    }

    #[test]
    fn test_parse_invalid_json() {
        assert!(parse_line("not json").is_none());
        assert!(parse_line("{invalid}").is_none());
    }

    // ==================== is_completed tests ====================

    #[test]
    fn test_is_completed_success() {
        let event = StreamEvent::Result {
            subtype: "success".to_string(),
            session_id: "123".to_string(),
            result: Some("Done".to_string()),
            cost_usd: None,
            duration_ms: None,
            duration_api_ms: None,
            is_error: None,
            num_turns: None,
        };
        assert!(is_completed(&event));
    }

    #[test]
    fn test_is_completed_error() {
        let event = StreamEvent::Result {
            subtype: "error".to_string(),
            session_id: "123".to_string(),
            result: Some("Failed".to_string()),
            cost_usd: None,
            duration_ms: None,
            duration_api_ms: None,
            is_error: Some(true),
            num_turns: None,
        };
        assert!(is_completed(&event));
    }

    #[test]
    fn test_is_not_completed_system() {
        let event = StreamEvent::System {
            subtype: "init".to_string(),
            session_id: Some("123".to_string()),
            model: None,
        };
        assert!(!is_completed(&event));
    }

    // ==================== extract_text tests ====================

    #[test]
    fn test_extract_text_single() {
        let message = AssistantMessage {
            id: None,
            role: "assistant".to_string(),
            content: vec![ContentBlock::Text {
                text: "Hello world".to_string(),
            }],
            stop_reason: None,
        };
        assert_eq!(extract_text(&message), "Hello world");
    }

    #[test]
    fn test_extract_text_multiple() {
        let message = AssistantMessage {
            id: None,
            role: "assistant".to_string(),
            content: vec![
                ContentBlock::Text {
                    text: "First".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "tool1".to_string(),
                    name: "Bash".to_string(),
                    input: serde_json::json!({}),
                },
                ContentBlock::Text {
                    text: "Second".to_string(),
                },
            ],
            stop_reason: None,
        };
        assert_eq!(extract_text(&message), "First\nSecond");
    }

    #[test]
    fn test_extract_text_no_text() {
        let message = AssistantMessage {
            id: None,
            role: "assistant".to_string(),
            content: vec![ContentBlock::ToolUse {
                id: "tool1".to_string(),
                name: "Read".to_string(),
                input: serde_json::json!({"path": "test.txt"}),
            }],
            stop_reason: None,
        };
        assert_eq!(extract_text(&message), "");
    }

    // ==================== StreamParser tests ====================

    #[test]
    fn test_stream_parser_new() {
        let parser = StreamParser::new();
        assert!(parser.session_id.is_none());
        assert!(parser.events.is_empty());
        assert!(!parser.completed);
    }

    #[test]
    fn test_stream_parser_feed_single_line() {
        let mut parser = StreamParser::new();
        let events = parser.feed(r#"{"type":"system","subtype":"init","session_id":"abc123"}
"#);
        assert_eq!(events.len(), 1);
        assert_eq!(parser.session_id, Some("abc123".to_string()));
    }

    #[test]
    fn test_stream_parser_feed_multiple_lines() {
        let mut parser = StreamParser::new();
        let input = r#"{"type":"system","subtype":"init","session_id":"abc"}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Hi"}]}}
{"type":"result","subtype":"success","session_id":"abc","result":"Done"}
"#;
        let events = parser.feed(input);
        assert_eq!(events.len(), 3);
        assert!(parser.completed);
        assert_eq!(parser.result, Some("Done".to_string()));
    }

    #[test]
    fn test_stream_parser_feed_partial() {
        let mut parser = StreamParser::new();

        // Feed partial line (split at a safe point - before newline)
        let events1 = parser.feed(r#"{"type":"system","subtype":"init","session_id":"abc123"}"#);
        assert!(events1.is_empty()); // No newline yet, so no complete line

        // Complete the line with a newline
        let events2 = parser.feed("\n");
        assert_eq!(events2.len(), 1);
        assert_eq!(parser.session_id, Some("abc123".to_string()));
    }

    #[test]
    fn test_stream_parser_error_result() {
        let mut parser = StreamParser::new();
        parser.feed(r#"{"type":"result","subtype":"error","session_id":"abc","is_error":true,"result":"Something went wrong"}
"#);
        assert!(parser.completed);
        assert_eq!(parser.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_stream_parser_last_assistant_text() {
        let mut parser = StreamParser::new();
        parser.feed(r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"First message"}]}}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Second message"}]}}
"#);
        assert_eq!(parser.last_assistant_text(), Some("Second message".to_string()));
    }

    #[test]
    fn test_stream_parser_tool_uses() {
        let mut parser = StreamParser::new();
        parser.feed(r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"tool1","name":"Read","input":{"path":"test.txt"}}]}}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"tool2","name":"Write","input":{}}]}}
"#);
        let tool_uses = parser.tool_uses();
        assert_eq!(tool_uses.len(), 2);
        assert_eq!(tool_uses[0], ("tool1", "Read"));
        assert_eq!(tool_uses[1], ("tool2", "Write"));
    }

    // ==================== ContentBlock tests ====================

    #[test]
    fn test_content_block_text_serialize() {
        let block = ContentBlock::Text {
            text: "Hello".to_string(),
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello\""));
    }

    #[test]
    fn test_content_block_tool_use_serialize() {
        let block = ContentBlock::ToolUse {
            id: "abc".to_string(),
            name: "Bash".to_string(),
            input: serde_json::json!({"command": "ls"}),
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"tool_use\""));
        assert!(json.contains("\"name\":\"Bash\""));
    }

    #[test]
    fn test_content_block_tool_result_serialize() {
        let block = ContentBlock::ToolResult {
            tool_use_id: "abc".to_string(),
            content: Some("output".to_string()),
            is_error: Some(false),
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"tool_result\""));
        assert!(json.contains("\"tool_use_id\":\"abc\""));
    }

    // ==================== User message tests ====================

    #[test]
    fn test_parse_user_event() {
        let line = r#"{"type":"user","message":{"role":"user","content":[{"type":"text","text":"Hello"}]}}"#;
        let event = parse_line(line).unwrap();
        match event {
            StreamEvent::User { message, .. } => {
                assert_eq!(message.role, "user");
            }
            _ => panic!("Expected User event"),
        }
    }

    // ==================== Edge cases ====================

    #[test]
    fn test_parse_minimal_result() {
        let line = r#"{"type":"result","subtype":"success","session_id":"abc"}"#;
        let event = parse_line(line).unwrap();
        assert!(is_completed(&event));
    }

    #[test]
    fn test_stream_parser_empty_feed() {
        let mut parser = StreamParser::new();
        let events = parser.feed("");
        assert!(events.is_empty());
    }

    #[test]
    fn test_stream_parser_whitespace_only() {
        let mut parser = StreamParser::new();
        let events = parser.feed("   \n   \n");
        assert!(events.is_empty());
    }
}
