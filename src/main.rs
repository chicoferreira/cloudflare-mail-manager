use clap::{Parser, Subcommand};

mod cloudflare_api;
mod command;
mod config;

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Setup {
        email: String,
        api_token: String,
        api_key: String,
    },
    List,
    Addresses,
    Zones,
    Create {
        matcher: Option<cloudflare_api::EmailRoutingRuleMatcher>,
        action: Option<cloudflare_api::EmailRoutingRuleAction>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        priority: Option<usize>,
    },
    Delete {
        identifier: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Setup {
            email,
            api_token,
            api_key,
        } => {
            command::handle_setup(email, api_token, api_key).await?;
        }
        Command::List => command::handle_list_rules().await?,
        Command::Addresses => command::handle_list_addresses().await?,
        Command::Create {
            matcher,
            action,
            name,
            priority,
        } => command::handle_create_rule(matcher, action, name, priority).await?,
        Command::Delete { identifier } => {
            command::handle_delete_rule(identifier).await?;
        },
        Command::Zones => command::handle_list_zones().await?,
    }

    Ok(())
}
