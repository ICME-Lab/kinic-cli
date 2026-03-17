mod provider;

use provider::MailProvider;
use tui_kit_host::runtime_loop::{run_provider_app, RuntimeLoopConfig};
use tui_kit_render::ui::{BrandingText, HeaderText, TabId, TabSpec, UiConfig};

// Example architecture:
// - `examples/mail/provider.rs` provides provider behavior
// - `examples/mail/adapter.rs` provides domain-to-UI mappings
// - `tui-kit-runtime` provides interaction contracts and shared state
// - `tui-kit-render` renders state into a ratatui widget tree
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut provider = MailProvider::sample();
    run_provider_app(
        &mut provider,
        RuntimeLoopConfig {
            initial_tab_id: "mail-inbox",
            tab_ids: &["mail-inbox", "mail-invoices", "mail-alerts", "mail-news"],
            ui_config: mail_ui_config,
        },
    )
}

fn mail_ui_config() -> UiConfig {
    UiConfig {
        branding: BrandingText {
            logo_lines: vec![
                "███╗   ███╗ █████╗ ██╗██╗".to_string(),
                "████╗ ████║██╔══██╗██║██║".to_string(),
                "██╔████╔██║███████║██║██║".to_string(),
                "██║╚██╔╝██║██╔══██║██║██║".to_string(),
                "██║ ╚═╝ ██║██║  ██║██║███████╗".to_string(),
                "╚═╝     ╚═╝╚═╝  ╚═╝╚═╝╚══════╝".to_string(),
            ],
            attribution: "mail demo".to_string(),
        },
        header: HeaderText {
            visible_icon: "✉".to_string(),
            visible_suffix: "messages".to_string(),
            contexts_icon: "📂".to_string(),
            contexts_suffix: "mailboxes".to_string(),
            data_label: "cache".to_string(),
        },
        tabs: vec![
            TabSpec {
                id: TabId::new("mail-inbox"),
                title: "Inbox".to_string(),
                search_placeholder: "Search inbox messages...".to_string(),
            },
            TabSpec {
                id: TabId::new("mail-invoices"),
                title: "Invoices".to_string(),
                search_placeholder: "Search billing/invoice mail...".to_string(),
            },
            TabSpec {
                id: TabId::new("mail-alerts"),
                title: "Alerts".to_string(),
                search_placeholder: "Search security/system alerts...".to_string(),
            },
            TabSpec {
                id: TabId::new("mail-news"),
                title: "News".to_string(),
                search_placeholder: "Search welcome/newsletters...".to_string(),
            },
        ],
        ..UiConfig::default()
    }
}
