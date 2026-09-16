// specs/003_hue_control.md
//! Philips Hue Bridge client over the local API (`http://<ip>/api/<username>`).

use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::settings::Target;

const DISCOVERY_URL: &str = "https://discovery.meethue.com";
const DEVICE_TYPE: &str = "open-actions#hue-control";
const TIMEOUT: Duration = Duration::from_secs(6);

/// A failure while talking to the bridge.
#[derive(Debug)]
pub enum BridgeError {
    /// The bridge could not be reached at all.
    Transport(String),
    /// The bridge answered with an error object (Hue reports these inside HTTP 200).
    Api { kind: u64, description: String },
    /// The bridge answered with a shape we do not understand.
    Shape(String),
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(message) => write!(f, "bridge unreachable: {message}"),
            Self::Api { kind, description } => write!(f, "bridge error {kind}: {description}"),
            Self::Shape(message) => write!(f, "unexpected bridge response: {message}"),
        }
    }
}

impl std::error::Error for BridgeError {}

impl BridgeError {
    /// Hue reports "link button not pressed" as error 101. The bridge is reachable and
    /// healthy; pairing simply has to wait for the user to press the button.
    pub fn link_button_pending(&self) -> bool {
        matches!(self, Self::Api { kind: 101, .. })
    }
}

impl From<reqwest::Error> for BridgeError {
    fn from(error: reqwest::Error) -> Self {
        Self::Transport(error.to_string())
    }
}

/// A bridge found by Hue's discovery service.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiscoveredBridge {
    pub id: String,
    #[serde(rename = "internalipaddress")]
    pub ip: String,
}

/// A room or zone on the bridge.
#[derive(Clone, Debug, Serialize)]
pub struct GroupInfo {
    pub id: String,
    pub name: String,
    pub lights: Vec<String>,
}

/// A single lamp on the bridge.
#[derive(Clone, Debug, Serialize)]
pub struct LightInfo {
    pub id: String,
    pub name: String,
}

/// A scene, which always belongs to a group.
#[derive(Clone, Debug, Serialize)]
pub struct SceneInfo {
    pub id: String,
    pub name: String,
    pub group: String,
}

/// Everything the property inspector needs to populate its pickers.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Targets {
    pub groups: Vec<GroupInfo>,
    pub lights: Vec<LightInfo>,
    pub scenes: Vec<SceneInfo>,
}

/// A paired bridge, addressed by IP and username.
pub struct Bridge {
    client: reqwest::Client,
    base: String,
}

impl Bridge {
    pub fn new(ip: &str, username: &str) -> Result<Self, BridgeError> {
        let ip = ip.trim();
        let username = username.trim();
        if ip.is_empty() || username.is_empty() {
            return Err(BridgeError::Shape(
                "bridge ip and username are required".to_owned(),
            ));
        }
        let client = reqwest::Client::builder().timeout(TIMEOUT).build()?;
        Ok(Self {
            client,
            base: format!("http://{ip}/api/{username}"),
        })
    }

    async fn get(&self, path: &str) -> Result<Value, BridgeError> {
        let url = format!("{}/{path}", self.base);
        let value: Value = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        reject_errors(&value)?;
        Ok(value)
    }

    async fn put(&self, path: &str, body: &Value) -> Result<(), BridgeError> {
        let url = format!("{}/{path}", self.base);
        let value: Value = self
            .client
            .put(url)
            .json(body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        reject_errors(&value)
    }

    /// Apply a state object to the target light or group.
    pub async fn set_state(&self, target: &Target, state: Value) -> Result<(), BridgeError> {
        self.put(&target.action_path(), &state).await
    }

    /// Read the target's current state object.
    pub async fn read_state(&self, target: &Target) -> Result<Value, BridgeError> {
        let resource = self.get(&target.resource_path()).await?;
        resource.get(target.state_key()).cloned().ok_or_else(|| {
            BridgeError::Shape(format!("no '{}' object in resource", target.state_key()))
        })
    }

    /// Fetch groups, lights, and scenes for the property inspector.
    pub async fn targets(&self) -> Result<Targets, BridgeError> {
        let groups = self.get("groups").await?;
        let lights = self.get("lights").await?;
        let scenes = self.get("scenes").await?;

        Ok(Targets {
            groups: map_entries(&groups, |id, entry| GroupInfo {
                id: id.to_owned(),
                name: name_of(entry),
                lights: entry
                    .get("lights")
                    .and_then(Value::as_array)
                    .map(|lights| {
                        lights
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default(),
            }),
            lights: map_entries(&lights, |id, entry| LightInfo {
                id: id.to_owned(),
                name: name_of(entry),
            }),
            scenes: map_entries(&scenes, |id, entry| SceneInfo {
                id: id.to_owned(),
                name: name_of(entry),
                group: entry
                    .get("group")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            }),
        })
    }
}

/// List bridges advertised on the local network.
pub async fn discover() -> Result<Vec<DiscoveredBridge>, BridgeError> {
    let client = reqwest::Client::builder().timeout(TIMEOUT).build()?;
    let bridges: Vec<DiscoveredBridge> = client
        .get(DISCOVERY_URL)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(bridges)
}

/// Read a bridge's own id from `/api/config`, so it can be keyed in global settings.
pub async fn config_id(ip: &str) -> Result<String, BridgeError> {
    let ip = ip.trim();
    if ip.is_empty() {
        return Err(BridgeError::Shape("a bridge ip is required".to_owned()));
    }
    let client = reqwest::Client::builder().timeout(TIMEOUT).build()?;
    let value: Value = client
        .get(format!("http://{ip}/api/config"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    value
        .get("bridgeid")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| BridgeError::Shape("config has no bridgeid".to_owned()))
}

/// Create a bridge username. The bridge link button must have been pressed first.
pub async fn pair(ip: &str) -> Result<String, BridgeError> {
    let ip = ip.trim();
    if ip.is_empty() {
        return Err(BridgeError::Shape(
            "a bridge ip is required to pair".to_owned(),
        ));
    }
    let client = reqwest::Client::builder().timeout(TIMEOUT).build()?;
    let value: Value = client
        .post(format!("http://{ip}/api"))
        .json(&json!({ "devicetype": DEVICE_TYPE }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    if let Some(username) = first_username(&value) {
        return Ok(username);
    }
    reject_errors(&value)?;
    Err(BridgeError::Shape(
        "pairing returned no username".to_owned(),
    ))
}

/// Extract the username from a pairing response.
fn first_username(value: &Value) -> Option<String> {
    value.as_array()?.iter().find_map(|item| {
        item.get("success")?
            .get("username")?
            .as_str()
            .map(str::to_owned)
    })
}

/// Reject Hue's in-band error objects, which arrive inside HTTP 200 responses.
fn reject_errors(value: &Value) -> Result<(), BridgeError> {
    let Some(items) = value.as_array() else {
        return Ok(());
    };
    for item in items {
        let Some(error) = item.get("error") else {
            continue;
        };
        let kind = error.get("type").and_then(Value::as_u64).unwrap_or(0);
        let description = error
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("unknown error")
            .to_owned();
        return Err(BridgeError::Api { kind, description });
    }
    Ok(())
}

/// Map a Hue id→object response into a sorted vector.
fn map_entries<T>(value: &Value, build: impl Fn(&str, &Value) -> T) -> Vec<T> {
    let Some(object) = value.as_object() else {
        return Vec::new();
    };
    let mut entries: Vec<(u64, T)> = object
        .iter()
        .map(|(id, entry)| (id.parse().unwrap_or(u64::MAX), build(id, entry)))
        .collect();
    entries.sort_by_key(|(order, _)| *order);
    entries.into_iter().map(|(_, entry)| entry).collect()
}

/// Read a resource's display name, falling back to an empty string.
fn name_of(value: &Value) -> String {
    value
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_the_pending_link_button_error() {
        let pending = BridgeError::Api {
            kind: 101,
            description: "link button not pressed".to_owned(),
        };
        assert!(pending.link_button_pending());
        let unauthorized = BridgeError::Api {
            kind: 1,
            description: "unauthorized user".to_owned(),
        };
        assert!(!unauthorized.link_button_pending());
        assert!(!BridgeError::Shape("whatever".to_owned()).link_button_pending());
    }

    #[test]
    fn extracts_the_username_from_a_pairing_response() {
        let value = json!([{ "success": { "username": "abc123" } }]);
        assert_eq!(first_username(&value), Some("abc123".to_owned()));
    }

    #[test]
    fn pairing_response_without_success_has_no_username() {
        let value = json!([{ "error": { "type": 101, "description": "link button not pressed" } }]);
        assert_eq!(first_username(&value), None);
    }

    #[test]
    fn rejects_in_band_api_errors() {
        let value = json!([{ "error": { "type": 1, "description": "unauthorized user" } }]);
        let error = reject_errors(&value).expect_err("should reject");
        assert!(matches!(error, BridgeError::Api { kind: 1, .. }));
    }

    #[test]
    fn accepts_success_payloads() {
        let value = json!([{ "success": { "/lights/8/state/on": true } }]);
        assert!(reject_errors(&value).is_ok());
    }

    #[test]
    fn accepts_non_array_payloads() {
        let value = json!({ "name": "Salón" });
        assert!(reject_errors(&value).is_ok());
    }

    #[test]
    fn maps_entries_in_numeric_id_order() {
        let value = json!({
            "10": { "name": "ten" },
            "2": { "name": "two" },
            "8": { "name": "eight" }
        });
        let names: Vec<String> =
            map_entries(&value, |id, entry| format!("{}:{}", id, name_of(entry)));
        assert_eq!(names, vec!["2:two", "8:eight", "10:ten"]);
    }

    #[test]
    fn requires_both_ip_and_username() {
        assert!(Bridge::new("192.168.1.74", "user").is_ok());
        assert!(Bridge::new("", "user").is_err());
        assert!(Bridge::new("192.168.1.74", "  ").is_err());
    }

    #[test]
    fn config_id_requires_an_ip() {
        let error = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(config_id("  "))
            .expect_err("should reject");
        assert!(matches!(error, BridgeError::Shape(_)));
    }

    #[test]
    fn pairing_requires_an_ip() {
        let error = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(pair("  "))
            .expect_err("should reject");
        assert!(matches!(error, BridgeError::Shape(_)));
    }

    #[test]
    fn deserializes_discovery_results() {
        let bridges: Vec<DiscoveredBridge> = serde_json::from_str(
            r#"[{"id":"001788fffe7a9abf","internalipaddress":"192.168.1.74"}]"#,
        )
        .expect("valid discovery payload");
        assert_eq!(bridges[0].id, "001788fffe7a9abf");
        assert_eq!(bridges[0].ip, "192.168.1.74");
    }
}
