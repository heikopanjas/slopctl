//! LLM provider abstraction for AI-assisted operations
//!
//! Supports OpenAI, Anthropic, Ollama, and Mistral as backend providers.
//! API keys are read from environment variables; Ollama requires no key.

use std::{
    env,
    io::{BufRead, BufReader},
    time::Duration
};

use serde::{Deserialize, Serialize};

use crate::Result;

/// Supported LLM providers
#[derive(Debug, Clone, PartialEq)]
pub enum Provider
{
    OpenAi,
    Anthropic,
    Ollama,
    Mistral
}

impl Provider
{
    /// Parses a provider name string into a `Provider` enum value
    ///
    /// # Errors
    ///
    /// Returns an error if the provider name is not recognized
    pub fn from_name(name: &str) -> Result<Self>
    {
        match name.to_lowercase().as_str()
        {
            | "openai" => Ok(Self::OpenAi),
            | "anthropic" => Ok(Self::Anthropic),
            | "ollama" => Ok(Self::Ollama),
            | "mistral" => Ok(Self::Mistral),
            | _ => Err(anyhow::anyhow!("Unknown provider: {}\nSupported: openai, anthropic, ollama, mistral", name))
        }
    }

    /// Returns the environment variable name that holds the API key for this provider
    fn api_key_env_var(&self) -> Option<&'static str>
    {
        match self
        {
            | Self::OpenAi => Some("OPENAI_API_KEY"),
            | Self::Anthropic => Some("ANTHROPIC_API_KEY"),
            | Self::Ollama => None,
            | Self::Mistral => Some("MISTRAL_API_KEY")
        }
    }

    /// Detects a provider by checking which API key environment variables are set
    ///
    /// Checks in order: Anthropic, OpenAI, Mistral. Returns the first provider
    /// whose API key is present in the environment. Ollama is not auto-detected
    /// because it requires no key (would always match).
    pub fn detect_from_env() -> Option<Self>
    {
        let candidates = [Self::Anthropic, Self::OpenAi, Self::Mistral];

        for provider in candidates
        {
            if let Some(env_var) = provider.api_key_env_var() &&
                env::var(env_var).is_ok() == true
            {
                return Some(provider);
            }
        }

        None
    }

    /// Returns the provider name as a lowercase string
    pub fn name(&self) -> &'static str
    {
        match self
        {
            | Self::OpenAi => "openai",
            | Self::Anthropic => "anthropic",
            | Self::Ollama => "ollama",
            | Self::Mistral => "mistral"
        }
    }

    /// Returns the default model for this provider
    pub fn default_model(&self) -> &'static str
    {
        match self
        {
            | Self::OpenAi => "gpt-4o",
            | Self::Anthropic => "claude-sonnet-4-6",
            | Self::Ollama => "llama3",
            | Self::Mistral => "mistral-large-latest"
        }
    }

    /// Returns the base API endpoint URL for chat completions
    fn endpoint(&self) -> &'static str
    {
        match self
        {
            | Self::OpenAi => "https://api.openai.com/v1/chat/completions",
            | Self::Anthropic => "https://api.anthropic.com/v1/messages",
            | Self::Ollama => "http://localhost:11434/api/chat",
            | Self::Mistral => "https://api.mistral.ai/v1/chat/completions"
        }
    }

    /// Returns the API endpoint URL for listing available models
    pub fn models_endpoint(&self) -> &'static str
    {
        match self
        {
            | Self::OpenAi => "https://api.openai.com/v1/models",
            | Self::Anthropic => "https://api.anthropic.com/v1/models",
            | Self::Ollama => "http://localhost:11434/api/tags",
            | Self::Mistral => "https://api.mistral.ai/v1/models"
        }
    }
}

/// A message in the chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage
{
    pub role:    String,
    pub content: String
}

/// Response from a chat completion API call
#[derive(Debug)]
pub struct ChatResponse
{
    /// The assistant's response text
    pub content:       String,
    /// Number of input/prompt tokens consumed
    pub input_tokens:  Option<u64>,
    /// Number of output/completion tokens generated
    pub output_tokens: Option<u64>,
    /// Why the model stopped generating (e.g. "end_turn", "stop", "max_tokens", "length")
    pub stop_reason:   Option<String>
}

/// Client for making LLM API calls
pub struct LlmClient
{
    provider: Provider,
    model:    String,
    api_key:  Option<String>,
    http:     reqwest::blocking::Client
}

impl std::fmt::Debug for LlmClient
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        f.debug_struct("LlmClient").field("provider", &self.provider).field("model", &self.model).finish()
    }
}

impl LlmClient
{
    /// Creates a new LLM client for the given provider and model
    ///
    /// Reads the API key from the appropriate environment variable.
    /// Ollama does not require an API key.
    ///
    /// # Arguments
    ///
    /// * `provider` - The LLM provider to use
    /// * `model` - Optional model override; uses provider default if None
    ///
    /// # Errors
    ///
    /// Returns an error if a required API key is not set in the environment
    pub fn new(provider: Provider, model: Option<&str>) -> Result<Self>
    {
        let api_key = if let Some(env_var) = provider.api_key_env_var()
        {
            let key = env::var(env_var).map_err(|_| anyhow::anyhow!("{} environment variable not set\nSet it with: export {}=<your-key>", env_var, env_var))?;
            Some(key)
        }
        else
        {
            None
        };

        let model_name = model.unwrap_or(provider.default_model()).to_string();

        let http = reqwest::blocking::Client::builder().user_agent("slopctl").connect_timeout(Duration::from_secs(30)).timeout(Duration::from_secs(300)).build()?;

        Ok(Self { provider, model: model_name, api_key, http })
    }

    /// Sends a chat completion request and returns the response with usage metadata
    ///
    /// Convenience wrapper around `chat_stream` with a no-op callback.
    pub fn chat(&self, messages: &[ChatMessage]) -> Result<ChatResponse>
    {
        self.chat_stream(messages, |_| {})
    }

    /// Sends a streaming chat completion request, invoking `on_chunk` for each content token
    ///
    /// Returns the accumulated response with full content and usage metadata.
    /// The `on_chunk` callback receives each content fragment as it arrives.
    pub fn chat_stream(&self, messages: &[ChatMessage], on_chunk: impl FnMut(&str)) -> Result<ChatResponse>
    {
        match self.provider
        {
            | Provider::Anthropic => self.stream_anthropic(messages, on_chunk),
            | _ => self.stream_openai_compatible(messages, on_chunk)
        }
    }

    /// Returns the provider name for display
    pub fn provider_name(&self) -> &str
    {
        self.provider.name()
    }

    /// Returns the model name for display
    pub fn model_name(&self) -> &str
    {
        &self.model
    }

    /// Queries the provider's API for available models
    ///
    /// Returns a sorted list of model identifier strings.
    /// Each provider has a different response format:
    /// - OpenAI / Mistral: `data[].id`
    /// - Anthropic: `data[].id` (with `x-api-key` auth and `anthropic-version` header)
    /// - Ollama: `models[].name`
    ///
    /// # Errors
    ///
    /// Returns an error if the API call fails or the response cannot be parsed
    pub fn list_models(&self) -> Result<Vec<String>>
    {
        match self.provider
        {
            | Provider::Anthropic => self.list_models_anthropic(),
            | Provider::Ollama => self.list_models_ollama(),
            | _ => self.list_models_openai_compatible()
        }
    }

    /// Lists models via the OpenAI-compatible `/v1/models` endpoint (OpenAI, Mistral)
    fn list_models_openai_compatible(&self) -> Result<Vec<String>>
    {
        let mut request = self.http.get(self.provider.models_endpoint());

        if let Some(ref key) = self.api_key
        {
            request = request.bearer_auth(key);
        }

        let response = request.send()?;
        let status = response.status();

        if status.is_success() == false
        {
            let error_body = response.text().unwrap_or_default();
            return Err(anyhow::anyhow!("{} API error ({}): {}", self.provider_name(), status, error_body));
        }

        let json: serde_json::Value = response.json()?;
        let mut models: Vec<String> =
            json["data"].as_array().map(|arr| arr.iter().filter_map(|m| m["id"].as_str().map(|s| s.to_string())).collect()).unwrap_or_default();

        models.sort();
        Ok(models)
    }

    /// Lists models via the Anthropic `/v1/models` endpoint
    fn list_models_anthropic(&self) -> Result<Vec<String>>
    {
        let key = self.api_key.as_deref().ok_or_else(|| anyhow::anyhow!("Anthropic API key not set"))?;

        let response =
            self.http.get(self.provider.models_endpoint()).query(&[("limit", "1000")]).header("x-api-key", key).header("anthropic-version", "2023-06-01").send()?;

        let status = response.status();

        if status.is_success() == false
        {
            let error_body = response.text().unwrap_or_default();
            return Err(anyhow::anyhow!("Anthropic API error ({}): {}", status, error_body));
        }

        let json: serde_json::Value = response.json()?;
        let mut models: Vec<String> =
            json["data"].as_array().map(|arr| arr.iter().filter_map(|m| m["id"].as_str().map(|s| s.to_string())).collect()).unwrap_or_default();

        models.sort();
        Ok(models)
    }

    /// Lists models via the Ollama `/api/tags` endpoint
    fn list_models_ollama(&self) -> Result<Vec<String>>
    {
        let response = self.http.get(self.provider.models_endpoint()).send()?;
        let status = response.status();

        if status.is_success() == false
        {
            let error_body = response.text().unwrap_or_default();
            return Err(anyhow::anyhow!("Ollama API error ({}): {}", status, error_body));
        }

        let json: serde_json::Value = response.json()?;
        let mut models: Vec<String> =
            json["models"].as_array().map(|arr| arr.iter().filter_map(|m| m["name"].as_str().map(|s| s.to_string())).collect()).unwrap_or_default();

        models.sort();
        Ok(models)
    }

    /// Streaming OpenAI-compatible chat completion (OpenAI, Ollama, Mistral)
    fn stream_openai_compatible(&self, messages: &[ChatMessage], mut on_chunk: impl FnMut(&str)) -> Result<ChatResponse>
    {
        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "temperature": 0.0,
            "stream": true
        });

        if self.provider == Provider::Ollama
        {
            body["max_tokens"] = serde_json::json!(32768);
        }
        else
        {
            body["max_completion_tokens"] = serde_json::json!(32768);
            body["stream_options"] = serde_json::json!({"include_usage": true});
        }

        let mut request = self.http.post(self.provider.endpoint()).json(&body);

        if let Some(ref key) = self.api_key
        {
            request = request.bearer_auth(key);
        }

        let response = request.send()?;
        let status = response.status();

        if status.is_success() == false
        {
            let error_body = response.text().unwrap_or_default();
            return Err(anyhow::anyhow!("{} API error ({}): {}", self.provider_name(), status, error_body));
        }

        let mut content = String::new();
        let mut input_tokens: Option<u64> = None;
        let mut output_tokens: Option<u64> = None;
        let mut stop_reason: Option<String> = None;

        if self.provider == Provider::Ollama
        {
            let reader = BufReader::new(response);
            for line in reader.lines()
            {
                let line = line?;
                if line.is_empty() == true
                {
                    continue;
                }
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line)
                {
                    if let Some(text) = json["message"]["content"].as_str()
                    {
                        content.push_str(text);
                        on_chunk(text);
                    }
                    if json["done"].as_bool() == Some(true)
                    {
                        input_tokens = json["prompt_eval_count"].as_u64();
                        output_tokens = json["eval_count"].as_u64();
                        stop_reason = json["done_reason"].as_str().map(|s| s.to_string());
                    }
                }
            }
        }
        else
        {
            let reader = BufReader::new(response);
            for line in reader.lines()
            {
                let line = line?;
                if line.starts_with("data: ") == false
                {
                    continue;
                }
                let data = &line[6..];
                if data == "[DONE]"
                {
                    break;
                }
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data)
                {
                    if let Some(text) = json["choices"][0]["delta"]["content"].as_str()
                    {
                        content.push_str(text);
                        on_chunk(text);
                    }
                    if let Some(reason) = json["choices"][0]["finish_reason"].as_str()
                    {
                        stop_reason = Some(reason.to_string());
                    }
                    if let Some(usage) = json["usage"].as_object()
                    {
                        input_tokens = usage.get("prompt_tokens").and_then(|v| v.as_u64());
                        output_tokens = usage.get("completion_tokens").and_then(|v| v.as_u64());
                    }
                }
            }
        }

        Ok(ChatResponse { content, input_tokens, output_tokens, stop_reason })
    }

    /// Streaming Anthropic Messages API
    fn stream_anthropic(&self, messages: &[ChatMessage], mut on_chunk: impl FnMut(&str)) -> Result<ChatResponse>
    {
        let system_msg = messages.iter().find(|m| m.role == "system").map(|m| m.content.as_str()).unwrap_or("");

        let api_messages: Vec<serde_json::Value> =
            messages.iter().filter(|m| m.role != "system").map(|m| serde_json::json!({"role": m.role, "content": m.content})).collect();

        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 32768,
            "system": system_msg,
            "messages": api_messages,
            "stream": true
        });

        let key = self.api_key.as_deref().ok_or_else(|| anyhow::anyhow!("Anthropic API key not set"))?;

        let response = self
            .http
            .post(self.provider.endpoint())
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()?;

        let status = response.status();

        if status.is_success() == false
        {
            let error_body = response.text().unwrap_or_default();
            return Err(anyhow::anyhow!("Anthropic API error ({}): {}", status, error_body));
        }

        let mut content = String::new();
        let mut input_tokens: Option<u64> = None;
        let mut output_tokens: Option<u64> = None;
        let mut stop_reason: Option<String> = None;

        let reader = BufReader::new(response);
        for line in reader.lines()
        {
            let line = line?;
            if line.starts_with("data: ") == false
            {
                continue;
            }
            let data = &line[6..];
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data)
            {
                let event_type = json["type"].as_str().unwrap_or("");
                match event_type
                {
                    | "message_start" =>
                    {
                        input_tokens = json["message"]["usage"]["input_tokens"].as_u64();
                    }
                    | "content_block_delta" =>
                    {
                        if let Some(text) = json["delta"]["text"].as_str()
                        {
                            content.push_str(text);
                            on_chunk(text);
                        }
                    }
                    | "message_delta" =>
                    {
                        output_tokens = json["delta"]["usage"]["output_tokens"].as_u64().or(json["usage"]["output_tokens"].as_u64());
                        stop_reason = json["delta"]["stop_reason"].as_str().map(|s| s.to_string());
                    }
                    | _ =>
                    {}
                }
            }
        }

        Ok(ChatResponse { content, input_tokens, output_tokens, stop_reason })
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_provider_from_name_valid()
    {
        assert_eq!(Provider::from_name("openai").expect("parse"), Provider::OpenAi);
        assert_eq!(Provider::from_name("OpenAI").expect("parse"), Provider::OpenAi);
        assert_eq!(Provider::from_name("anthropic").expect("parse"), Provider::Anthropic);
        assert_eq!(Provider::from_name("ollama").expect("parse"), Provider::Ollama);
        assert_eq!(Provider::from_name("mistral").expect("parse"), Provider::Mistral);
        assert_eq!(Provider::from_name("MISTRAL").expect("parse"), Provider::Mistral);
    }

    #[test]
    fn test_provider_from_name_invalid()
    {
        let err = Provider::from_name("grok").unwrap_err();
        assert!(err.to_string().contains("Unknown provider") == true);
    }

    #[test]
    fn test_provider_default_model()
    {
        assert_eq!(Provider::OpenAi.default_model(), "gpt-4o");
        assert_eq!(Provider::Anthropic.default_model(), "claude-sonnet-4-6");
        assert_eq!(Provider::Ollama.default_model(), "llama3");
        assert_eq!(Provider::Mistral.default_model(), "mistral-large-latest");
    }

    #[test]
    fn test_provider_api_key_env_var()
    {
        assert_eq!(Provider::OpenAi.api_key_env_var(), Some("OPENAI_API_KEY"));
        assert_eq!(Provider::Anthropic.api_key_env_var(), Some("ANTHROPIC_API_KEY"));
        assert_eq!(Provider::Ollama.api_key_env_var(), None);
        assert_eq!(Provider::Mistral.api_key_env_var(), Some("MISTRAL_API_KEY"));
    }

    #[test]
    fn test_provider_endpoint()
    {
        assert!(Provider::OpenAi.endpoint().contains("openai.com") == true);
        assert!(Provider::Anthropic.endpoint().contains("anthropic.com") == true);
        assert!(Provider::Ollama.endpoint().contains("localhost") == true);
        assert!(Provider::Mistral.endpoint().contains("mistral.ai") == true);
    }

    #[test]
    fn test_chat_message_serde() -> anyhow::Result<()>
    {
        let msg = ChatMessage { role: "user".to_string(), content: "hello".to_string() };
        let json = serde_json::to_string(&msg)?;
        let parsed: ChatMessage = serde_json::from_str(&json)?;
        assert_eq!(parsed.role, "user");
        assert_eq!(parsed.content, "hello");
        Ok(())
    }

    #[test]
    fn test_llm_client_new_ollama_no_key_required() -> anyhow::Result<()>
    {
        let client = LlmClient::new(Provider::Ollama, None)?;
        assert_eq!(client.provider_name(), "ollama");
        assert_eq!(client.model_name(), "llama3");
        Ok(())
    }

    #[test]
    fn test_llm_client_new_with_model_override() -> anyhow::Result<()>
    {
        let client = LlmClient::new(Provider::Ollama, Some("codellama"))?;
        assert_eq!(client.model_name(), "codellama");
        Ok(())
    }

    #[test]
    fn test_llm_client_missing_api_key()
    {
        let _lock = ENV_LOCK.lock().expect("env lock");
        let saved = env::var("OPENAI_API_KEY").ok();
        unsafe { env::remove_var("OPENAI_API_KEY") };

        let result = LlmClient::new(Provider::OpenAi, None);
        assert!(result.is_err() == true);
        assert!(result.unwrap_err().to_string().contains("OPENAI_API_KEY") == true);

        if let Some(key) = saved
        {
            unsafe { env::set_var("OPENAI_API_KEY", key) };
        }
    }

    #[test]
    fn test_provider_name()
    {
        assert_eq!(Provider::OpenAi.name(), "openai");
        assert_eq!(Provider::Anthropic.name(), "anthropic");
        assert_eq!(Provider::Ollama.name(), "ollama");
        assert_eq!(Provider::Mistral.name(), "mistral");
    }

    #[test]
    fn test_detect_from_env_anthropic()
    {
        let _lock = ENV_LOCK.lock().expect("env lock");
        let saved_a = env::var("ANTHROPIC_API_KEY").ok();
        let saved_o = env::var("OPENAI_API_KEY").ok();
        let saved_m = env::var("MISTRAL_API_KEY").ok();

        unsafe { env::set_var("ANTHROPIC_API_KEY", "test-key") };
        unsafe { env::remove_var("OPENAI_API_KEY") };
        unsafe { env::remove_var("MISTRAL_API_KEY") };

        let detected = Provider::detect_from_env();
        assert_eq!(detected, Some(Provider::Anthropic));

        // Restore
        unsafe { env::remove_var("ANTHROPIC_API_KEY") };
        if let Some(k) = saved_a
        {
            unsafe { env::set_var("ANTHROPIC_API_KEY", k) };
        }
        if let Some(k) = saved_o
        {
            unsafe { env::set_var("OPENAI_API_KEY", k) };
        }
        if let Some(k) = saved_m
        {
            unsafe { env::set_var("MISTRAL_API_KEY", k) };
        }
    }

    #[test]
    fn test_detect_from_env_openai()
    {
        let _lock = ENV_LOCK.lock().expect("env lock");
        let saved_a = env::var("ANTHROPIC_API_KEY").ok();
        let saved_o = env::var("OPENAI_API_KEY").ok();
        let saved_m = env::var("MISTRAL_API_KEY").ok();

        unsafe { env::remove_var("ANTHROPIC_API_KEY") };
        unsafe { env::set_var("OPENAI_API_KEY", "test-key") };
        unsafe { env::remove_var("MISTRAL_API_KEY") };

        let detected = Provider::detect_from_env();
        assert_eq!(detected, Some(Provider::OpenAi));

        // Restore
        unsafe { env::remove_var("OPENAI_API_KEY") };
        if let Some(k) = saved_a
        {
            unsafe { env::set_var("ANTHROPIC_API_KEY", k) };
        }
        if let Some(k) = saved_o
        {
            unsafe { env::set_var("OPENAI_API_KEY", k) };
        }
        if let Some(k) = saved_m
        {
            unsafe { env::set_var("MISTRAL_API_KEY", k) };
        }
    }

    #[test]
    fn test_detect_from_env_none()
    {
        let _lock = ENV_LOCK.lock().expect("env lock");
        let saved_a = env::var("ANTHROPIC_API_KEY").ok();
        let saved_o = env::var("OPENAI_API_KEY").ok();
        let saved_m = env::var("MISTRAL_API_KEY").ok();

        unsafe { env::remove_var("ANTHROPIC_API_KEY") };
        unsafe { env::remove_var("OPENAI_API_KEY") };
        unsafe { env::remove_var("MISTRAL_API_KEY") };

        let detected = Provider::detect_from_env();
        assert_eq!(detected, None);

        // Restore
        if let Some(k) = saved_a
        {
            unsafe { env::set_var("ANTHROPIC_API_KEY", k) };
        }
        if let Some(k) = saved_o
        {
            unsafe { env::set_var("OPENAI_API_KEY", k) };
        }
        if let Some(k) = saved_m
        {
            unsafe { env::set_var("MISTRAL_API_KEY", k) };
        }
    }

    #[test]
    fn test_provider_models_endpoint()
    {
        assert_eq!(Provider::OpenAi.models_endpoint(), "https://api.openai.com/v1/models");
        assert_eq!(Provider::Anthropic.models_endpoint(), "https://api.anthropic.com/v1/models");
        assert_eq!(Provider::Ollama.models_endpoint(), "http://localhost:11434/api/tags");
        assert_eq!(Provider::Mistral.models_endpoint(), "https://api.mistral.ai/v1/models");
    }

    /// Serializes tests that modify environment variables
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
