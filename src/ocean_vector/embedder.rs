use std::fmt;

pub trait Embedder: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedderError>;
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedderError>;
    fn dimension(&self) -> usize;
    fn model_name(&self) -> &str;
}

#[derive(Debug, Clone)]
pub enum EmbedderError {
    ConnectionFailed(String),
    AuthenticationFailed(String),
    RateLimited(String),
    ModelReturnedError(String),
    Timeout(String),
    Unexpected(String),
}

impl fmt::Display for EmbedderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmbedderError::ConnectionFailed(msg) => write!(f, "connection failed: {}", msg),
            EmbedderError::AuthenticationFailed(msg) => write!(f, "authentication failed: {}", msg),
            EmbedderError::RateLimited(msg) => write!(f, "rate limited: {}", msg),
            EmbedderError::ModelReturnedError(msg) => write!(f, "model error: {}", msg),
            EmbedderError::Timeout(msg) => write!(f, "timeout: {}", msg),
            EmbedderError::Unexpected(msg) => write!(f, "unexpected error: {}", msg),
        }
    }
}

impl std::error::Error for EmbedderError {}

pub(crate) fn normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

// ── Ollama Embedder ──────────────────────────────────────────────

pub struct OllamaEmbedder {
    url: String,
    model: String,
    dimension: usize,
    client: reqwest::blocking::Client,
}

impl OllamaEmbedder {
    pub fn new(model: &str, url: &str) -> Result<Self, EmbedderError> {
        Self::with_timeout(model, url, 30)
    }

    pub fn with_timeout(model: &str, url: &str, timeout_secs: u64) -> Result<Self, EmbedderError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| EmbedderError::ConnectionFailed(e.to_string()))?;
        Ok(Self {
            url: url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            dimension: 768,
            client,
        })
    }

    pub fn with_dimension(model: &str, url: &str, dimension: usize) -> Result<Self, EmbedderError> {
        let mut s = Self::new(model, url)?;
        s.dimension = dimension;
        Ok(s)
    }
}

impl Embedder for OllamaEmbedder {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedderError> {
        let mut results = self.embed_batch(&[text])?;
        results.pop().ok_or_else(|| EmbedderError::ModelReturnedError("empty response".into()))
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedderError> {
        let body = serde_json::json!({
            "model": self.model,
            "input": texts,
        });

        let resp = self
            .client
            .post(format!("{}/api/embed", self.url))
            .json(&body)
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    EmbedderError::Timeout(e.to_string())
                } else if e.is_connect() {
                    EmbedderError::ConnectionFailed(e.to_string())
                } else {
                    EmbedderError::Unexpected(e.to_string())
                }
            })?;

        if resp.status().is_server_error() {
            return Err(EmbedderError::ModelReturnedError(format!(
                "Ollama server error: {}",
                resp.status()
            )));
        }

        if resp.status().is_client_error() {
            let status = resp.status();
            let body_text = resp.text().unwrap_or_default();
            return Err(EmbedderError::ModelReturnedError(format!(
                "Ollama client error {}: {}",
                status, body_text
            )));
        }

        let parsed: OllamaEmbedResponse = resp
            .json()
            .map_err(|e| EmbedderError::Unexpected(format!("invalid JSON response: {}", e)))?;

        let dim = self.dimension;
        let embeddings: Vec<Vec<f32>> = parsed
            .embeddings
            .into_iter()
            .map(|mut emb| {
                if emb.len() != dim {
                    return Err(EmbedderError::ModelReturnedError(format!(
                        "expected dim {}, got {}",
                        dim,
                        emb.len()
                    )));
                }
                normalize(&mut emb);
                Ok(emb)
            })
            .collect::<Result<Vec<_>, _>>()?;

        if embeddings.len() != texts.len() {
            return Err(EmbedderError::ModelReturnedError(format!(
                "expected {} embeddings, got {}",
                texts.len(),
                embeddings.len()
            )));
        }

        Ok(embeddings)
    }
}

#[derive(serde::Deserialize)]
struct OllamaEmbedResponse {
    #[allow(dead_code)]
    model: String,
    embeddings: Vec<Vec<f32>>,
}

// ── OpenAI-Compatible Embedder ───────────────────────────────────

pub struct OpenAIEmbedder {
    base_url: String,
    api_key: String,
    model: String,
    dimension: usize,
    client: reqwest::blocking::Client,
}

impl OpenAIEmbedder {
    pub fn new(model: &str, base_url: &str, api_key: &str) -> Result<Self, EmbedderError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| EmbedderError::ConnectionFailed(e.to_string()))?;
        let dim = match model {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => 768,
        };
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            dimension: dim,
            client,
        })
    }

    pub fn with_dimension(model: &str, base_url: &str, api_key: &str, dimension: usize) -> Result<Self, EmbedderError> {
        let mut s = Self::new(model, base_url, api_key)?;
        s.dimension = dimension;
        Ok(s)
    }
}

impl Embedder for OpenAIEmbedder {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedderError> {
        let mut results = self.embed_batch(&[text])?;
        results.pop().ok_or_else(|| EmbedderError::ModelReturnedError("empty response".into()))
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedderError> {
        let mut body = serde_json::json!({
            "model": self.model,
            "input": texts,
        });
        if self.dimension > 0 && (self.model == "text-embedding-3-small" || self.model == "text-embedding-3-large") {
            body["dimensions"] = serde_json::json!(self.dimension);
        }

        let resp = self
            .client
            .post(format!("{}/v1/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    EmbedderError::Timeout(e.to_string())
                } else if e.is_connect() {
                    EmbedderError::ConnectionFailed(e.to_string())
                } else {
                    EmbedderError::Unexpected(e.to_string())
                }
            })?;

        let status = resp.status();
        if status == 401 || status == 403 {
            return Err(EmbedderError::AuthenticationFailed(format!(
                "HTTP {}: check your API key",
                status
            )));
        }
        if status == 429 {
            return Err(EmbedderError::RateLimited(format!(
                "HTTP {}: rate limit exceeded",
                status
            )));
        }
        if !status.is_success() {
            let body_text = resp.text().unwrap_or_default();
            return Err(EmbedderError::ModelReturnedError(format!(
                "HTTP {}: {}",
                status, body_text
            )));
        }

        let parsed: OpenAIEmbedResponse = resp
            .json()
            .map_err(|e| EmbedderError::Unexpected(format!("invalid JSON response: {}", e)))?;

        let dim = self.dimension;
        let mut embeddings: Vec<Vec<f32>> = vec![Vec::with_capacity(dim); texts.len()];

        for item in parsed.data {
            let idx = item.index;
            if idx >= embeddings.len() {
                return Err(EmbedderError::ModelReturnedError(format!(
                    "index {} out of range (expected {})",
                    idx,
                    texts.len()
                )));
            }
            let mut emb = item.embedding;
            if emb.len() != dim {
                return Err(EmbedderError::ModelReturnedError(format!(
                    "expected dim {}, got {} at index {}",
                    dim,
                    emb.len(),
                    idx
                )));
            }
            normalize(&mut emb);
            embeddings[idx] = emb;
        }

        for (i, emb) in embeddings.iter().enumerate() {
            if emb.is_empty() {
                return Err(EmbedderError::ModelReturnedError(format!(
                    "missing embedding for index {}",
                    i
                )));
            }
        }

        Ok(embeddings)
    }
}

// ── Anthropic-Compatible Embedder ─────────────────────────────────

pub struct AnthropicEmbedder {
    base_url: String,
    api_key: String,
    model: String,
    dimension: usize,
    client: reqwest::blocking::Client,
}

impl AnthropicEmbedder {
    pub fn new(model: &str, base_url: &str, api_key: &str) -> Result<Self, EmbedderError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| EmbedderError::ConnectionFailed(e.to_string()))?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            dimension: 768,
            client,
        })
    }

    pub fn with_dimension(model: &str, base_url: &str, api_key: &str, dimension: usize) -> Result<Self, EmbedderError> {
        let mut s = Self::new(model, base_url, api_key)?;
        s.dimension = dimension;
        Ok(s)
    }
}

impl Embedder for AnthropicEmbedder {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedderError> {
        let mut results = self.embed_batch(&[text])?;
        results.pop().ok_or_else(|| EmbedderError::ModelReturnedError("empty response".into()))
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedderError> {
        let body = serde_json::json!({
            "model": self.model,
            "input": texts,
        });

        let resp = self
            .client
            .post(format!("{}/v1/embeddings", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    EmbedderError::Timeout(e.to_string())
                } else if e.is_connect() {
                    EmbedderError::ConnectionFailed(e.to_string())
                } else {
                    EmbedderError::Unexpected(e.to_string())
                }
            })?;

        let status = resp.status();
        if status == 401 || status == 403 {
            return Err(EmbedderError::AuthenticationFailed(format!(
                "HTTP {}: check your API key",
                status
            )));
        }
        if status == 429 {
            return Err(EmbedderError::RateLimited(format!(
                "HTTP {}: rate limit exceeded",
                status
            )));
        }
        if !status.is_success() {
            let body_text = resp.text().unwrap_or_default();
            return Err(EmbedderError::ModelReturnedError(format!(
                "HTTP {}: {}",
                status, body_text
            )));
        }

        let parsed: OpenAIEmbedResponse = resp
            .json()
            .map_err(|e| EmbedderError::Unexpected(format!("invalid JSON response: {}", e)))?;

        let dim = self.dimension;
        let mut embeddings: Vec<Vec<f32>> = vec![Vec::with_capacity(dim); texts.len()];

        for item in parsed.data {
            let idx = item.index;
            if idx >= embeddings.len() {
                return Err(EmbedderError::ModelReturnedError(format!(
                    "index {} out of range (expected {})",
                    idx,
                    texts.len()
                )));
            }
            let mut emb = item.embedding;
            if emb.len() != dim {
                return Err(EmbedderError::ModelReturnedError(format!(
                    "expected dim {}, got {} at index {}",
                    dim,
                    emb.len(),
                    idx
                )));
            }
            normalize(&mut emb);
            embeddings[idx] = emb;
        }

        for (i, emb) in embeddings.iter().enumerate() {
            if emb.is_empty() {
                return Err(EmbedderError::ModelReturnedError(format!(
                    "missing embedding for index {}",
                    i
                )));
            }
        }

        Ok(embeddings)
    }
}

// ── Google Gemini Embedder ────────────────────────────────────────

pub struct GeminiEmbedder {
    api_key: String,
    model: String,
    dimension: usize,
    client: reqwest::blocking::Client,
}

impl GeminiEmbedder {
    pub fn new(model: &str, api_key: &str) -> Result<Self, EmbedderError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| EmbedderError::ConnectionFailed(e.to_string()))?;
        let dim = match model {
            "text-embedding-004" => 768,
            "text-embedding-005" => 768,
            "text-multilingual-embedding-002" => 768,
            _ => 768,
        };
        Ok(Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            dimension: dim,
            client,
        })
    }

    pub fn with_dimension(model: &str, api_key: &str, dimension: usize) -> Result<Self, EmbedderError> {
        let mut s = Self::new(model, api_key)?;
        s.dimension = dimension;
        Ok(s)
    }
}

impl Embedder for GeminiEmbedder {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedderError> {
        let resp = self
            .client
            .post(format!(
                "https://generativelanguage.googleapis.com/v1/models/{}:embedContent?key={}",
                self.model, self.api_key
            ))
            .json(&serde_json::json!({
                "model": format!("models/{}", self.model),
                "content": {
                    "parts": [{ "text": text }]
                }
            }))
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    EmbedderError::Timeout(e.to_string())
                } else if e.is_connect() {
                    EmbedderError::ConnectionFailed(e.to_string())
                } else {
                    EmbedderError::Unexpected(e.to_string())
                }
            })?;

        let status = resp.status();
        if status == 401 || status == 403 {
            return Err(EmbedderError::AuthenticationFailed(format!(
                "HTTP {}: check your API key",
                status
            )));
        }
        if status == 429 {
            return Err(EmbedderError::RateLimited(format!(
                "HTTP {}: rate limit exceeded",
                status
            )));
        }
        if !status.is_success() {
            let body_text = resp.text().unwrap_or_default();
            return Err(EmbedderError::ModelReturnedError(format!(
                "HTTP {}: {}",
                status, body_text
            )));
        }

        let parsed: GeminiEmbedResponse = resp
            .json()
            .map_err(|e| EmbedderError::Unexpected(format!("invalid JSON response: {}", e)))?;

        let dim = self.dimension;
        let mut embedding = parsed.embedding.values;
        if embedding.len() != dim {
            return Err(EmbedderError::ModelReturnedError(format!(
                "expected dim {}, got {}",
                dim,
                embedding.len()
            )));
        }
        normalize(&mut embedding);
        Ok(embedding)
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedderError> {
        let requests: Vec<serde_json::Value> = texts
            .iter()
            .map(|t| {
                serde_json::json!({
                    "model": format!("models/{}", self.model),
                    "content": {
                        "parts": [{ "text": t }]
                    }
                })
            })
            .collect();

        let body = serde_json::json!({ "requests": requests });

        let resp = self
            .client
            .post(format!(
                "https://generativelanguage.googleapis.com/v1/models/{}:batchEmbedContents?key={}",
                self.model, self.api_key
            ))
            .json(&body)
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    EmbedderError::Timeout(e.to_string())
                } else if e.is_connect() {
                    EmbedderError::ConnectionFailed(e.to_string())
                } else {
                    EmbedderError::Unexpected(e.to_string())
                }
            })?;

        let status = resp.status();
        if status == 401 || status == 403 {
            return Err(EmbedderError::AuthenticationFailed(format!(
                "HTTP {}: check your API key",
                status
            )));
        }
        if status == 429 {
            return Err(EmbedderError::RateLimited(format!(
                "HTTP {}: rate limit exceeded",
                status
            )));
        }
        if !status.is_success() {
            let body_text = resp.text().unwrap_or_default();
            return Err(EmbedderError::ModelReturnedError(format!(
                "HTTP {}: {}",
                status, body_text
            )));
        }

        let parsed: GeminiBatchEmbedResponse = resp
            .json()
            .map_err(|e| EmbedderError::Unexpected(format!("invalid JSON response: {}", e)))?;

        let dim = self.dimension;
        let mut embeddings: Vec<Vec<f32>> = Vec::with_capacity(texts.len());

        for item in parsed.embeddings {
            let mut emb = item.values;
            if emb.len() != dim {
                return Err(EmbedderError::ModelReturnedError(format!(
                    "expected dim {}, got {}",
                    dim,
                    emb.len()
                )));
            }
            normalize(&mut emb);
            embeddings.push(emb);
        }

        if embeddings.len() != texts.len() {
            return Err(EmbedderError::ModelReturnedError(format!(
                "expected {} embeddings, got {}",
                texts.len(),
                embeddings.len()
            )));
        }

        Ok(embeddings)
    }
}

#[derive(serde::Deserialize)]
struct GeminiEmbedResponse {
    embedding: GeminiEmbedding,
}

#[derive(serde::Deserialize)]
struct GeminiBatchEmbedResponse {
    embeddings: Vec<GeminiEmbedding>,
}

#[derive(serde::Deserialize)]
struct GeminiEmbedding {
    values: Vec<f32>,
}

#[derive(serde::Deserialize)]
struct OpenAIEmbedResponse {
    data: Vec<OpenAIEmbedding>,
    #[allow(dead_code)]
    model: String,
}

#[derive(serde::Deserialize)]
struct OpenAIEmbedding {
    embedding: Vec<f32>,
    index: usize,
}
