use pyo3::ffi::c_str;
use pyo3::{prelude::*, types::PyModule};
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub(crate) struct AiUsage {
    pub(crate) prompt_tokens: Option<u64>,
    pub(crate) output_tokens: Option<u64>,
    pub(crate) total_tokens: Option<u64>,
    pub(crate) total_duration_ms: Option<f64>,
    pub(crate) load_duration_ms: Option<f64>,
}

#[derive(Debug, Clone)]
pub(crate) struct AiGeneration {
    pub(crate) text: String,
    pub(crate) usage: Option<AiUsage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AiProvider {
    #[default]
    Ollama,
    OpenAiCompatible,
    OpenAi,
}

impl AiProvider {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Ollama => "Ollama",
            Self::OpenAiCompatible => "OpenAI-Compatible",
            Self::OpenAi => "OpenAI Cloud",
        }
    }

    pub(crate) fn default_base_url(self) -> &'static str {
        match self {
            Self::Ollama => "http://127.0.0.1:11434",
            Self::OpenAiCompatible => "http://127.0.0.1:1234/v1",
            Self::OpenAi => "https://api.openai.com/v1",
        }
    }

    pub(crate) fn default_model(self) -> &'static str {
        match self {
            Self::Ollama => "qwen3-coder:480b-cloud",
            Self::OpenAiCompatible => "",
            Self::OpenAi => "gpt-5-mini",
        }
    }

    pub(crate) fn model_hint(self) -> &'static str {
        match self {
            Self::Ollama => "qwen3-coder:480b-cloud",
            Self::OpenAiCompatible => "Select a local OpenAI-compatible model",
            Self::OpenAi => "gpt-5-mini",
        }
    }

    pub(crate) fn help_text(self) -> &'static str {
        match self {
            Self::Ollama => {
                "Uses qwen3-coder:480b-cloud by default through the local Ollama app after `ollama signin`."
            }
            Self::OpenAiCompatible => {
                "For a local or self-hosted OpenAI-compatible server such as LM Studio."
            }
            Self::OpenAi => "Uses OpenAI's hosted Responses API and requires OPENAI_API_KEY.",
        }
    }
}

pub(crate) fn generate_text(
    provider: AiProvider,
    base_url: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    schema: &Value,
) -> Result<AiGeneration, String> {
    if model.trim().is_empty() {
        return Err("No AI model is selected.".to_owned());
    }

    match provider {
        AiProvider::Ollama => {
            let response_body =
                call_ollama_generate_api(base_url, model, system_prompt, user_prompt, schema)
                    .map_err(|error| format!("Ollama request failed: {error}"))?;
            let response_json: Value = serde_json::from_str(&response_body).map_err(|error| {
                format!("Failed to parse the Ollama API response JSON: {error}")
            })?;
            let text = response_json
                .get("response")
                .and_then(Value::as_str)
                .ok_or_else(|| "The Ollama API response did not include response text.".to_owned())?
                .to_owned();

            Ok(AiGeneration {
                text,
                usage: ollama_usage_from_response(&response_json),
            })
        }
        AiProvider::OpenAiCompatible => {
            let response_body = call_openai_responses_api(
                base_url,
                None,
                model,
                system_prompt,
                user_prompt,
                schema,
            )
            .map_err(|error| format!("OpenAI-compatible request failed: {error}"))?;

            let response_json: Value = serde_json::from_str(&response_body).map_err(|error| {
                format!("Failed to parse the OpenAI-compatible API response JSON: {error}")
            })?;
            let text = extract_output_text(&response_json)?;
            Ok(AiGeneration {
                text,
                usage: openai_usage_from_response(&response_json),
            })
        }
        AiProvider::OpenAi => {
            let api_key = std::env::var("OPENAI_API_KEY")
                .map_err(|error| format!("OPENAI_API_KEY is not set: {error}"))?;
            let response_body = call_openai_responses_api(
                base_url,
                Some(&api_key),
                model,
                system_prompt,
                user_prompt,
                schema,
            )
            .map_err(|error| format!("OpenAI request failed: {error}"))?;

            let response_json: Value = serde_json::from_str(&response_body).map_err(|error| {
                format!("Failed to parse the OpenAI API response JSON: {error}")
            })?;
            let text = extract_output_text(&response_json)?;
            Ok(AiGeneration {
                text,
                usage: openai_usage_from_response(&response_json),
            })
        }
    }
}

pub(crate) fn list_models(provider: AiProvider, base_url: &str) -> Result<Vec<String>, String> {
    let mut models = match provider {
        AiProvider::Ollama => list_ollama_models(base_url)
            .map_err(|error| format!("Ollama model listing failed: {error}"))?,
        AiProvider::OpenAiCompatible => list_openai_compatible_models(base_url, None)
            .map_err(|error| format!("OpenAI-compatible model listing failed: {error}"))?,
        AiProvider::OpenAi => {
            return Err("Local model listing is only available for local providers.".to_owned());
        }
    };

    models.sort();
    models.dedup();
    Ok(models)
}

fn extract_output_text(response_json: &Value) -> Result<String, String> {
    let output = response_json
        .get("output")
        .and_then(Value::as_array)
        .ok_or_else(|| "The OpenAI API response did not include an `output` array.".to_owned())?;

    for item in output {
        let Some(content) = item.get("content").and_then(Value::as_array) else {
            continue;
        };

        for content_item in content {
            match content_item.get("type").and_then(Value::as_str) {
                Some("output_text") => {
                    if let Some(text) = content_item.get("text").and_then(Value::as_str) {
                        return Ok(text.to_owned());
                    }
                }
                Some("refusal") => {
                    let refusal = content_item
                        .get("refusal")
                        .and_then(Value::as_str)
                        .unwrap_or("The model refused to produce a response.");
                    return Err(format!("The model refused the request: {refusal}"));
                }
                _ => {}
            }
        }
    }

    Err("The OpenAI API response did not include any output text.".to_owned())
}

fn ollama_usage_from_response(response_json: &Value) -> Option<AiUsage> {
    let prompt_tokens = response_json
        .get("prompt_eval_count")
        .and_then(Value::as_u64);
    let output_tokens = response_json.get("eval_count").and_then(Value::as_u64);
    let total_tokens = match (prompt_tokens, output_tokens) {
        (Some(prompt), Some(output)) => Some(prompt + output),
        _ => None,
    };
    let total_duration_ms = response_json
        .get("total_duration")
        .and_then(Value::as_u64)
        .map(nanos_to_millis);
    let load_duration_ms = response_json
        .get("load_duration")
        .and_then(Value::as_u64)
        .map(nanos_to_millis);

    if prompt_tokens.is_none()
        && output_tokens.is_none()
        && total_duration_ms.is_none()
        && load_duration_ms.is_none()
    {
        return None;
    }

    Some(AiUsage {
        prompt_tokens,
        output_tokens,
        total_tokens,
        total_duration_ms,
        load_duration_ms,
    })
}

fn openai_usage_from_response(response_json: &Value) -> Option<AiUsage> {
    let usage = response_json.get("usage")?;
    let prompt_tokens = usage.get("input_tokens").and_then(Value::as_u64);
    let output_tokens = usage.get("output_tokens").and_then(Value::as_u64);
    let total_tokens = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .or_else(|| match (prompt_tokens, output_tokens) {
            (Some(prompt), Some(output)) => Some(prompt + output),
            _ => None,
        });

    Some(AiUsage {
        prompt_tokens,
        output_tokens,
        total_tokens,
        total_duration_ms: None,
        load_duration_ms: None,
    })
}

fn nanos_to_millis(nanos: u64) -> f64 {
    nanos as f64 / 1_000_000.0
}

fn call_openai_responses_api(
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    schema: &Value,
) -> PyResult<String> {
    let schema_json = serde_json::to_string(schema).map_err(|error| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Failed to serialize the Spectrix response schema: {error}"
        ))
    })?;

    Python::attach(|py| {
        let code = c_str!(
            r##"
import json
import urllib.error
import urllib.request


def create_response(base_url, api_key, model, system_prompt, user_prompt, schema_json):
    schema = json.loads(schema_json)
    payload = {
        "model": model,
        "store": False,
        "max_output_tokens": 2500,
        "input": [
            {
                "role": "system",
                "content": [
                    {
                        "type": "input_text",
                        "text": system_prompt,
                    }
                ],
            },
            {
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": user_prompt,
                    }
                ],
            },
        ],
        "text": {
            "format": {
                "type": "json_schema",
                "name": "spectrix_ai_help",
                "schema": schema,
                "strict": True,
            }
        },
    }

    request_url = base_url.rstrip("/") + "/responses"
    headers = {
        "Content-Type": "application/json",
    }
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"

    request = urllib.request.Request(
        request_url,
        data=json.dumps(payload).encode("utf-8"),
        headers=headers,
        method="POST",
    )

    try:
        with urllib.request.urlopen(request, timeout=90) as response:
            return response.read().decode("utf-8")
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"HTTP {error.code}: {body}") from error
    except urllib.error.URLError as error:
        raise RuntimeError(f"Network error: {error}") from error
	"##
        );

        let module = PyModule::from_code(py, code, c_str!("openai_api.py"), c_str!("openai_api"))?;
        module
            .getattr("create_response")?
            .call1((
                base_url,
                api_key,
                model,
                system_prompt,
                user_prompt,
                schema_json,
            ))?
            .extract()
    })
}

fn list_openai_compatible_models(base_url: &str, api_key: Option<&str>) -> PyResult<Vec<String>> {
    Python::attach(|py| {
        let code = c_str!(
            r##"
import json
import urllib.error
import urllib.request


def list_models(base_url, api_key):
    request_url = base_url.rstrip("/") + "/models"
    headers = {}
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"

    request = urllib.request.Request(
        request_url,
        headers=headers,
        method="GET",
    )

    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            body = json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"HTTP {error.code}: {body}") from error
    except urllib.error.URLError as error:
        raise RuntimeError(f"Network error: {error}") from error

    data = body.get("data")
    if not isinstance(data, list):
        raise RuntimeError(f"Model list missing `data`: {body}")

    models = []
    for item in data:
        if isinstance(item, dict):
            model_id = item.get("id")
            if isinstance(model_id, str) and model_id:
                models.append(model_id)
    return models
	"##
        );

        let module = PyModule::from_code(
            py,
            code,
            c_str!("openai_model_list.py"),
            c_str!("openai_model_list"),
        )?;
        module
            .getattr("list_models")?
            .call1((base_url, api_key))?
            .extract()
    })
}

fn list_ollama_models(base_url: &str) -> PyResult<Vec<String>> {
    Python::attach(|py| {
        let code = c_str!(
            r##"
import json
import urllib.error
import urllib.request


def list_models(base_url):
    request = urllib.request.Request(
        base_url.rstrip("/") + "/api/tags",
        headers={
            "Content-Type": "application/json",
        },
        method="GET",
    )

    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            body = json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"HTTP {error.code}: {body}") from error
    except urllib.error.URLError as error:
        raise RuntimeError(f"Network error: {error}") from error

    entries = body.get("models")
    if not isinstance(entries, list):
        raise RuntimeError(f"Model list missing `models`: {body}")

    models = []
    for item in entries:
        if isinstance(item, dict):
            name = item.get("name") or item.get("model")
            if isinstance(name, str) and name:
                models.append(name)
    return models
	"##
        );

        let module = PyModule::from_code(
            py,
            code,
            c_str!("ollama_model_list.py"),
            c_str!("ollama_model_list"),
        )?;
        module.getattr("list_models")?.call1((base_url,))?.extract()
    })
}

fn call_ollama_generate_api(
    base_url: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    schema: &Value,
) -> PyResult<String> {
    let schema_json = serde_json::to_string(schema).map_err(|error| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Failed to serialize the Spectrix response schema: {error}"
        ))
    })?;

    Python::attach(|py| {
        let code = c_str!(
            r#"
import json
import urllib.error
import urllib.request


def create_response(base_url, model, system_prompt, user_prompt, schema_json):
    payload = {
        "model": model,
        "system": system_prompt,
        "prompt": user_prompt,
        "stream": False,
        "format": json.loads(schema_json),
    }

    request = urllib.request.Request(
        base_url.rstrip("/") + "/api/generate",
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
        },
        method="POST",
    )

    try:
        with urllib.request.urlopen(request, timeout=90) as response:
            body = json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"HTTP {error.code}: {body}") from error
    except urllib.error.URLError as error:
        raise RuntimeError(f"Network error: {error}") from error

    if not isinstance(body.get("response"), str):
        raise RuntimeError(f"Ollama response missing `response` text: {body}")
    return json.dumps(body)
"#
        );

        let module = PyModule::from_code(py, code, c_str!("ollama_api.py"), c_str!("ollama_api"))?;
        module
            .getattr("create_response")?
            .call1((base_url, model, system_prompt, user_prompt, schema_json))?
            .extract()
    })
}
