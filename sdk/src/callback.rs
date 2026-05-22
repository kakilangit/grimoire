use std::sync::OnceLock;

use reqwest::Client;
use serde::Serialize;
use serde_json::Value;

use crate::contract::{
    CallbackResponse, EmitEventParams, InvocationResult, InvokeAgentParams, KeyParams,
    SetStateParams,
};
use crate::types::{Context, PluginError};

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn client() -> &'static Client {
    HTTP_CLIENT.get_or_init(Client::new)
}

#[derive(Serialize)]
struct CallbackRequest<'a, P: Serialize> {
    action: &'a str,
    params: P,
}

impl Context {
    /// Read a config value.
    #[must_use]
    pub fn config(&self, key: &str) -> Option<&str> {
        self.config.get(key).map(String::as_str)
    }

    /// Read a required config value.
    ///
    /// # Errors
    ///
    /// Returns `PluginError::Callback` if the key is missing from config.
    pub fn config_required(&self, key: &str) -> Result<&str, PluginError> {
        self.config(key)
            .ok_or_else(|| PluginError::Callback(format!("missing required config key: {key}")))
    }

    async fn call<P: Serialize>(
        &self,
        action: &str,
        params: P,
    ) -> Result<CallbackResponse, PluginError> {
        let resp = client()
            .post(&self.callback_url)
            .header("Content-Type", "application/json")
            .header("X-Plugin-Token", &self.callback_token)
            .header("X-Workspace-Id", &self.workspace_id)
            .header("X-Plugin-Id", &self.plugin_id)
            .json(&CallbackRequest { action, params })
            .send()
            .await
            .map_err(|e| PluginError::Callback(e.to_string()))?;

        let body = resp
            .json::<CallbackResponse>()
            .await
            .map_err(|e| PluginError::Callback(e.to_string()))?;

        if !body.ok {
            return Err(PluginError::Callback(
                body.error
                    .unwrap_or_else(|| "unknown callback error".into()),
            ));
        }

        Ok(body)
    }

    /// Invoke an agent synchronously — blocks until complete.
    ///
    /// # Errors
    ///
    /// Returns `PluginError::Callback` if the request fails or the response
    /// is missing required fields (`invocation_id`, `output`).
    pub async fn invoke_agent(
        &self,
        agent: &str,
        message: &str,
        external_ref: Option<&str>,
    ) -> Result<InvocationResult, PluginError> {
        let params = InvokeAgentParams {
            agent,
            message,
            external_ref,
        };

        let resp = self.call("invoke_agent", params).await?;

        let result = resp
            .result
            .ok_or_else(|| PluginError::Callback("missing result in response".into()))?;

        serde_json::from_value::<InvocationResult>(result)
            .map_err(|e| PluginError::Callback(format!("invalid invocation result: {e}")))
    }

    /// Invoke an agent asynchronously — result arrives via event.
    ///
    /// # Errors
    ///
    /// Returns `PluginError::Callback` if the request fails.
    pub async fn invoke_agent_async(
        &self,
        agent: &str,
        message: &str,
        external_ref: Option<&str>,
    ) -> Result<(), PluginError> {
        let params = InvokeAgentParams {
            agent,
            message,
            external_ref,
        };

        self.call("invoke_agent_async", params).await?;
        Ok(())
    }

    /// Emit a namespaced event.
    ///
    /// # Errors
    ///
    /// Returns `PluginError::Callback` if the callback request fails.
    pub async fn emit_event(&self, event: &str, data: Value) -> Result<(), PluginError> {
        self.call("emit_event", EmitEventParams { event, data })
            .await?;
        Ok(())
    }

    /// Get a plugin state value.
    ///
    /// # Errors
    ///
    /// Returns `PluginError::Callback` if the callback request fails.
    pub async fn get_state(&self, key: &str) -> Result<Option<Value>, PluginError> {
        let resp = self.call("get_state", KeyParams { key }).await?;

        Ok(resp.result.and_then(|r| {
            let v = &r["value"];
            if v.is_null() { None } else { Some(v.clone()) }
        }))
    }

    /// Set a plugin state value.
    ///
    /// # Errors
    ///
    /// Returns `PluginError::Callback` if the callback request fails.
    pub async fn set_state(&self, key: &str, value: Value) -> Result<(), PluginError> {
        self.call("set_state", SetStateParams { key, value })
            .await?;
        Ok(())
    }

    /// Delete a plugin state value.
    ///
    /// # Errors
    ///
    /// Returns `PluginError::Callback` if the callback request fails.
    pub async fn delete_state(&self, key: &str) -> Result<(), PluginError> {
        self.call("delete_state", KeyParams { key }).await?;
        Ok(())
    }
}
