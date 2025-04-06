# cloudflare-mail-manager

A simple command-line tool to manage email redirect rules on Cloudflare, using
the [Cloudflare API](https://developers.cloudflare.com/api/).
Initially created to generate random temporary email addresses, it has evolved into a versatile utility for creating,
listing, and deleting email routing rules.

## Installation

Install [Rust](https://www.rust-lang.org/), clone the repository and install the tool:

```bash
git clone https://github.com/chicoferreira/cloudflare-mail-manager
cd cloudflare-mail-manager
cargo install --path .
```

This will install the `cloudflare-mail-manager` binary globally on your system.
If you don't want this and only want to run the tool, you can run it directly with `cargo run -- <args>`.

## Setup Credentials

Before using the tool, configure it with your Cloudflare credentials using the `setup` command. You'll need:

1. **API Token**: Create one at [Cloudflare API Tokens](https://dash.cloudflare.com/profile/api-tokens) with these
   permissions:
    - `Account: Email Routing Addresses (Edit)`
    - `Zone: Email Routing Rules (Edit)`
    - `Zone: Zone Settings (Edit)`
    - `Zone: Zone (Edit)`

2. **API Key**: Obtain your Global API Key from [Cloudflare API Tokens](https://dash.cloudflare.com/profile/api-tokens).

3. **Email Address**: Use the email associated with your Cloudflare account.

Now you can run the `setup` command:

```bash
cloudflare-mail-manager setup [EMAIL] [API_TOKEN] [API_KEY]
```

> [!WARNING]  
> This will store the credentials in a plain text on the config folder on your home directory.
> For now, there is no way to safely store the credentials.

## Usage

```bash
cloudflare-mail-manager [COMMAND]
```

### Commands

#### `list`

Lists all email redirect rules for the selected zone.

```
$ cloudflare-mail-manager list
Selected zone: REDACTED Account (id = REDACTED)
Rules:
- test@mail.com -> Forward to REDACTED (ID: REDACTED)
- tb5refv6zj7lmu1p@mail.com -> Forward to mypersonalemail@mail.com (ID: REDACTED, Name: REDACTED)
```

#### `create [MATCHER] [FORWARD TO] --name [NAME] --priority [PRIORITY]`

Creates a new email redirect rule. Supports multiple use cases:

- **Random email:** Generate a random address forwarding to the default destination.
    ```
    $ cloudflare-mail-manager create
    Selected zone: REDACTED Account (id = REDACTED)
    No domain specified. Fetching it from the zone...
    Found domain: mail.com
    No matcher specified. Generated random username: tb5refv6zj7lmu1p
    Rule created: tb5refv6zj7lmu1p@mail.com -> Forward to mypersonalemail@mail.com (ID: REDACTED)
    ```
- **Custom username:** Specify a username without a domain.
    ```
    $ cloudflare-mail-manager create test
    Selected zone: REDACTED Account (id = REDACTED)
    No domain specified. Fetching it from the zone...
    Found domain: mail.com
    Rule created: test@mail.com -> Forward to mypersonalemail@mail.com (ID: REDACTED)
    ```
- **Full email:** Specify the full email address.
    ```
    $ cloudflare-mail-manager create test@mail.com
    Selected zone: REDACTED Account (id = REDACTED)
    Rule created: test@mail.com -> Forward to mypersonalemail@mail.com (ID: REDACTED)
    ```
- **Custom forward:** Specify both matcher and forward destination.
    ```
    $ cloudflare-mail-manager create test@mail.com mypersonalemail@mail.com
    Selected zone: REDACTED Account (id = REDACTED)
    Rule created: test@mail.com -> Forward to mypersonalemail@mail.com (ID: REDACTED)
    ```

#### `delete [PARTS OF IDENTIFIER OR MATCHER]`

Deletes a rule by matching its ID or email matcher (partial matches supported).

  ```
  $ cloudflare-mail-manager delete youtube
  Selected zone: REDACTED Account (id = REDACTED)
  Found rule: youtube2@mail.com -> Forward to mypersonalemail@mail.com (ID: REDACTED)
  Rule deleted successfully.
  ```

#### `zones`

Lists all zones associated with your Cloudflare account.

  ```
  $ cloudflare-mail-manager zones
  Zones:
  - REDACTED Account (id = REDACTED)
  ```

#### `addresses`

Lists all destination email addresses.

  ```
  $ cloudflare-mail-manager addresses
  Selected zone: REDACTED Account (id = REDACTED)
  Addresses:
  - mypersonalemail@mail.com (id = REDACTED)
  ```
