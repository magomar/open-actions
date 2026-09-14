// specs/001_opencode_usage.md
use serde::{Deserialize, Serialize};

pub const USAGE_URL: &str = "https://opencode.ai/zen/go/v1/usage";

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    #[default]
    #[serde(rename = "go_5h")]
    Go5h,
    GoWeekly,
    GoMonthly,
}

impl Metric {
    pub const ALL: [Self; 3] = [Self::Go5h, Self::GoWeekly, Self::GoMonthly];

    pub fn label(self) -> &'static str {
        match self {
            Self::Go5h => "Go 5h",
            Self::GoWeekly => "Go Wk",
            Self::GoMonthly => "Go Mo",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UsageResponse {
    pub usage: UsageWindows,
}

#[derive(Debug, Deserialize)]
pub struct UsageWindows {
    pub rolling: UsageWindow,
    pub weekly: UsageWindow,
    pub monthly: UsageWindow,
}

#[derive(Debug, Deserialize)]
pub struct UsageWindow {
    pub percent: u8,
}

impl UsageResponse {
    pub fn percent(&self, metric: Metric) -> u8 {
        match metric {
            Metric::Go5h => self.usage.rolling.percent,
            Metric::GoWeekly => self.usage.weekly.percent,
            Metric::GoMonthly => self.usage.monthly.percent,
        }
    }
}

pub async fn fetch(api_key: &str) -> Result<UsageResponse, reqwest::Error> {
    reqwest::Client::new()
        .get(USAGE_URL)
        .bearer_auth(api_key)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}

pub fn image(metric: Metric, percent: u8) -> String {
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 144 144"><rect width="144" height="144" rx="18" fill="#18181b"/><text x="72" y="42" fill="#61dafb" font-family="sans-serif" font-size="22" font-weight="700" text-anchor="middle">{}</text><text x="72" y="96" fill="#fff" font-family="sans-serif" font-size="56" font-weight="700" text-anchor="middle">{}%</text><rect x="24" y="112" width="96" height="8" rx="4" fill="#3f3f46"/><rect x="24" y="112" width="{}" height="8" rx="4" fill="#61dafb"/></svg>"##,
        metric.label(),
        percent,
        96 * u16::from(percent.min(100)) / 100,
    );
    format!("data:image/svg+xml;base64,{}", base64(svg.as_bytes()))
}

pub fn status(text: &str) -> String {
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 144 144"><rect width="144" height="144" rx="18" fill="#18181b"/><text x="72" y="80" fill="#61dafb" font-family="sans-serif" font-size="20" font-weight="700" text-anchor="middle">{text}</text></svg>"##
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
    fn parses_the_documented_usage_windows() {
        let usage: UsageResponse = serde_json::from_str(
			r#"{"usage":{"rolling":{"percent":12},"weekly":{"percent":34},"monthly":{"percent":56}}}"#,
		)
		.unwrap();
        assert_eq!(usage.percent(Metric::Go5h), 12);
        assert_eq!(usage.percent(Metric::GoWeekly), 34);
        assert_eq!(usage.percent(Metric::GoMonthly), 56);
    }

    #[test]
    fn renders_usage_as_an_svg_data_image() {
        assert!(image(Metric::Go5h, 12).starts_with("data:image/svg+xml;base64,"));
    }

    #[test]
    fn deserializes_all_metric_names_from_the_inspector() {
        assert_eq!(
            serde_json::from_str::<Metric>("\"go_5h\"").unwrap(),
            Metric::Go5h
        );
        assert_eq!(
            serde_json::from_str::<Metric>("\"go_weekly\"").unwrap(),
            Metric::GoWeekly
        );
        assert_eq!(
            serde_json::from_str::<Metric>("\"go_monthly\"").unwrap(),
            Metric::GoMonthly
        );
    }

    #[test]
    fn renders_status_as_an_svg_data_image() {
        assert!(status("Configure key").starts_with("data:image/svg+xml;base64,"));
    }
}
