use _lib::cli::{Cli, Command, ConfigCommand, ConfigUsersCommand};
use clap::Parser;

#[test]
fn search_requires_exactly_one_target_selector() {
    let missing = Cli::try_parse_from(["kinic-cli", "search", "--query", "hello"]);
    assert!(missing.is_err());

    let conflicting = Cli::try_parse_from([
        "kinic-cli",
        "search",
        "--memory-id",
        "aaaaa-aa",
        "--all",
        "--query",
        "hello",
    ]);
    assert!(conflicting.is_err());
}

#[test]
fn search_accepts_all_scope() {
    let cli = Cli::try_parse_from(["kinic-cli", "search", "--all", "--query", "hello"])
        .expect("search --all should parse");

    match cli.command {
        Command::Search(args) => {
            assert!(args.all);
            assert_eq!(args.memory_id, None);
            assert_eq!(args.query, "hello");
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn config_users_subcommands_parse() {
    let cli = Cli::try_parse_from([
        "kinic-cli",
        "config",
        "users",
        "change",
        "--memory-id",
        "aaaaa-aa",
        "--principal",
        "anonymous",
        "--role",
        "reader",
    ])
    .expect("config users change should parse");

    match cli.command {
        Command::Config(args) => match args.command {
            ConfigCommand::Users(users) => match users.command {
                ConfigUsersCommand::Change(change) => {
                    assert_eq!(change.memory_id, "aaaaa-aa");
                    assert_eq!(change.principal, "anonymous");
                    assert_eq!(change.role, "reader");
                }
                other => panic!("unexpected users subcommand: {other:?}"),
            },
        },
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn transfer_requires_explicit_yes_flag_to_parse_as_true() {
    let cli = Cli::try_parse_from([
        "kinic-cli",
        "transfer",
        "--to",
        "aaaaa-aa",
        "--amount",
        "1.25",
        "--yes",
    ])
    .expect("transfer should parse");

    match cli.command {
        Command::Transfer(args) => {
            assert_eq!(args.to, "aaaaa-aa");
            assert_eq!(args.amount, "1.25");
            assert!(args.yes);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn read_commands_accept_json_flag() {
    let list = Cli::try_parse_from(["kinic-cli", "list", "--json"]).expect("list should parse");
    let show = Cli::try_parse_from(["kinic-cli", "show", "--memory-id", "aaaaa-aa", "--json"])
        .expect("show should parse");
    let search = Cli::try_parse_from([
        "kinic-cli",
        "search",
        "--memory-id",
        "aaaaa-aa",
        "--query",
        "hello",
        "--json",
    ])
    .expect("search should parse");

    match list.command {
        Command::List(args) => assert!(args.json),
        other => panic!("unexpected command: {other:?}"),
    }
    match show.command {
        Command::Show(args) => assert!(args.json),
        other => panic!("unexpected command: {other:?}"),
    }
    match search.command {
        Command::Search(args) => assert!(args.json),
        other => panic!("unexpected command: {other:?}"),
    }
}
