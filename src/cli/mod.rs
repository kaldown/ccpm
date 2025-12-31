use crate::plugin::{PluginDiscovery, PluginService, Scope, ScopeFilter};
use clap::{Parser, Subcommand, ValueEnum};
use color_eyre::Result;

#[derive(Parser)]
#[command(name = "ccpm")]
#[command(author = "CCPM Contributors")]
#[command(version)]
#[command(about = "Claude Code Plugin Manager - Manage your Claude Code plugins", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all plugins
    List {
        /// Filter by scope
        #[arg(short, long, value_enum, default_value = "all")]
        scope: ScopeArg,

        /// Show only enabled plugins
        #[arg(short, long)]
        enabled: bool,

        /// Show only disabled plugins
        #[arg(short, long)]
        disabled: bool,
    },

    /// Enable a plugin
    Enable {
        /// Plugin ID (name@marketplace)
        plugin: String,

        /// Scope to enable in
        #[arg(short, long, value_enum, default_value = "user")]
        scope: ScopeArg,
    },

    /// Disable a plugin
    Disable {
        /// Plugin ID (name@marketplace)
        plugin: String,

        /// Scope to disable in
        #[arg(short, long, value_enum, default_value = "user")]
        scope: ScopeArg,
    },

    /// Show plugin details
    Info {
        /// Plugin ID (name@marketplace)
        plugin: String,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum ScopeArg {
    All,
    User,
    Local,
}

impl From<ScopeArg> for ScopeFilter {
    fn from(arg: ScopeArg) -> Self {
        match arg {
            ScopeArg::All => ScopeFilter::All,
            ScopeArg::User => ScopeFilter::User,
            ScopeArg::Local => ScopeFilter::Local,
        }
    }
}

impl From<ScopeArg> for Scope {
    fn from(arg: ScopeArg) -> Self {
        match arg {
            ScopeArg::All | ScopeArg::User => Scope::User,
            ScopeArg::Local => Scope::Local,
        }
    }
}

pub fn run_command(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::List {
            scope,
            enabled,
            disabled,
        } => list_plugins(scope.into(), enabled, disabled),
        Commands::Enable { plugin, scope } => enable_plugin(&plugin, scope.into()),
        Commands::Disable { plugin, scope } => disable_plugin(&plugin, scope.into()),
        Commands::Info { plugin } => show_info(&plugin),
    }
}

fn list_plugins(scope_filter: ScopeFilter, only_enabled: bool, only_disabled: bool) -> Result<()> {
    let discovery = PluginDiscovery::new()?;
    let plugins = discovery.discover_all()?;

    let filtered: Vec<_> = plugins
        .iter()
        .filter(|p| match scope_filter {
            ScopeFilter::All => true,
            ScopeFilter::User => p.scope == Scope::User,
            ScopeFilter::Local => p.scope == Scope::Local,
        })
        .filter(|p| {
            if only_enabled {
                p.enabled
            } else if only_disabled {
                !p.enabled
            } else {
                true
            }
        })
        .collect();

    if filtered.is_empty() {
        println!("No plugins found.");
        return Ok(());
    }

    println!(
        "{:<30} {:<25} {:<8} {:<6}",
        "NAME", "MARKETPLACE", "STATUS", "SCOPE"
    );
    println!("{}", "-".repeat(75));

    for plugin in filtered {
        let status = if plugin.enabled {
            "enabled"
        } else {
            "disabled"
        };
        println!(
            "{:<30} {:<25} {:<8} {:<6}",
            plugin.name,
            plugin.marketplace,
            status,
            plugin.scope.to_string()
        );
    }

    Ok(())
}

fn enable_plugin(plugin_id: &str, scope: Scope) -> Result<()> {
    let service = PluginService::new()?;
    service.enable_plugin(plugin_id, scope)?;
    println!("Enabled {} in {} scope", plugin_id, scope);
    Ok(())
}

fn disable_plugin(plugin_id: &str, scope: Scope) -> Result<()> {
    let service = PluginService::new()?;
    service.disable_plugin(plugin_id, scope)?;
    println!("Disabled {} in {} scope", plugin_id, scope);
    Ok(())
}

fn show_info(plugin_id: &str) -> Result<()> {
    let discovery = PluginDiscovery::new()?;
    let plugins = discovery.discover_all()?;

    let plugin = plugins.iter().find(|p| p.id == plugin_id);

    match plugin {
        Some(p) => {
            println!("Name:        {}", p.name);
            println!("Marketplace: {}", p.marketplace);
            println!("ID:          {}", p.id);
            println!(
                "Status:      {}",
                if p.enabled { "enabled" } else { "disabled" }
            );
            println!("Scope:       {}", p.scope);

            if let Some(ref version) = p.version {
                println!("Version:     {}", version);
            }

            if let Some(ref author) = p.author {
                let author_str = if let Some(ref email) = author.email {
                    format!("{} <{}>", author.name, email)
                } else {
                    author.name.clone()
                };
                println!("Author:      {}", author_str);
            }

            if let Some(ref path) = p.install_path {
                println!("Path:        {}", path.display());
            }

            if let Some(ref desc) = p.description {
                println!("\nDescription:\n{}", desc);
            }
        }
        None => {
            println!("Plugin '{}' not found.", plugin_id);
        }
    }

    Ok(())
}
