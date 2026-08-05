use crate::{CapabilityDescriptor, ProviderKind, ProviderProfile, ProviderToolOutput};
use aurora_core::{AppError, AppResult, ErrorSeverity};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderToolCall {
    pub id: String,
    pub capability_id: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStep {
    pub response_id: Option<String>,
    pub assistant_text: Option<String>,
    pub tool_calls: Vec<ProviderToolCall>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub output_items: Vec<Value>,
}

#[derive(Debug, Clone, Copy)]
pub struct ProviderRequestContext<'a> {
    pub system_prompt: &'a str,
    pub task_context: &'a str,
    pub tools: &'a [CapabilityDescriptor],
    pub allow_parallel: bool,
    pub max_output_tokens: u32,
    pub previous_response_id: Option<&'a str>,
    pub tool_outputs: &'a [ProviderToolOutput],
    pub replay_items: &'a [Value],
}

pub fn provider_tool_name(capability_id: &str) -> String {
    capability_id.replace('.', "__")
}

fn capability_id_from_tool_name(name: &str) -> String {
    name.replace("__", ".")
}

pub fn build_provider_request(
    provider: &ProviderProfile,
    context: ProviderRequestContext<'_>,
) -> Value {
    let ProviderRequestContext {
        system_prompt,
        task_context,
        tools,
        allow_parallel,
        max_output_tokens,
        previous_response_id,
        tool_outputs,
        replay_items,
    } = context;
    let chat_tools = tools
        .iter()
        .map(|tool| {
            let strict = provider.supports_structured_output
                && schema_supports_strict(&tool.input_schema);
            json!({
                "type": "function",
                "function": {
                    "name": provider_tool_name(&tool.id),
                    "description": tool.description,
                    "strict": strict,
                    "parameters": if strict { strict_schema(&tool.input_schema) } else { tool.input_schema.clone() },
                }
            })
        })
        .collect::<Vec<_>>();
    let response_tools = tools
        .iter()
        .map(|tool| {
            let strict = provider.supports_structured_output
                && schema_supports_strict(&tool.input_schema);
            json!({
                "type": "function",
                "name": provider_tool_name(&tool.id),
                "description": tool.description,
                "strict": strict,
                "parameters": if strict { strict_schema(&tool.input_schema) } else { tool.input_schema.clone() },
            })
        })
        .collect::<Vec<_>>();
    let mut request = match provider.kind {
        ProviderKind::OpenAiResponses => {
            let function_outputs = tool_outputs
                .iter()
                .map(|result| {
                    json!({
                        "type": "function_call_output",
                        "call_id": result.call_id,
                        "output": serde_json::to_string(&result.output).unwrap_or_else(|_| "null".to_owned()),
                    })
                })
                .collect::<Vec<_>>();
            let input = if provider.store_responses {
                if previous_response_id.is_some() && !function_outputs.is_empty() {
                    Value::Array(function_outputs)
                } else {
                    Value::String(task_context.to_owned())
                }
            } else if replay_items.is_empty() {
                Value::String(task_context.to_owned())
            } else {
                Value::Array(
                    replay_items
                        .iter()
                        .cloned()
                        .chain(function_outputs)
                        .collect(),
                )
            };
            let mut request = json!({
                "model": provider.model,
                "instructions": system_prompt,
                "input": input,
                "tools": response_tools,
                "parallel_tool_calls": allow_parallel,
                "store": provider.store_responses,
            });
            if provider.store_responses
                && let Some(response_id) = previous_response_id
            {
                request["previous_response_id"] = json!(response_id);
            }
            request["max_output_tokens"] = json!(max_output_tokens);
            request
        }
        _ => json!({
            "model": provider.model,
            "temperature": 0,
            "messages": [
                {"role":"system","content":system_prompt},
                {"role":"user","content":task_context}
            ],
            "tools": chat_tools,
            "parallel_tool_calls": allow_parallel,
        }),
    };
    match provider.kind {
        ProviderKind::OpenAiResponses => {}
        ProviderKind::OpenAiChatCompletions => {
            request["max_completion_tokens"] = json!(max_output_tokens);
        }
        _ => {
            request["max_tokens"] = json!(max_output_tokens);
        }
    }
    if let Some(temperature) = provider.temperature_milli {
        request["temperature"] = json!(f64::from(temperature) / 1_000.0);
    }
    if let Some(effort) = provider
        .reasoning_effort
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        if provider.kind == ProviderKind::OpenAiResponses {
            request["reasoning"] = json!({ "effort": effort });
        } else {
            request["reasoning_effort"] = json!(effort);
        }
    }
    request
}

pub fn decode_provider_response(kind: ProviderKind, bytes: &[u8]) -> AppResult<ProviderStep> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| {
        provider_error(
            "AGENT_PROVIDER_RESPONSE_INVALID",
            "Le fournisseur n’a pas renvoyé un JSON valide.",
            error.to_string(),
        )
    })?;
    match kind {
        ProviderKind::OpenAiResponses => decode_responses(value),
        _ => decode_chat(value),
    }
}

fn decode_chat(value: Value) -> AppResult<ProviderStep> {
    let message = value.pointer("/choices/0/message").ok_or_else(|| {
        provider_error(
            "AGENT_PROVIDER_RESPONSE_INVALID",
            "La réponse ne contient aucun message assistant.",
            "missing choices[0].message",
        )
    })?;
    let assistant_text = message
        .get("content")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let mut tool_calls = Vec::new();
    for tool_call in message
        .get("tool_calls")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let name = tool_call
            .pointer("/function/name")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_tool_call("missing function name"))?;
        let arguments = tool_call
            .pointer("/function/arguments")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_tool_call("missing function arguments"))?;
        tool_calls.push(ProviderToolCall {
            id: tool_call
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or(name)
                .to_owned(),
            capability_id: capability_id_from_tool_name(name),
            arguments: serde_json::from_str(arguments)
                .map_err(|error| invalid_tool_call(error.to_string()))?,
        });
    }
    Ok(ProviderStep {
        response_id: value.get("id").and_then(Value::as_str).map(str::to_owned),
        assistant_text,
        tool_calls,
        input_tokens: value
            .pointer("/usage/prompt_tokens")
            .and_then(Value::as_u64),
        output_tokens: value
            .pointer("/usage/completion_tokens")
            .and_then(Value::as_u64),
        output_items: Vec::new(),
    })
}

fn decode_responses(value: Value) -> AppResult<ProviderStep> {
    let output_items = value
        .get("output")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut assistant_text = None;
    let mut tool_calls = Vec::new();
    for item in value
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        match item.get("type").and_then(Value::as_str) {
            Some("function_call") => {
                let name = item
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid_tool_call("missing Responses function name"))?;
                let arguments = item
                    .get("arguments")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid_tool_call("missing Responses function arguments"))?;
                tool_calls.push(ProviderToolCall {
                    id: item
                        .get("call_id")
                        .or_else(|| item.get("id"))
                        .and_then(Value::as_str)
                        .unwrap_or(name)
                        .to_owned(),
                    capability_id: capability_id_from_tool_name(name),
                    arguments: serde_json::from_str(arguments)
                        .map_err(|error| invalid_tool_call(error.to_string()))?,
                });
            }
            Some("message") => {
                assistant_text =
                    item.get("content")
                        .and_then(Value::as_array)
                        .and_then(|contents| {
                            contents.iter().find_map(|content| {
                                content
                                    .get("text")
                                    .and_then(Value::as_str)
                                    .map(str::to_owned)
                            })
                        });
            }
            _ => {}
        }
    }
    Ok(ProviderStep {
        response_id: value.get("id").and_then(Value::as_str).map(str::to_owned),
        assistant_text,
        tool_calls,
        input_tokens: value.pointer("/usage/input_tokens").and_then(Value::as_u64),
        output_tokens: value
            .pointer("/usage/output_tokens")
            .and_then(Value::as_u64),
        output_items,
    })
}

fn strict_schema(schema: &Value) -> Value {
    let mut schema = schema.clone();
    if let Some(object) = schema.as_object_mut()
        && object.get("type").and_then(Value::as_str) == Some("object")
    {
        object.insert("additionalProperties".to_owned(), Value::Bool(false));
        let required = object
            .get("properties")
            .and_then(Value::as_object)
            .map(|properties| {
                Value::Array(
                    properties
                        .keys()
                        .map(|key| Value::String(key.clone()))
                        .collect(),
                )
            })
            .unwrap_or_else(|| Value::Array(Vec::new()));
        object.insert("required".to_owned(), required);
    }
    schema
}

fn schema_supports_strict(schema: &Value) -> bool {
    match schema {
        Value::Object(object) => {
            if object.is_empty() {
                return false;
            }
            if object.get("type").and_then(Value::as_str) == Some("object")
                && object.get("additionalProperties") != Some(&Value::Bool(false))
            {
                return false;
            }
            object.values().all(schema_supports_strict)
        }
        Value::Array(values) => values.iter().all(schema_supports_strict),
        _ => true,
    }
}

fn invalid_tool_call(detail: impl Into<String>) -> Box<AppError> {
    provider_error(
        "AGENT_PROVIDER_TOOL_CALL_INVALID",
        "Le fournisseur a proposé un appel d’outil invalide.",
        detail,
    )
}

fn provider_error(
    code: impl Into<String>,
    user_message: impl Into<String>,
    technical_message: impl Into<String>,
) -> Box<AppError> {
    Box::new(
        AppError::new(code, user_message, technical_message, ErrorSeverity::Error)
            .with_import_stage("agent_provider"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentLimits, CapabilityRegistry};

    fn chat_provider() -> ProviderProfile {
        ProviderProfile {
            id: "test".to_owned(),
            name: "Test".to_owned(),
            kind: ProviderKind::OpenAiChatCompletions,
            endpoint: "https://example.invalid/v1/chat/completions".to_owned(),
            model: "model".to_owned(),
            reasoning_effort: None,
            temperature_milli: None,
            supports_tools: true,
            supports_parallel_tools: false,
            supports_structured_output: true,
            store_responses: false,
            input_cost_micro_usd_per_million_tokens: 0,
            output_cost_micro_usd_per_million_tokens: 0,
        }
    }

    fn test_request(
        provider: &ProviderProfile,
        tools: &[CapabilityDescriptor],
        allow_parallel: bool,
        previous_response_id: Option<&str>,
        tool_outputs: &[ProviderToolOutput],
        replay_items: &[Value],
    ) -> Value {
        build_provider_request(
            provider,
            ProviderRequestContext {
                system_prompt: "system",
                task_context: "task",
                tools,
                allow_parallel,
                max_output_tokens: 8_192,
                previous_response_id,
                tool_outputs,
                replay_items,
            },
        )
    }

    #[test]
    fn builds_strict_chat_tools() {
        let registry = CapabilityRegistry::standard();
        let body = test_request(
            &chat_provider(),
            &registry.capabilities[..1],
            false,
            None,
            &[],
            &[],
        );
        assert_eq!(body["tools"][0]["function"]["strict"], true);
        assert_eq!(body["max_completion_tokens"], 8_192);
        assert_eq!(
            body["tools"][0]["function"]["parameters"]["additionalProperties"],
            false
        );
    }

    #[test]
    fn disables_strict_tool_schemas_for_a_compatible_provider_without_support() {
        let registry = CapabilityRegistry::standard();
        let mut provider = chat_provider();
        provider.supports_structured_output = false;
        let body = test_request(
            &provider,
            &registry.capabilities[..1],
            false,
            None,
            &[],
            &[],
        );
        assert_eq!(body["tools"][0]["function"]["strict"], false);
    }

    #[test]
    fn the_complete_registry_fits_the_default_prompt_budget() {
        let registry = CapabilityRegistry::standard();
        let body = test_request(
            &chat_provider(),
            &registry.capabilities,
            true,
            None,
            &[],
            &[],
        );
        let size = serde_json::to_vec(&body).expect("provider body").len();
        assert!(
            size <= AgentLimits::default().max_prompt_bytes,
            "complete tool registry uses {size} bytes"
        );
    }

    #[test]
    fn decodes_chat_and_responses_tool_calls() {
        let chat = br#"{"id":"chat","choices":[{"message":{"content":null,"tool_calls":[{"id":"call","function":{"name":"module__inspect","arguments":"{}"}}]}}],"usage":{"prompt_tokens":3,"completion_tokens":2}}"#;
        let decoded = decode_provider_response(ProviderKind::Ollama, chat).expect("chat");
        assert_eq!(decoded.tool_calls[0].capability_id, "module.inspect");
        let responses = br#"{"id":"resp","output":[{"type":"function_call","call_id":"call","name":"module__inspect","arguments":"{}"}],"usage":{"input_tokens":4,"output_tokens":1}}"#;
        let decoded =
            decode_provider_response(ProviderKind::OpenAiResponses, responses).expect("responses");
        assert_eq!(decoded.tool_calls[0].capability_id, "module.inspect");
    }

    #[test]
    fn continues_responses_with_function_outputs() {
        let registry = CapabilityRegistry::standard();
        let mut provider = chat_provider();
        provider.kind = ProviderKind::OpenAiResponses;
        provider.store_responses = true;
        let outputs = vec![ProviderToolOutput {
            call_id: "call-1".to_owned(),
            output: json!({"ok": true}),
        }];
        let body = test_request(
            &provider,
            &registry.capabilities[..1],
            false,
            Some("resp-1"),
            &outputs,
            &[],
        );
        assert_eq!(body["previous_response_id"], "resp-1");
        assert_eq!(body["max_output_tokens"], 8_192);
        assert_eq!(body["input"][0]["type"], "function_call_output");
        assert_eq!(body["input"][0]["call_id"], "call-1");
        assert_eq!(body["input"][0]["output"], r#"{"ok":true}"#);
    }

    #[test]
    fn replays_responses_locally_when_provider_storage_is_disabled() {
        let registry = CapabilityRegistry::standard();
        let mut provider = chat_provider();
        provider.kind = ProviderKind::OpenAiResponses;
        provider.store_responses = false;
        let outputs = vec![ProviderToolOutput {
            call_id: "call-1".to_owned(),
            output: json!({"ok": true}),
        }];
        let replay = vec![
            json!({"role":"user","content":"task"}),
            json!({"type":"function_call","call_id":"call-1","name":"module__inspect","arguments":"{}"}),
        ];
        let body = test_request(
            &provider,
            &registry.capabilities[..1],
            false,
            Some("must-not-be-used"),
            &outputs,
            &replay,
        );
        assert_eq!(body["store"], false);
        assert!(body.get("previous_response_id").is_none());
        assert_eq!(body["input"].as_array().map(Vec::len), Some(3));
        assert_eq!(body["input"][2]["type"], "function_call_output");
    }
}
