use crate::{api::Client, emit};
use anyhow::Result;
use clap::Subcommand;
use lain_wire::{
    ChannelId, IncomingId,
    source::{InspectSource, SourceCursor, SourceQuery, SourceWindow},
};
use serde_json::{Value, json};

#[derive(Subcommand)]
pub enum Commands {
    List,
    Show {
        id: ChannelId,
    },
    /// Inspect the independent private-memory authorization; default is disabled.
    MemoryGrant {
        id: ChannelId,
    },
    /// Explicit administrative decision, never available as an Actor tool.
    MemoryEnable {
        id: ChannelId,
        #[arg(long)]
        revision: u64,
        #[arg(long)]
        expires_at_ms: Option<u64>,
    },
    MemoryRevoke {
        id: ChannelId,
        #[arg(long)]
        revision: u64,
    },
    MemoryGrants {
        id: ChannelId,
    },
    MemoryHistory {
        id: ChannelId,
        #[arg(long, default_value_t = 0)]
        after: u64,
    },
    /// Independent, default-off authorization to read a fixed received-UTC history window.
    SourceGrant {
        id: ChannelId,
    },
    SourceEnable {
        id: ChannelId,
        #[arg(long)]
        revision: u64,
        #[arg(long)]
        from_ms: u64,
        #[arg(long)]
        to_ms: u64,
        #[arg(long)]
        expires_at_ms: Option<u64>,
    },
    SourceRevoke {
        id: ChannelId,
        #[arg(long)]
        revision: u64,
    },
    SourceGrants {
        id: ChannelId,
    },
    SourceSearch {
        id: ChannelId,
        #[arg(long)]
        text: Option<String>,
        #[arg(long, requires = "to_ms")]
        from_ms: Option<u64>,
        #[arg(long, requires = "from_ms")]
        to_ms: Option<u64>,
        /// Exact JSON next_cursor returned by the previous page.
        #[arg(long)]
        after: Option<String>,
        #[arg(long, default_value_t = 3)]
        limit: u16,
    },
    SourceInspect {
        id: ChannelId,
        incoming_id: IncomingId,
        #[arg(long, requires = "to_ms")]
        from_ms: Option<u64>,
        #[arg(long, requires = "from_ms")]
        to_ms: Option<u64>,
    },
    SourceReads {
        id: ChannelId,
        #[arg(long, default_value_t = 0)]
        after: u64,
    },
}
pub async fn run(client: &Client, command: Commands, machine: bool) -> Result<()> {
    let value:Value=match command {
        Commands::SourceGrant { id } => client.get(&format!("v1/channels/{id}/source-grant")).await?,
        Commands::SourceEnable { id, revision, from_ms, to_ms, expires_at_ms } => client.post(&format!("v1/channels/{id}/source-grant"),Some(json!({"enabled":true,"expected_revision":revision,"window":{"from_received_at_ms":from_ms,"to_received_at_ms":to_ms},"expires_at_ms":expires_at_ms}))).await?,
        Commands::SourceRevoke { id, revision } => client.post(&format!("v1/channels/{id}/source-grant"),Some(json!({"enabled":false,"expected_revision":revision,"window":null,"expires_at_ms":null}))).await?,
        Commands::SourceGrants { id } => client.get(&format!("v1/channels/{id}/source-grant/history")).await?,
        Commands::SourceReads { id, after } => client.get(&format!("v1/channels/{id}/source-reads?after={after}")).await?,
        Commands::SourceSearch { id, text, from_ms, to_ms, after, limit } => {
            let after = after.map(|value| serde_json::from_str::<SourceCursor>(&value).map_err(|_| anyhow::anyhow!("invalid source cursor JSON"))).transpose()?;
            let query = SourceQuery { text, window: window(from_ms,to_ms), after, limit };
            client.post(&format!("v1/channels/{id}/sources/search"),Some(serde_json::to_value(query)?)).await?
        },
        Commands::SourceInspect { id, incoming_id, from_ms, to_ms } => client.post(&format!("v1/channels/{id}/sources/inspect"),Some(serde_json::to_value(InspectSource { incoming_id, window: window(from_ms,to_ms) })?)).await?,
        Commands::List => client.get("v1/channels/status").await?,
        Commands::Show { id } => client.get(&format!("v1/channels/{id}")).await?,
        Commands::MemoryGrant { id } => client.get(&format!("v1/channels/{id}/memory-grant")).await?,
        Commands::MemoryEnable { id,revision,expires_at_ms } => client.post(&format!("v1/channels/{id}/memory-grant"),Some(json!({"enabled":true,"expected_revision":revision,"expires_at_ms":expires_at_ms}))).await?,
        Commands::MemoryRevoke { id,revision } => client.post(&format!("v1/channels/{id}/memory-grant"),Some(json!({"enabled":false,"expected_revision":revision,"expires_at_ms":null}))).await?,
        Commands::MemoryGrants { id } => client.get(&format!("v1/channels/{id}/memory-grant/history")).await?,
        Commands::MemoryHistory { id,after } => client.get(&format!("v1/channels/{id}/memory-effects?after={after}")).await?,
    };
    emit(&value, machine)
}

fn window(from_ms: Option<u64>, to_ms: Option<u64>) -> Option<SourceWindow> {
    from_ms
        .zip(to_ms)
        .map(|(from_received_at_ms, to_received_at_ms)| SourceWindow {
            from_received_at_ms,
            to_received_at_ms,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[tokio::test]
    async fn parsed_channel_commands_use_authenticated_management_contracts() {
        use axum::{Router, body::to_bytes, extract::Request};
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let app = Router::new().fallback(move |request: Request| {
            let tx = tx.clone();
            async move {
                assert_eq!(
                    request.headers()["authorization"],
                    "Bearer fixture-owner-token-0123456789abcdef"
                );
                let method = request.method().to_string();
                let path = request.uri().to_string();
                let bytes = to_bytes(request.into_body(), 4096).await.unwrap();
                tx.send((
                    method,
                    path,
                    serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null),
                ))
                .unwrap();
                axum::Json(json!({"fixture":true}))
            }
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let client = Client::new(
            &format!("http://{}", listener.local_addr().unwrap()),
            "fixture-owner-token-0123456789abcdef",
        )
        .unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let id = ChannelId::new().to_string();
        let incoming = IncomingId::new().to_string();
        for (args, path) in [
            (
                vec!["source-grant", &id],
                format!("v1/channels/{id}/source-grant"),
            ),
            (
                vec![
                    "source-enable",
                    &id,
                    "--revision",
                    "0",
                    "--from-ms",
                    "1000",
                    "--to-ms",
                    "2000",
                    "--expires-at-ms",
                    "3000",
                ],
                format!("v1/channels/{id}/source-grant"),
            ),
            (
                vec!["source-revoke", &id, "--revision", "1"],
                format!("v1/channels/{id}/source-grant"),
            ),
            (
                vec!["source-grants", &id],
                format!("v1/channels/{id}/source-grant/history"),
            ),
            (
                vec![
                    "source-search",
                    &id,
                    "--text",
                    "word",
                    "--limit",
                    "2",
                    "--from-ms",
                    "1000",
                    "--to-ms",
                    "2000",
                ],
                format!("v1/channels/{id}/sources/search"),
            ),
            (
                vec!["source-inspect", &id, &incoming],
                format!("v1/channels/{id}/sources/inspect"),
            ),
            (
                vec!["source-reads", &id, "--after", "17"],
                format!("v1/channels/{id}/source-reads?after=17"),
            ),
            (vec!["list"], "v1/channels/status".into()),
            (vec!["show", &id], format!("v1/channels/{id}")),
            (
                vec!["memory-grant", &id],
                format!("v1/channels/{id}/memory-grant"),
            ),
            (
                vec![
                    "memory-enable",
                    &id,
                    "--revision",
                    "0",
                    "--expires-at-ms",
                    "2000000000000",
                ],
                format!("v1/channels/{id}/memory-grant"),
            ),
            (
                vec!["memory-revoke", &id, "--revision", "1"],
                format!("v1/channels/{id}/memory-grant"),
            ),
            (
                vec!["memory-grants", &id],
                format!("v1/channels/{id}/memory-grant/history"),
            ),
            (
                vec!["memory-history", &id, "--after", "17"],
                format!("v1/channels/{id}/memory-effects?after=17"),
            ),
        ] {
            let mut argv = vec!["lain", "--json", "channel"];
            argv.extend(args.clone());
            let parsed = crate::Args::try_parse_from(argv).unwrap();
            let Some(crate::Command::Channel { command }) = parsed.command else {
                panic!("wrong parsed command")
            };
            run(&client, command, true).await.unwrap();
            let (method, actual, body) = rx.recv().await.unwrap();
            assert_eq!(actual, format!("/{path}"));
            match args[0] {
                "source-enable" => {
                    assert_eq!(method, "POST");
                    assert_eq!(
                        body,
                        json!({"enabled":true,"expected_revision":0,"window":{"from_received_at_ms":1000,"to_received_at_ms":2000},"expires_at_ms":3000})
                    );
                }
                "source-revoke" => {
                    assert_eq!(method, "POST");
                    assert_eq!(
                        body,
                        json!({"enabled":false,"expected_revision":1,"window":null,"expires_at_ms":null})
                    );
                }
                "source-search" => {
                    assert_eq!(method, "POST");
                    assert_eq!(
                        body,
                        json!({"text":"word","window":{"from_received_at_ms":1000,"to_received_at_ms":2000},"after":null,"limit":2})
                    );
                }
                "source-inspect" => {
                    assert_eq!(method, "POST");
                    assert_eq!(body, json!({"incoming_id":incoming,"window":null}));
                }
                "memory-enable" => {
                    assert_eq!(method, "POST");
                    assert_eq!(
                        body,
                        json!({"enabled":true,"expected_revision":0,"expires_at_ms":2000000000000u64})
                    );
                }
                "memory-revoke" => {
                    assert_eq!(method, "POST");
                    assert_eq!(
                        body,
                        json!({"enabled":false,"expected_revision":1,"expires_at_ms":null})
                    );
                }
                _ => {
                    assert_eq!(method, "GET");
                    assert!(body.is_null());
                }
            }
        }
        assert!(crate::Args::try_parse_from(["lain", "channel", "memory-enable", &id]).is_err());
        assert!(
            crate::Args::try_parse_from([
                "lain",
                "channel",
                "source-enable",
                &id,
                "--revision",
                "0"
            ])
            .is_err()
        );
        assert!(
            crate::Args::try_parse_from([
                "lain",
                "channel",
                "source-search",
                &id,
                "--from-ms",
                "1000"
            ])
            .is_err()
        );
        assert!(
            crate::Args::try_parse_from(["lain", "memory", "list", "--channel", &id, "--global"])
                .is_err()
        );
        server.abort();
    }
}
