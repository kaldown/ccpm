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

        /// Show debug information (Option values and file paths)
        #[arg(long)]
        debug: bool,
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
    Project,
    Local,
}

impl From<ScopeArg> for ScopeFilter {
    fn from(arg: ScopeArg) -> Self {
        match arg {
            ScopeArg::All => ScopeFilter::All,
            ScopeArg::User => ScopeFilter::User,
            ScopeArg::Project => ScopeFilter::Project,
            ScopeArg::Local => ScopeFilter::Local,
        }
    }
}

impl From<ScopeArg> for Scope {
    fn from(arg: ScopeArg) -> Self {
        match arg {
            ScopeArg::All | ScopeArg::User => Scope::User,
            ScopeArg::Project => Scope::Project,
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
            debug,
        } => list_plugins(scope.into(), enabled, disabled, debug),
        Commands::Enable { plugin, scope } => enable_plugin(&plugin, scope.into()),
        Commands::Disable { plugin, scope } => disable_plugin(&plugin, scope.into()),
        Commands::Info { plugin } => show_info(&plugin),
    }
}

fn list_plugins(scope_filter: ScopeFilter, only_enabled: bool, only_disabled: bool, debug: bool) -> Result<()> {
    let discovery = PluginDiscovery::new()?;
    let plugins = discovery.discover_all()?;

    // Debug output before filtering
    if debug {
        eprintln!("DEBUG: Loading {} plugins...", plugins.len());
        for plugin in &plugins {
            eprintln!(
                "DEBUG: {} -> user={:?} project={:?} local={:?} -> is_enabled={} project_path={:?}",
                plugin.id,
                plugin.enabled_user,
                plugin.enabled_project,
                plugin.enabled_local,
                plugin.is_enabled(),
                plugin.project_path
            );
        }
        eprintln!();
    }

    let filtered: Vec<_> = plugins
        .iter()
        .filter(|p| match scope_filter {
            ScopeFilter::All => true,
            ScopeFilter::User => p.install_scope == Scope::User,
            ScopeFilter::Project => p.install_scope == Scope::Project,
            ScopeFilter::Local => p.install_scope == Scope::Local,
        })
        .filter(|p| {
            if only_enabled {
                p.is_enabled()
            } else if only_disabled {
                !p.is_enabled()
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
        "{:<30} {:<25} {:<8} {:<10} {:<12}",
        "NAME", "MARKETPLACE", "STATUS", "INSTALLED", "ENABLED IN"
    );
    println!("{}", "-".repeat(90));

    for plugin in filtered {
        let status = if plugin.is_enabled() {
            "enabled"
        } else {
            "disabled"
        };
        let installed = match (plugin.install_scope, plugin.is_current_project) {
            (Scope::User, _) => "user",
            (Scope::Project, true) => "project",
            (Scope::Project, false) => "project*",
            (Scope::Local, true) => "local",
            (Scope::Local, false) => "local*",
        };
        println!(
            "{:<30} {:<25} {:<8} {:<10} {:<12}",
            plugin.name,
            plugin.marketplace,
            status,
            installed,
            plugin.enabled_context()
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
                if p.is_enabled() {
                    "enabled"
                } else {
                    "disabled"
                }
            );

            let installed = match (p.install_scope, p.is_current_project) {
                (Scope::User, _) => "User (~/.claude)".to_string(),
                (Scope::Project, true) => "Project (this project)".to_string(),
                (Scope::Project, false) => "Project (other project)".to_string(),
                (Scope::Local, true) => "Local (this project)".to_string(),
                (Scope::Local, false) => "Local (other project)".to_string(),
            };
            println!("Installed:   {}", installed);
            println!("Enabled in:  {}", p.enabled_context());

            // Show project path for project/local scope plugins
            if p.install_scope != Scope::User {
                if let Some(path_display) = p.project_path_display() {
                    println!("Project:     {}", path_display);
                }
            }

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
