// specs/005_antigravity_usage.md
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    #[default]
    GeminiQuota,
    ClaudeQuota,
    PromptCredits,
    FlowCredits,
}

impl Metric {
    pub const ALL: [Self; 4] = [
        Self::GeminiQuota,
        Self::ClaudeQuota,
        Self::PromptCredits,
        Self::FlowCredits,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::GeminiQuota => "Gemini",
            Self::ClaudeQuota => "Claude",
            Self::PromptCredits => "Credits",
            Self::FlowCredits => "Flow",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayFractionAs {
    #[default]
    Remaining,
    Used,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct UserStatusResponse {
    #[serde(rename = "userStatus", alias = "user_status")]
    pub user_status: UserStatus,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct UserStatus {
    #[serde(rename = "planStatus", alias = "plan_status", default)]
    pub plan_status: Option<PlanStatus>,
    #[serde(
        rename = "cascadeModelConfigData",
        alias = "cascade_model_config_data",
        default
    )]
    pub cascade_model_config_data: Option<CascadeModelConfigData>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct PlanStatus {
    #[serde(rename = "planInfo", alias = "plan_info", default)]
    pub plan_info: Option<PlanInfo>,
    #[serde(
        rename = "availablePromptCredits",
        alias = "available_prompt_credits",
        default
    )]
    pub available_prompt_credits: Option<u64>,
    #[serde(
        rename = "availableFlowCredits",
        alias = "available_flow_credits",
        default
    )]
    pub available_flow_credits: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct PlanInfo {
    #[serde(rename = "planName", alias = "plan_name", default)]
    pub plan_name: Option<String>,
    #[serde(
        rename = "monthlyPromptCredits",
        alias = "monthly_prompt_credits",
        default
    )]
    pub monthly_prompt_credits: Option<u64>,
    #[serde(rename = "monthlyFlowCredits", alias = "monthly_flow_credits", default)]
    pub monthly_flow_credits: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct CascadeModelConfigData {
    #[serde(rename = "clientModelConfigs", alias = "client_model_configs", default)]
    pub client_model_configs: Vec<ClientModelConfig>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct ClientModelConfig {
    #[serde(default)]
    pub label: String,
    #[serde(rename = "quotaInfo", alias = "quota_info", default)]
    pub quota_info: Option<QuotaInfo>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct QuotaInfo {
    #[serde(rename = "remainingFraction", alias = "remaining_fraction", default)]
    pub remaining_fraction: Option<f64>,
    #[serde(rename = "resetTime", alias = "reset_time", default)]
    pub reset_time: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetricDisplay {
    pub label: &'static str,
    pub hero: String,
    pub bar_percent: u8,
    pub subtitle: String,
}

impl UserStatusResponse {
    pub fn display(&self, metric: Metric, fraction_mode: DisplayFractionAs) -> MetricDisplay {
        let configs = self
            .user_status
            .cascade_model_config_data
            .as_ref()
            .map(|c| c.client_model_configs.as_slice())
            .unwrap_or(&[]);

        match metric {
            Metric::GeminiQuota => {
                let gemini_cfg = configs
                    .iter()
                    .find(|c| {
                        c.label.to_lowercase().starts_with("gemini") && c.quota_info.is_some()
                    })
                    .and_then(|c| c.quota_info.as_ref());

                Self::format_model_quota(Metric::GeminiQuota.label(), gemini_cfg, fraction_mode)
            }
            Metric::ClaudeQuota => {
                let claude_cfg = configs
                    .iter()
                    .find(|c| {
                        c.label.to_lowercase().starts_with("claude") && c.quota_info.is_some()
                    })
                    .and_then(|c| c.quota_info.as_ref());

                Self::format_model_quota(Metric::ClaudeQuota.label(), claude_cfg, fraction_mode)
            }
            Metric::PromptCredits => {
                let plan = self.user_status.plan_status.as_ref();
                let avail = plan.and_then(|p| p.available_prompt_credits).unwrap_or(0);
                let monthly = plan
                    .and_then(|p| p.plan_info.as_ref())
                    .and_then(|i| i.monthly_prompt_credits)
                    .unwrap_or(0);

                let (hero, bar_pct) = if monthly > 0 {
                    let pct = ((avail as f64 / monthly as f64) * 100.0).round().min(100.0) as u8;
                    let display_pct = if fraction_mode == DisplayFractionAs::Used {
                        100u8.saturating_sub(pct)
                    } else {
                        pct
                    };
                    (format!("{display_pct}%"), pct)
                } else {
                    (format!("{avail}"), 100)
                };

                let subtitle = if monthly > 0 {
                    format!("{avail} / {}", format_count(monthly))
                } else {
                    format!("{avail} avail")
                };

                MetricDisplay {
                    label: Metric::PromptCredits.label(),
                    hero,
                    bar_percent: bar_pct,
                    subtitle,
                }
            }
            Metric::FlowCredits => {
                let plan = self.user_status.plan_status.as_ref();
                let avail = plan.and_then(|p| p.available_flow_credits).unwrap_or(0);
                let monthly = plan
                    .and_then(|p| p.plan_info.as_ref())
                    .and_then(|i| i.monthly_flow_credits)
                    .unwrap_or(0);

                let (hero, bar_pct) = if monthly > 0 {
                    let pct = ((avail as f64 / monthly as f64) * 100.0).round().min(100.0) as u8;
                    let display_pct = if fraction_mode == DisplayFractionAs::Used {
                        100u8.saturating_sub(pct)
                    } else {
                        pct
                    };
                    (format!("{display_pct}%"), pct)
                } else {
                    (format!("{avail}"), 100)
                };

                let subtitle = if monthly > 0 {
                    format!("{avail} / {}", format_count(monthly))
                } else {
                    format!("{avail} avail")
                };

                MetricDisplay {
                    label: Metric::FlowCredits.label(),
                    hero,
                    bar_percent: bar_pct,
                    subtitle,
                }
            }
        }
    }

    fn format_model_quota(
        label: &'static str,
        quota: Option<&QuotaInfo>,
        fraction_mode: DisplayFractionAs,
    ) -> MetricDisplay {
        let Some(q) = quota else {
            return MetricDisplay {
                label,
                hero: "N/A".to_string(),
                bar_percent: 0,
                subtitle: "No quota".to_string(),
            };
        };

        let rem_fraction = q.remaining_fraction.unwrap_or(1.0).clamp(0.0, 1.0);
        let rem_pct = (rem_fraction * 100.0).round() as u8;
        let display_pct = if fraction_mode == DisplayFractionAs::Used {
            100u8.saturating_sub(rem_pct)
        } else {
            rem_pct
        };

        let hero = format!("{display_pct}%");
        let subtitle = q
            .reset_time
            .as_deref()
            .map(format_reset_countdown)
            .unwrap_or_else(|| "Ready".to_string());

        MetricDisplay {
            label,
            hero,
            bar_percent: rem_pct,
            subtitle,
        }
    }
}

pub fn format_reset_countdown(iso_str: &str) -> String {
    if let Ok(target) = DateTime::parse_from_rfc3339(iso_str) {
        let now = Utc::now();
        let target_utc = target.with_timezone(&Utc);
        let diff = target_utc.signed_duration_since(now);
        if diff.num_seconds() <= 0 {
            return "Ready".to_string();
        }
        let total_mins = diff.num_minutes();
        if total_mins < 60 {
            format!("{total_mins}m")
        } else {
            let hours = total_mins / 60;
            let mins = total_mins % 60;
            format!("{hours}h {mins}m")
        }
    } else {
        "Ready".to_string()
    }
}

fn format_count(val: u64) -> String {
    if val >= 1_000_000 {
        format!("{}M", val / 1_000_000)
    } else if val >= 1_000 {
        format!("{}k", val / 1_000)
    } else {
        val.to_string()
    }
}

pub async fn fetch(port: u16, csrf_token: &str) -> Result<UserStatusResponse, reqwest::Error> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let url = format!(
        "https://127.0.0.1:{port}/exa.language_server_pb.LanguageServerService/GetUserStatus"
    );

    client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Connect-Protocol-Version", "1")
        .header("X-Codeium-Csrf-Token", csrf_token)
        .json(&serde_json::json!({
            "metadata": {
                "ideName": "antigravity",
                "extensionName": "antigravity",
                "locale": "en"
            }
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}

pub fn render_image(display: &MetricDisplay) -> String {
    let bar_color = if display.bar_percent >= 25 {
        "#4285f4" // Google blue
    } else if display.bar_percent >= 10 {
        "#f59e0b" // Amber
    } else {
        "#ef4444" // Crimson
    };

    let bar_w = (104 * u16::from(display.bar_percent.min(100))) / 100;

    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 144 144"><rect width="144" height="144" rx="20" fill="#18181b"/><text x="72" y="34" fill="#8ab4f8" font-family="system-ui,-apple-system,sans-serif" font-size="20" font-weight="700" text-anchor="middle">{}</text><text x="72" y="80" fill="#ffffff" font-family="system-ui,-apple-system,sans-serif" font-size="42" font-weight="700" text-anchor="middle">{}</text><text x="72" y="104" fill="#a1a1aa" font-family="system-ui,-apple-system,sans-serif" font-size="16" font-weight="600" text-anchor="middle">{}</text><rect x="20" y="116" width="104" height="8" rx="4" fill="#27272a"/><rect x="20" y="116" width="{}" height="8" rx="4" fill="{}"/></svg>"##,
        display.label, display.hero, display.subtitle, bar_w, bar_color
    );

    format!("data:image/svg+xml;base64,{}", base64(svg.as_bytes()))
}

pub fn render_status(title: &str, subtitle: &str) -> String {
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 144 144"><rect width="144" height="144" rx="20" fill="#18181b"/><text x="72" y="60" fill="#ef4444" font-family="system-ui,-apple-system,sans-serif" font-size="22" font-weight="700" text-anchor="middle">{title}</text><text x="72" y="94" fill="#a1a1aa" font-family="system-ui,-apple-system,sans-serif" font-size="16" font-weight="600" text-anchor="middle">{subtitle}</text></svg>"##
    );
    format!("data:image/svg+xml;base64,{}", base64(svg.as_bytes()))
}

fn base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);

    for chunk in input.chunks(3) {
        let bytes = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        output.push(TABLE[(bytes[0] >> 2) as usize] as char);
        output.push(TABLE[(((bytes[0] & 0b11) << 4) | (bytes[1] >> 4)) as usize] as char);
        output.push(if chunk.len() > 1 {
            TABLE[(((bytes[1] & 0b1111) << 2) | (bytes[2] >> 6)) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[(bytes[2] & 0b11_1111) as usize] as char
        } else {
            '='
        });
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_user_status_live_payload_structure() {
        let raw = r#"{
            "userStatus": {
                "planStatus": {
                    "planInfo": {
                        "planName": "Pro",
                        "monthlyPromptCredits": 50000,
                        "monthlyFlowCredits": 150000
                    },
                    "availablePromptCredits": 500,
                    "availableFlowCredits": 100
                },
                "cascadeModelConfigData": {
                    "clientModelConfigs": [
                        {
                            "label": "Gemini 3.8 Flash (High)",
                            "quotaInfo": {
                                "remainingFraction": 0.59487,
                                "resetTime": "2099-01-01T00:00:00Z"
                            }
                        },
                        {
                            "label": "Claude Sonnet 4.6 (Thinking)",
                            "quotaInfo": {
                                "remainingFraction": 1.0,
                                "resetTime": "2099-01-01T00:00:00Z"
                            }
                        }
                    ]
                }
            }
        }"#;

        let parsed: UserStatusResponse = serde_json::from_str(raw).expect("valid parse");
        let gemini = parsed.display(Metric::GeminiQuota, DisplayFractionAs::Remaining);
        assert_eq!(gemini.label, "Gemini");
        assert_eq!(gemini.hero, "59%");
        assert_eq!(gemini.bar_percent, 59);

        let gemini_used = parsed.display(Metric::GeminiQuota, DisplayFractionAs::Used);
        assert_eq!(gemini_used.hero, "41%");

        let claude = parsed.display(Metric::ClaudeQuota, DisplayFractionAs::Remaining);
        assert_eq!(claude.label, "Claude");
        assert_eq!(claude.hero, "100%");
        assert_eq!(claude.bar_percent, 100);

        let credits = parsed.display(Metric::PromptCredits, DisplayFractionAs::Remaining);
        assert_eq!(credits.label, "Credits");
        assert_eq!(credits.hero, "1%");
        assert_eq!(credits.subtitle, "500 / 50k");

        let flow = parsed.display(Metric::FlowCredits, DisplayFractionAs::Remaining);
        assert_eq!(flow.label, "Flow");
        assert_eq!(flow.subtitle, "100 / 150k");
    }

    #[test]
    fn metric_serialization_deserialization() {
        assert_eq!(
            serde_json::from_str::<Metric>("\"gemini_quota\"").unwrap(),
            Metric::GeminiQuota
        );
        assert_eq!(
            serde_json::from_str::<Metric>("\"claude_quota\"").unwrap(),
            Metric::ClaudeQuota
        );
        assert_eq!(
            serde_json::from_str::<Metric>("\"prompt_credits\"").unwrap(),
            Metric::PromptCredits
        );
        assert_eq!(
            serde_json::from_str::<Metric>("\"flow_credits\"").unwrap(),
            Metric::FlowCredits
        );
    }

    #[test]
    fn renders_svg_data_uri() {
        let display = MetricDisplay {
            label: "Gemini",
            hero: "60%".to_string(),
            bar_percent: 60,
            subtitle: "1h 14m".to_string(),
        };
        let img = render_image(&display);
        assert!(img.starts_with("data:image/svg+xml;base64,"));

        let status_img = render_status("Offline", "Start AGY");
        assert!(status_img.starts_with("data:image/svg+xml;base64,"));
    }

    #[test]
    fn formats_past_reset_time_as_ready() {
        let past = "2020-01-01T00:00:00Z";
        assert_eq!(format_reset_countdown(past), "Ready");
    }
}
