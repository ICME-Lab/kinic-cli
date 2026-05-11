use std::fs;

use anyhow::{Result, bail};

use crate::{
    cli::{
        WikiAppendArgs, WikiArgs, WikiChildrenArgs, WikiCommand, WikiDatabaseCommand,
        WikiDatabaseRoleArg, WikiDeleteArgs, WikiEditArgs, WikiJsonArgs, WikiNodeKindArg,
        WikiReadArgs, WikiSearchArgs, WikiWriteArgs,
    },
    commands::CommandContext,
    wiki_bridge::{
        AppendNodeRequest, DatabaseRole, DeleteNodeRequest, EditNodeRequest, ListChildrenRequest,
        NodeKind, SearchNodesRequest, SearchPreviewMode, WikiClient, WriteNodeRequest,
        wiki_canister_id_from_env,
    },
};

pub async fn handle(args: WikiArgs, ctx: &CommandContext) -> Result<()> {
    let agent = ctx.agent_factory.build().await?;
    let client = WikiClient::new(agent, wiki_canister_id_from_env())?;
    match args.command {
        WikiCommand::Database(database) => match database.command {
            WikiDatabaseCommand::List(args) => list_databases(&client, args).await,
            WikiDatabaseCommand::Create(args) => create_database(&client, args).await,
            WikiDatabaseCommand::Grant(args) => grant_database(&client, args).await,
        },
        WikiCommand::Read(args) => read_node(&client, args).await,
        WikiCommand::Children(args) => list_children(&client, args).await,
        WikiCommand::Search(args) => search_nodes(&client, args).await,
        WikiCommand::Write(args) => write_node(&client, args).await,
        WikiCommand::Append(args) => append_node(&client, args).await,
        WikiCommand::Edit(args) => edit_node(&client, args).await,
        WikiCommand::Delete(args) => delete_node(&client, args).await,
    }
}

async fn list_databases(client: &WikiClient, args: WikiJsonArgs) -> Result<()> {
    let databases = client.list_databases().await?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&databases)?);
    } else {
        for database in databases {
            println!(
                "{}\t{:?}\t{:?}\t{}",
                database.database_id, database.status, database.role, database.logical_size_bytes
            );
        }
    }
    Ok(())
}

async fn create_database(client: &WikiClient, args: WikiJsonArgs) -> Result<()> {
    let database_id = client.create_database().await?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "database_id": database_id }))?
        );
    } else {
        println!("{database_id}");
    }
    Ok(())
}

async fn grant_database(
    client: &WikiClient,
    args: crate::cli::WikiDatabaseGrantArgs,
) -> Result<()> {
    let role = database_role(args.role);
    client
        .grant_database_access(&args.database_id, &args.principal, role)
        .await?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "database_id": args.database_id,
                "principal": args.principal,
                "role": format!("{role:?}"),
            }))?
        );
    } else {
        println!(
            "granted {:?} {} on {}",
            role, args.principal, args.database_id
        );
    }
    Ok(())
}

async fn read_node(client: &WikiClient, args: WikiReadArgs) -> Result<()> {
    let Some(node) = client.read_node(&args.database_id, &args.path).await? else {
        bail!("node not found: {}", args.path);
    };
    if args.json {
        println!("{}", serde_json::to_string_pretty(&node)?);
    } else {
        println!("{}", node.content);
    }
    Ok(())
}

async fn list_children(client: &WikiClient, args: WikiChildrenArgs) -> Result<()> {
    let children = client
        .list_children(ListChildrenRequest {
            database_id: args.database_id,
            path: args.path,
        })
        .await?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&children)?);
    } else {
        for child in children {
            println!(
                "{}\t{:?}\t{}",
                child.path,
                child.kind,
                child.etag.unwrap_or_default()
            );
        }
    }
    Ok(())
}

async fn search_nodes(client: &WikiClient, args: WikiSearchArgs) -> Result<()> {
    let hits = client
        .search_nodes(SearchNodesRequest {
            database_id: args.database_id,
            query_text: args.query,
            prefix: Some(args.prefix),
            top_k: args.top_k,
            preview_mode: Some(SearchPreviewMode::ContentStart),
        })
        .await?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&hits)?);
    } else {
        for hit in hits {
            let snippet = hit
                .preview
                .and_then(|preview| preview.excerpt)
                .or(hit.snippet)
                .unwrap_or_default();
            println!("{:.3}\t{}\t{}", hit.score, hit.path, snippet);
        }
    }
    Ok(())
}

async fn write_node(client: &WikiClient, args: WikiWriteArgs) -> Result<()> {
    let kind = node_kind(args.kind);
    validate_source_path(&args.path, &kind)?;
    let content = fs::read_to_string(&args.input)?;
    let result = client
        .write_node(WriteNodeRequest {
            database_id: args.database_id,
            path: args.path,
            kind,
            content,
            metadata_json: args.metadata_json,
            expected_etag: args.expected_etag,
        })
        .await?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}", result.node.etag);
    }
    Ok(())
}

async fn append_node(client: &WikiClient, args: WikiAppendArgs) -> Result<()> {
    let kind = args.kind.map(node_kind);
    if let Some(kind) = kind.as_ref() {
        validate_source_path(&args.path, kind)?;
    }
    let json = args.json;
    let content = fs::read_to_string(&args.input)?;
    let result = client
        .append_node(append_request(args, content, kind))
        .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}", result.node.etag);
    }
    Ok(())
}

fn append_request(
    args: WikiAppendArgs,
    content: String,
    kind: Option<NodeKind>,
) -> AppendNodeRequest {
    AppendNodeRequest {
        database_id: args.database_id,
        path: args.path,
        content,
        expected_etag: args.expected_etag,
        separator: args.separator,
        metadata_json: args.metadata_json,
        kind,
    }
}

async fn edit_node(client: &WikiClient, args: WikiEditArgs) -> Result<()> {
    let result = client
        .edit_node(EditNodeRequest {
            database_id: args.database_id,
            path: args.path,
            old_text: args.old_text,
            new_text: args.new_text,
            expected_etag: args.expected_etag,
            replace_all: args.replace_all,
        })
        .await?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}\t{}", result.replacement_count, result.node.etag);
    }
    Ok(())
}

async fn delete_node(client: &WikiClient, args: WikiDeleteArgs) -> Result<()> {
    if !args.yes {
        bail!("--yes is required to delete a wiki node");
    }
    let result = client
        .delete_node(DeleteNodeRequest {
            database_id: args.database_id,
            path: args.path,
            expected_etag: args.expected_etag,
        })
        .await?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}", result.path);
    }
    Ok(())
}

fn database_role(role: WikiDatabaseRoleArg) -> DatabaseRole {
    match role {
        WikiDatabaseRoleArg::Owner => DatabaseRole::Owner,
        WikiDatabaseRoleArg::Writer => DatabaseRole::Writer,
        WikiDatabaseRoleArg::Reader => DatabaseRole::Reader,
    }
}

fn node_kind(kind: WikiNodeKindArg) -> NodeKind {
    match kind {
        WikiNodeKindArg::File => NodeKind::File,
        WikiNodeKindArg::Source => NodeKind::Source,
    }
}

fn validate_source_path(path: &str, kind: &NodeKind) -> Result<()> {
    let is_source_path = path_matches_prefix_boundary(path, "/Sources/raw")
        || path_matches_prefix_boundary(path, "/Sources/sessions");
    if *kind != NodeKind::Source {
        if is_source_path {
            bail!(
                "source path must use source kind under /Sources/raw or /Sources/sessions: {path}"
            );
        }
        return Ok(());
    }
    if path_matches_prefix_boundary(path, "/Sources/raw") {
        return validate_source_path_under_prefix(path, "/Sources/raw");
    }
    if path_matches_prefix_boundary(path, "/Sources/sessions") {
        return validate_source_path_under_prefix(path, "/Sources/sessions");
    }
    bail!("source path must stay under /Sources/raw or /Sources/sessions: {path}");
}

fn path_matches_prefix_boundary(path: &str, prefix: &str) -> bool {
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn validate_source_path_under_prefix(path: &str, prefix: &str) -> Result<()> {
    let relative = path
        .strip_prefix(prefix)
        .ok_or_else(|| anyhow::anyhow!("source path must stay under {prefix}: {path}"))?;
    let segments = relative
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.len() != 2 {
        bail!("source path must use canonical form {prefix}/<id>/<id>.md: {path}");
    }
    let [directory_name, file_name] = segments.as_slice() else {
        unreachable!();
    };
    if directory_name.is_empty() || *file_name != format!("{directory_name}.md") {
        bail!("source path must use canonical form {prefix}/<id>/<id>.md: {path}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Command, WikiDatabaseCommand};
    use clap::Parser;

    #[test]
    fn wiki_read_command_parses() {
        let cli = Cli::try_parse_from([
            "kinic-cli",
            "--identity",
            "alice",
            "wiki",
            "read",
            "--database-id",
            "db",
            "--path",
            "/Wiki/a.md",
        ])
        .expect("wiki read should parse");

        match cli.command {
            Command::Wiki(args) => match args.command {
                crate::cli::WikiCommand::Read(args) => {
                    assert_eq!(args.database_id, "db");
                    assert_eq!(args.path, "/Wiki/a.md");
                }
                _ => panic!("expected wiki read"),
            },
            _ => panic!("expected wiki command"),
        }
    }

    #[test]
    fn wiki_database_grant_command_parses_role() {
        let cli = Cli::try_parse_from([
            "kinic-cli",
            "--identity",
            "alice",
            "wiki",
            "database",
            "grant",
            "db",
            "aaaaa-aa",
            "reader",
        ])
        .expect("wiki grant should parse");

        match cli.command {
            Command::Wiki(args) => match args.command {
                crate::cli::WikiCommand::Database(database) => match database.command {
                    WikiDatabaseCommand::Grant(args) => {
                        assert_eq!(args.role, WikiDatabaseRoleArg::Reader);
                        assert!(!args.json);
                    }
                    _ => panic!("expected wiki database grant"),
                },
                _ => panic!("expected wiki database"),
            },
            _ => panic!("expected wiki command"),
        }
    }

    #[test]
    fn wiki_database_grant_accepts_json() {
        let cli = Cli::try_parse_from([
            "kinic-cli",
            "--identity",
            "alice",
            "wiki",
            "database",
            "grant",
            "db",
            "aaaaa-aa",
            "reader",
            "--json",
        ])
        .expect("wiki grant json should parse");

        match cli.command {
            Command::Wiki(args) => match args.command {
                crate::cli::WikiCommand::Database(database) => match database.command {
                    WikiDatabaseCommand::Grant(args) => assert!(args.json),
                    _ => panic!("expected wiki database grant"),
                },
                _ => panic!("expected wiki database"),
            },
            _ => panic!("expected wiki command"),
        }
    }

    #[test]
    fn wiki_append_accepts_kind_and_metadata() {
        let cli = Cli::try_parse_from([
            "kinic-cli",
            "--identity",
            "alice",
            "wiki",
            "append",
            "--database-id",
            "db",
            "--path",
            "/Sources/raw/a/a.md",
            "--input",
            "note.md",
            "--kind",
            "source",
            "--metadata-json",
            "{}",
        ])
        .expect("wiki append should parse");

        match cli.command {
            Command::Wiki(args) => match args.command {
                crate::cli::WikiCommand::Append(args) => {
                    assert_eq!(args.kind, Some(WikiNodeKindArg::Source));
                    assert_eq!(args.metadata_json.as_deref(), Some("{}"));
                }
                _ => panic!("expected wiki append"),
            },
            _ => panic!("expected wiki command"),
        }
    }

    #[test]
    fn append_request_keeps_kind_and_metadata() {
        let request = append_request(
            WikiAppendArgs {
                database_id: "db".to_string(),
                path: "/Sources/raw/a/a.md".to_string(),
                input: "unused.md".into(),
                expected_etag: Some("etag".to_string()),
                separator: Some("\n".to_string()),
                kind: Some(WikiNodeKindArg::Source),
                metadata_json: Some("{}".to_string()),
                json: false,
            },
            "body".to_string(),
            Some(NodeKind::Source),
        );

        assert_eq!(request.kind, Some(NodeKind::Source));
        assert_eq!(request.metadata_json.as_deref(), Some("{}"));
    }

    #[test]
    fn source_path_validation_matches_wiki_domain_rules() {
        assert!(validate_source_path("/Sources/raw/a/a.md", &NodeKind::Source).is_ok());
        assert!(validate_source_path("/Sources/sessions/a/a.md", &NodeKind::Source).is_ok());
        assert!(validate_source_path("/Sources/raw/a/b.md", &NodeKind::Source).is_err());
        assert!(validate_source_path("/Sources/raw/a.md", &NodeKind::Source).is_err());
        assert!(validate_source_path("/Sources/rawfoo/a/a.md", &NodeKind::Source).is_err());
        assert!(validate_source_path("/Sources/raw/a/a.md", &NodeKind::File).is_err());
        assert!(validate_source_path("/Wiki/a.md", &NodeKind::File).is_ok());
    }

    #[test]
    fn wiki_delete_parses_without_yes_for_runtime_rejection() {
        let cli = Cli::try_parse_from([
            "kinic-cli",
            "--identity",
            "alice",
            "wiki",
            "delete",
            "--database-id",
            "db",
            "--path",
            "/Wiki/a.md",
        ])
        .expect("delete command should parse so handler can reject missing --yes");

        match cli.command {
            Command::Wiki(args) => match args.command {
                crate::cli::WikiCommand::Delete(args) => assert!(!args.yes),
                _ => panic!("expected wiki delete"),
            },
            _ => panic!("expected wiki command"),
        }
    }
}
