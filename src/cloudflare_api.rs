use anyhow::Context;
use reqwest::{Method, RequestBuilder};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize, Serializer};
use std::str::FromStr;

const API_BASE_URL: &str = "https://api.cloudflare.com/client/v4";

pub struct Client {
    client: reqwest::Client,
    email: String,
    api_token: String,
    api_key: String,
}

impl Client {
    pub async fn new(email: String, api_token: String, api_key: String) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .build()
            .context("Failed to create client")?;

        Ok(Client {
            client,
            email,
            api_token,
            api_key,
        })
    }

    pub async fn verify_token(&self) -> anyhow::Result<Response<VerifyTokenResult>> {
        let url = "/user/tokens/verify";
        self.send_get(url).await
    }

    pub async fn list_zones(&self) -> anyhow::Result<Response<Vec<Zone>>> {
        let url = "/zones";
        self.send_get(url).await
    }

    pub async fn get_email_routing_settings(
        &self,
        zone_id: &str,
    ) -> anyhow::Result<Response<EmailRoutingSettings>> {
        let url = format!("/zones/{zone_id}/email/routing");
        self.send_get(&url).await
    }

    pub async fn list_email_routing_rules(
        &self,
        zone_id: &str,
    ) -> anyhow::Result<Response<Vec<EmailRoutingRule>>> {
        let url = format!("/zones/{zone_id}/email/routing/rules");
        self.send_get(&url).await
    }

    pub async fn create_routing_rule(
        &self,
        zone_id: &str,
        rule: &CreateRoutingRuleRequest,
    ) -> anyhow::Result<Response<EmailRoutingRule>> {
        let url = format!("/zones/{zone_id}/email/routing/rules");
        self.send(Method::POST, &url, rule).await
    }

    pub async fn list_destination_addresses(
        &self,
        account_id: &str,
    ) -> anyhow::Result<Response<Vec<Address>>> {
        let url = format!("/accounts/{account_id}/email/routing/addresses");
        self.send_get(&url).await
    }

    pub async fn delete_routing_rule(
        &self,
        zone_id: &str,
        rule_identifier: &str,
    ) -> anyhow::Result<Response<EmailRoutingRule>> {
        let url = format!("/zones/{zone_id}/email/routing/rules/{rule_identifier}");
        self.send(Method::DELETE, &url, &()).await
    }

    async fn send_get<T: DeserializeOwned>(&self, url: &str) -> anyhow::Result<Response<T>> {
        self.send(Method::GET, url, &()).await
    }

    async fn send<B: Serialize, T: DeserializeOwned>(
        &self,
        method: Method,
        url: &str,
        body: &B,
    ) -> anyhow::Result<Response<T>> {
        self.add_auth_headers(self.client.request(method, format!("{API_BASE_URL}{url}")))
            .json(body)
            .send()
            .await?
            .json::<Response<T>>()
            .await
            .context("Couldn't parse json response")
    }

    fn add_auth_headers(&self, request_builder: RequestBuilder) -> RequestBuilder {
        request_builder
            .header("Authorization", format!("Bearer {}", &self.api_token))
            .header("X-Auth-Email", &self.email)
            .header("X-Auth-Key", &self.api_key)
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Response<R> {
    #[serde(default)]
    pub errors: Vec<RequestError>,
    #[serde(default)]
    pub messages: Vec<ResponseInfo>,
    pub success: bool,
    pub result: Option<R>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct ResponseInfo {
    pub code: usize,
    pub message: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct RequestError {
    pub code: usize,
    pub message: String,
    #[serde(default)]
    pub error_chain: Vec<RequestError>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct VerifyTokenResult {
    pub id: String,
    pub status: TokenStatus,
    pub expires_on: Option<String>,
    pub not_before: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TokenStatus {
    Active,
    Disabled,
    Expired,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct EmailRoutingSettings {
    pub id: String,
    pub enabled: bool,
    pub name: String,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub status: Option<EmailRoutingStatus>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum EmailRoutingStatus {
    Ready,
    Unconfigured,
    Misconfigured,
    #[serde(rename = "misconfigured/locked")]
    MisconfiguredOrLocked,
    Unlocked,
}

#[derive(Deserialize, Debug)]
pub struct Zone {
    pub id: String,
    pub account: ZoneAccount,
}

#[derive(Deserialize, Debug)]
pub struct ZoneAccount {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct EmailRoutingRule {
    pub id: String,
    #[serde(default)]
    pub actions: Vec<EmailRoutingRuleAction>,
    pub enabled: bool,
    #[serde(default)]
    pub matchers: Vec<EmailRoutingRuleMatcher>,
    pub name: Option<String>,
    pub priority: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmailRoutingRuleAction {
    #[serde(flatten)]
    pub action_type: EmailRoutingRuleActionType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub enum EmailRoutingRuleActionType {
    Drop,
    Forward { value: Vec<String> },
    Worker { value: Vec<String> },
}

impl FromStr for EmailRoutingRuleAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "drop" => Ok(EmailRoutingRuleAction {
                action_type: EmailRoutingRuleActionType::Drop,
            }),
            _ => Ok(EmailRoutingRuleAction {
                action_type: EmailRoutingRuleActionType::Forward {
                    value: vec![s.to_string()],
                },
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmailRoutingRuleMatcher {
    #[serde(flatten)]
    pub matcher_type: EmailRoutingRuleMatcherType,
}

impl FromStr for EmailRoutingRuleMatcher {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "*" => Ok(EmailRoutingRuleMatcher {
                matcher_type: EmailRoutingRuleMatcherType::All,
            }),
            _ => Ok(EmailRoutingRuleMatcher {
                matcher_type: EmailRoutingRuleMatcherType::Literal {
                    value: s.to_string(),
                },
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type")]
pub enum EmailRoutingRuleMatcherType {
    All,
    #[serde(serialize_with = "serialize_literal")]
    Literal {
        value: String,
    },
}

fn serialize_literal<S>(value: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use serde::ser::SerializeStruct;
    let mut lit = serializer.serialize_struct("Literal", 2)?;
    lit.serialize_field("value", value)?;
    lit.serialize_field("field", "to")?;
    lit.end()
}

#[derive(Serialize, Debug, Default)]
pub struct CreateRoutingRuleRequest {
    pub actions: Vec<EmailRoutingRuleAction>,
    pub matchers: Vec<EmailRoutingRuleMatcher>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Address {
    pub id: Option<String>,
    pub created: Option<String>,
    pub email: Option<String>,
    pub modified: Option<String>,
    pub tag: Option<String>,
    pub verified: Option<String>,
}
