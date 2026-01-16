//! Anthropic API client for cctakt
//!
//! Provides LLM integration for generating PR descriptions and summaries.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

/// Default model to use
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

/// Maximum tokens for response
pub const DEFAULT_MAX_TOKENS: u32 = 1024;

/// Anthropic API client
pub struct AnthropicClient {
    /// API key
    api_key: String,

    /// Model to use
    model: String,

    /// Max tokens for response
    max_tokens: u32,
}

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

/// Request body for messages API
#[derive(Debug, Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

/// Content block in response
#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

/// Response from messages API
#[derive(Debug, Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
    model: String,
    stop_reason: Option<String>,
    usage: Usage,
}

/// Token usage info
#[derive(Debug, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Result of a completion
#[derive(Debug)]
pub struct CompletionResult {
    /// The generated text
    pub text: String,

    /// Token usage
    pub usage: Usage,

    /// Stop reason
    pub stop_reason: Option<String>,
}

impl AnthropicClient {
    /// Create a new client
    ///
    /// API key is obtained from:
    /// 1. Provided api_key parameter
    /// 2. Environment variable `ANTHROPIC_API_KEY`
    pub fn new(api_key: Option<String>) -> Result<Self> {
        let key = api_key
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .ok_or_else(|| anyhow!("ANTHROPIC_API_KEY not set"))?;

        if key.is_empty() {
            return Err(anyhow!("ANTHROPIC_API_KEY is empty"));
        }

        Ok(Self {
            api_key: key,
            model: DEFAULT_MODEL.to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
        })
    }

    /// Create client with custom settings
    pub fn with_settings(api_key: String, model: String, max_tokens: u32) -> Self {
        Self {
            api_key,
            model,
            max_tokens,
        }
    }

    /// Set the model to use
    pub fn set_model(&mut self, model: &str) {
        self.model = model.to_string();
    }

    /// Set max tokens
    pub fn set_max_tokens(&mut self, max_tokens: u32) {
        self.max_tokens = max_tokens;
    }

    /// Send a simple message and get a response
    pub fn complete(&self, prompt: &str) -> Result<CompletionResult> {
        self.complete_with_system(prompt, None)
    }

    /// Send a message with a system prompt
    pub fn complete_with_system(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<CompletionResult> {
        let messages = vec![Message {
            role: Role::User,
            content: prompt.to_string(),
        }];

        self.send_messages(&messages, system)
    }

    /// Send multiple messages (for multi-turn conversations)
    pub fn send_messages(
        &self,
        messages: &[Message],
        system: Option<&str>,
    ) -> Result<CompletionResult> {
        let request = MessagesRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            messages: messages.to_vec(),
            system: system.map(String::from),
        };

        let response = ureq::post("https://api.anthropic.com/v1/messages")
            .set("Content-Type", "application/json")
            .set("x-api-key", &self.api_key)
            .set("anthropic-version", "2023-06-01")
            .send_json(&request)
            .context("Failed to send request to Anthropic API")?;

        if response.status() != 200 {
            return Err(anyhow!(
                "Anthropic API error: HTTP {}",
                response.status()
            ));
        }

        let response: MessagesResponse = response
            .into_json()
            .context("Failed to parse Anthropic API response")?;

        // Extract text from content blocks
        let text = response
            .content
            .iter()
            .filter_map(|block| {
                if block.content_type == "text" {
                    block.text.clone()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(CompletionResult {
            text,
            usage: response.usage,
            stop_reason: response.stop_reason,
        })
    }

    /// Generate a PR description based on issue and changes
    pub fn generate_pr_description(
        &self,
        issue_title: &str,
        issue_body: Option<&str>,
        commit_log: &str,
        diff_summary: &str,
    ) -> Result<String> {
        let system = "You are a helpful assistant that generates pull request descriptions.\n\
Generate a clear, concise PR description in markdown format.\n\
Include:\n\
- A brief summary of the changes\n\
- Key modifications made\n\
- Any notable implementation details\n\n\
Keep it professional and to the point. Do not include section headers - just write the content directly.\n\
Write in the same language as the issue (if Japanese, write in Japanese).";

        let prompt = format!(
            "Generate a pull request description for the following:\n\n\
=== Issue ===\n\
Title: {issue_title}\n\
{body_section}\n\n\
=== Commits ===\n\
{commit_log}\n\n\
=== Changes Summary ===\n\
{diff_summary}\n\n\
Generate a PR description:",
            body_section = issue_body
                .map(|b| format!("Body:\n{b}"))
                .unwrap_or_default(),
        );

        let result = self.complete_with_system(&prompt, Some(system))?;
        Ok(result.text)
    }

    /// Generate a commit message based on changes
    pub fn generate_commit_message(&self, diff: &str, context: Option<&str>) -> Result<String> {
        let system = "You are a helpful assistant that generates git commit messages.\n\
Generate a concise, conventional commit message.\n\
Format: <type>: <description>\n\
Types: feat, fix, docs, style, refactor, test, chore\n\
Keep the first line under 72 characters.\n\
Add a blank line and bullet points for details if needed.";

        let prompt = format!(
            "Generate a commit message for these changes:\n\n\
{context_section}\
=== Diff ===\n\
{diff}\n\n\
Generate commit message:",
            context_section = context
                .map(|c| format!("=== Context ===\n{c}\n\n"))
                .unwrap_or_default(),
        );

        let result = self.complete_with_system(&prompt, Some(system))?;
        Ok(result.text.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model() {
        assert_eq!(DEFAULT_MODEL, "claude-sonnet-4-20250514");
    }

    #[test]
    fn test_default_max_tokens() {
        assert_eq!(DEFAULT_MAX_TOKENS, 1024);
    }

    #[test]
    fn test_client_with_settings() {
        let client = AnthropicClient::with_settings(
            "test-key".to_string(),
            "claude-3-opus".to_string(),
            2048,
        );

        assert_eq!(client.model, "claude-3-opus");
        assert_eq!(client.max_tokens, 2048);
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message {
            role: Role::User,
            content: "Hello".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_assistant_role_serialization() {
        let msg = Message {
            role: Role::Assistant,
            content: "Hi there".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"assistant\""));
    }

    #[test]
    fn test_client_new_without_key() {
        // Clear env var for this test
        // SAFETY: This test is single-threaded and we're only removing a test env var
        unsafe { std::env::remove_var("ANTHROPIC_API_KEY") };
        let result = AnthropicClient::new(None);
        assert!(result.is_err());
    }

    #[test]
    fn test_client_new_with_key() {
        let client = AnthropicClient::new(Some("test-key".to_string())).unwrap();
        assert_eq!(client.api_key, "test-key");
        assert_eq!(client.model, DEFAULT_MODEL);
    }

    #[test]
    fn test_set_model() {
        let mut client = AnthropicClient::with_settings(
            "key".to_string(),
            "model1".to_string(),
            1000,
        );
        client.set_model("model2");
        assert_eq!(client.model, "model2");
    }

    #[test]
    fn test_set_max_tokens() {
        let mut client = AnthropicClient::with_settings(
            "key".to_string(),
            "model".to_string(),
            1000,
        );
        client.set_max_tokens(2000);
        assert_eq!(client.max_tokens, 2000);
    }

    #[test]
    fn test_usage_deserialize() {
        let json = r#"{"input_tokens": 100, "output_tokens": 50}"#;
        let usage: Usage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    #[ignore] // Run with: cargo test anthropic_integration -- --ignored
    fn test_complete_simple() {
        let client = AnthropicClient::new(None).unwrap();
        let result = client.complete("Say 'hello' and nothing else.").unwrap();

        assert!(result.text.to_lowercase().contains("hello"));
        assert!(result.usage.input_tokens > 0);
        assert!(result.usage.output_tokens > 0);
    }

    #[test]
    #[ignore]
    fn test_generate_pr_description() {
        let client = AnthropicClient::new(None).unwrap();
        let result = client
            .generate_pr_description(
                "Add user authentication",
                Some("Implement JWT-based authentication for the API"),
                "abc123 feat: add JWT middleware\ndef456 feat: add login endpoint",
                "5 files changed, 200 insertions(+), 10 deletions(-)",
            )
            .unwrap();

        assert!(!result.is_empty());
    }
}
