use crate::cloudflare_api;
use crate::cloudflare_api::EmailRoutingRuleMatcherType;
use crate::config;
use anyhow::{bail, Context};
use cloudflare_api::EmailRoutingRuleMatcher;
use rand::prelude::IteratorRandom;
use std::cmp::Reverse;

pub async fn handle_setup(email: String, api_token: String, api_key: String) -> anyhow::Result<()> {
    let config_path = config::get_config_path()?;
    let config = config::ClientConfig {
        email,
        api_token,
        api_key,
    };

    let client = cloudflare_api::Client::new(
        config.email.clone(),
        config.api_token.clone(),
        config.api_key.clone(),
    )
    .await?;

    println!("Verifying API token...");
    let response = client.verify_token().await?;

    if let Some(token) = response.result {
        if matches!(token.status, cloudflare_api::TokenStatus::Active) {
            println!(
                "Token is valid (id: {:?}, status: {:?}, expires on: {})",
                token.id,
                token.status,
                token.expires_on.unwrap_or("Never".to_string())
            );
        } else {
            bail!("Token is not active: {token:?}")
        }
    } else {
        bail!("Failed to verify token: {response:?}")
    }

    let config_content = toml::to_string(&config).context("Failed to serialize config")?;

    std::fs::create_dir_all(config_path.parent().unwrap())
        .context("Failed to create config directory")?;

    std::fs::write(&config_path, config_content)
        .with_context(|| format!("Failed to write config at {config_path:?}"))?;

    // TODO: encrypt file with password?
    // TODO: advise user that tokens are being stored in plaintext
    println!("Config saved at {}", config_path.display());

    Ok(())
}

async fn create_cf_client_from_config() -> anyhow::Result<cloudflare_api::Client> {
    let Some(config) = config::load_config()? else {
        bail!("No config found. Please run the setup command first.");
    };

    let client =
        cloudflare_api::Client::new(config.email, config.api_token, config.api_key).await?;

    Ok(client)
}

async fn select_first_zone(
    client: &cloudflare_api::Client,
) -> anyhow::Result<cloudflare_api::Zone> {
    // TODO: be able for user to select zone
    let zone = client
        .list_zones()
        .await?
        .result
        .map(|mut zones| zones.pop())
        .flatten()
        .context("No zone found")?;

    println!("Selected zone: {zone}");

    Ok(zone)
}

pub async fn handle_list_rules() -> anyhow::Result<()> {
    let client = create_cf_client_from_config().await?;

    let zone = select_first_zone(&client).await?;

    let response = client.list_email_routing_rules(&zone.id).await?;
    if let Some(mut rules) = response.result {
        if rules.is_empty() {
            println!("No rules found.");
        } else {
            println!("Rules:");
            rules.sort_by_key(|rule| Reverse(rule.priority.unwrap_or(0)));
            for rule in rules {
                println!("  - {rule}");
            }
        }
    } else {
        bail!("Failed to list rules: {response:?}")
    }

    Ok(())
}

async fn get_email_domain(
    client: &cloudflare_api::Client,
    zone_id: &str,
) -> anyhow::Result<String> {
    println!("No domain specified. Fetching it from the zone...");

    let settings = client
        .get_email_routing_settings(zone_id)
        .await?
        .result
        .context("Failed to get email routing settings")?;

    let domain = settings.name;
    println!("Found domain: {domain}");

    Ok(domain)
}

pub async fn handle_create_rule(
    matcher: Option<EmailRoutingRuleMatcher>,
    action: Option<cloudflare_api::EmailRoutingRuleAction>,
    name: Option<String>,
    priority: Option<usize>,
) -> anyhow::Result<()> {
    let client = create_cf_client_from_config().await?;

    let zone = select_first_zone(&client).await?;

    let action = match action {
        Some(action) => action,
        None => {
            // Select first address
            let addresses = client.list_destination_addresses(&zone.account.id).await?;
            let Some(mut addresses) = addresses.result else {
                bail!("Failed to list addresses: {addresses:?}")
            };

            let Some(address) = addresses.pop() else {
                bail!("No addresses found to redirect. Please create or specify one.")
            };

            let Some(email) = address.email else {
                bail!("Address {address:?} has no email")
            };

            cloudflare_api::EmailRoutingRuleAction {
                action_type: cloudflare_api::EmailRoutingRuleActionType::Forward {
                    value: vec![email],
                },
            }
        }
    };

    let matcher = match matcher {
        Some(matcher) => {
            match &matcher.matcher_type {
                EmailRoutingRuleMatcherType::All => matcher,
                EmailRoutingRuleMatcherType::Literal { value } if value.contains("@") => {
                    // TODO: maybe better email validation?
                    matcher
                }
                EmailRoutingRuleMatcherType::Literal { value } => {
                    // if there is no @, we assume the user just inputted the email's username
                    // cloudflare needs us to specify the domain as well, so fetch it
                    let domain = get_email_domain(&client, &zone.id).await?;

                    EmailRoutingRuleMatcher {
                        matcher_type: EmailRoutingRuleMatcherType::Literal {
                            value: format!("{value}@{domain}"),
                        },
                    }
                }
            }
        }
        None => {
            let domain = get_email_domain(&client, &zone.id).await?;

            let random_username = "abcdefghijklmnopqrstuvwxyz0123456789"
                .chars()
                .choose_multiple(&mut rand::rng(), 16)
                .into_iter()
                .collect::<String>();

            println!("No matcher specified. Generated random username: {random_username}");

            EmailRoutingRuleMatcher {
                matcher_type: EmailRoutingRuleMatcherType::Literal {
                    value: format!("{random_username}@{domain}"),
                },
            }
        }
    };

    let rule = cloudflare_api::CreateRoutingRuleRequest {
        actions: vec![action],
        matchers: vec![matcher],
        enabled: None,
        name,
        priority,
    };

    let response = client.create_routing_rule(&zone.id, &rule).await?;

    if let Some(rule) = response.result {
        println!("Rule created: {rule}");
    } else {
        bail!("Failed to create rule: {response:?}")
    }

    Ok(())
}

pub async fn handle_list_addresses() -> anyhow::Result<()> {
    let client = create_cf_client_from_config().await?;
    let zone = select_first_zone(&client).await?;

    let addresses = client.list_destination_addresses(&zone.account.id).await?;

    if let Some(addresses) = addresses.result {
        if addresses.is_empty() {
            println!("No addresses found.");
        } else {
            println!("Addresses:");
            for address in addresses {
                println!("  - {}", address);
            }
        }
    } else {
        bail!("Failed to list addresses: {addresses:?}")
    }

    Ok(())
}

pub async fn handle_delete_rule(rule_identifier: String) -> anyhow::Result<()> {
    let client = create_cf_client_from_config().await?;
    let zone = select_first_zone(&client).await?;

    let response = client.list_email_routing_rules(&zone.id).await?;

    let rule_identifier = match response.result {
        None => {
            println!(
                "Fetching rule identifier failed. Assuming user provided an existing rule ID."
            );
            rule_identifier
        }
        Some(rules) => {
            fn string_kinda_matches(input: &str, other: &str) -> bool {
                other.to_lowercase().contains(&input.to_lowercase())
            }

            let matched_rules = rules
                .iter()
                .filter(|rule| {
                    string_kinda_matches(&rule_identifier, &rule.id)
                        || rule.matchers.iter().any(|matcher| match matcher {
                            EmailRoutingRuleMatcher {
                                matcher_type: EmailRoutingRuleMatcherType::All,
                            } => false, // catch-all rules can't match
                            EmailRoutingRuleMatcher {
                                matcher_type: EmailRoutingRuleMatcherType::Literal { value },
                            } => string_kinda_matches(&rule_identifier, value),
                        })
                })
                .collect::<Vec<_>>();

            let rule_identifier = match matched_rules.as_slice() {
                [] => {
                    println!("No rules found with identifier {rule_identifier}.");
                    println!("Available rules:");
                    for rule in &rules {
                        println!("  - {rule}");
                    }
                    return Ok(());
                }
                [rule] => {
                    println!("Found rule: {rule}");
                    rule.id.clone()
                }
                rules => {
                    println!("Multiple rules found with identifier {rule_identifier}:");
                    for rule in rules {
                        println!("  - {rule}");
                    }
                    println!("Please specify a unique identifier.");
                    return Ok(());
                }
            };

            rule_identifier
        }
    };

    let response = client
        .delete_routing_rule(&zone.id, &rule_identifier)
        .await?;

    if response.success {
        println!("Rule deleted successfully.");
    } else {
        bail!("Failed to delete rule: {response:?}");
    }
    Ok(())
}

pub async fn handle_list_zones() -> anyhow::Result<()> {
    let client = create_cf_client_from_config().await?;

    let response = client.list_zones().await?;

    if let Some(zones) = response.result {
        if zones.is_empty() {
            println!("No zones found.");
        } else {
            println!("Zones:");
            for zone in zones {
                println!("  - {zone}");
            }
        }
    } else {
        bail!("Failed to list zones: {response:?}")
    }

    Ok(())
}

fn write_vec<T: std::fmt::Display>(f: &mut std::fmt::Formatter<'_>, vec: &[T]) -> std::fmt::Result {
    for (i, item) in vec.iter().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        write!(f, "{}", item)?;
    }
    Ok(())
}

impl std::fmt::Display for cloudflare_api::EmailRoutingRuleAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.action_type {
            cloudflare_api::EmailRoutingRuleActionType::Drop => {
                write!(f, "Drop")
            }
            cloudflare_api::EmailRoutingRuleActionType::Forward { value } => {
                write!(f, "Forward to {}", value.join(", "))
            }
            cloudflare_api::EmailRoutingRuleActionType::Worker { value } => {
                write!(f, "Worker ({})", value.join(", "))
            }
        }
    }
}

impl std::fmt::Display for EmailRoutingRuleMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.matcher_type {
            cloudflare_api::EmailRoutingRuleMatcherType::All => {
                write!(f, "* (catch-all)")
            }
            cloudflare_api::EmailRoutingRuleMatcherType::Literal { value } => {
                write!(f, "{}", value)
            }
        }
    }
}

impl std::fmt::Display for cloudflare_api::EmailRoutingRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write_vec(f, &self.matchers)?;
        write!(f, " -> ")?;
        write_vec(f, &self.actions)?;
        write!(f, " (ID: {}", self.id)?;
        if let Some(name) = &self.name {
            if !name.is_empty() {
                write!(f, ", Name: {name}")?;
            }
        }

        if !self.enabled {
            write!(f, ", Disabled")?;
        }

        if let Some(priority) = self.priority {
            if priority != 0 {
                write!(f, ", Priority: {}", priority)?;
            }
        }

        write!(f, ")")?;

        Ok(())
    }
}

impl std::fmt::Display for cloudflare_api::Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(email) = &self.email {
            write!(f, "{}", email)?;
        }

        if let Some(id) = &self.id {
            write!(f, " (id = {})", id)?;
        }

        Ok(())
    }
}

impl std::fmt::Display for cloudflare_api::Zone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (id = {})", self.account.name, self.id)
    }
}
