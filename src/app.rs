use crate::plugin::{Plugin, PluginDiscovery, PluginService, Scope, ScopeFilter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Search,
    Help,
    Confirm(ConfirmAction),
    DetailModal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmAction {
    Remove,
}

#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub is_error: bool,
}

impl StatusMessage {
    pub fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: false,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_error: true,
        }
    }
}

pub struct App {
    pub plugins: Vec<Plugin>,
    pub filtered_plugins: Vec<usize>,
    pub selected_index: usize,
    pub scope_filter: ScopeFilter,
    pub search_query: String,
    pub mode: AppMode,
    pub message: Option<StatusMessage>,
    pub should_quit: bool,
    pub service: PluginService,
}

impl App {
    pub fn new() -> color_eyre::Result<Self> {
        let discovery = PluginDiscovery::new()?;
        let plugins = discovery.discover_all()?;
        let filtered_plugins: Vec<usize> = (0..plugins.len()).collect();

        Ok(Self {
            plugins,
            filtered_plugins,
            selected_index: 0,
            scope_filter: ScopeFilter::All,
            search_query: String::new(),
            mode: AppMode::Normal,
            message: None,
            should_quit: false,
            service: PluginService::new()?,
        })
    }

    pub fn reload_plugins(&mut self) -> color_eyre::Result<()> {
        let discovery = PluginDiscovery::new()?;
        self.plugins = discovery.discover_all()?;
        self.apply_filter();
        Ok(())
    }

    pub fn selected_plugin(&self) -> Option<&Plugin> {
        self.filtered_plugins
            .get(self.selected_index)
            .and_then(|&idx| self.plugins.get(idx))
    }

    pub fn selected_plugin_mut(&mut self) -> Option<&mut Plugin> {
        self.filtered_plugins
            .get(self.selected_index)
            .and_then(|&idx| self.plugins.get_mut(idx))
    }

    pub fn move_selection(&mut self, delta: i32) {
        if self.filtered_plugins.is_empty() {
            return;
        }

        let len = self.filtered_plugins.len() as i32;
        let new_index = (self.selected_index as i32 + delta).rem_euclid(len);
        self.selected_index = new_index as usize;
    }

    pub fn select_first(&mut self) {
        self.selected_index = 0;
    }

    pub fn select_last(&mut self) {
        if !self.filtered_plugins.is_empty() {
            self.selected_index = self.filtered_plugins.len() - 1;
        }
    }

    pub fn cycle_scope_filter(&mut self) {
        self.scope_filter = self.scope_filter.next();
        self.apply_filter();
    }

    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
        self.apply_filter();
    }

    pub fn append_search_char(&mut self, c: char) {
        self.search_query.push(c);
        self.apply_filter();
    }

    pub fn delete_search_char(&mut self) {
        self.search_query.pop();
        self.apply_filter();
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.apply_filter();
    }

    fn apply_filter(&mut self) {
        let query_lower = self.search_query.to_lowercase();

        self.filtered_plugins = self
            .plugins
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                // Scope filter (based on installation scope)
                let scope_match = match self.scope_filter {
                    ScopeFilter::All => true,
                    ScopeFilter::User => p.install_scope == Scope::User,
                    ScopeFilter::Local => p.install_scope == Scope::Local,
                };

                // Search filter
                let search_match = query_lower.is_empty()
                    || p.name.to_lowercase().contains(&query_lower)
                    || p.marketplace.to_lowercase().contains(&query_lower)
                    || p.description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false);

                scope_match && search_match
            })
            .map(|(i, _)| i)
            .collect();

        // Adjust selection if needed
        if self.selected_index >= self.filtered_plugins.len() {
            self.selected_index = self.filtered_plugins.len().saturating_sub(1);
        }
    }

    pub fn toggle_selected_plugin(&mut self) {
        if let Some(plugin) = self.selected_plugin() {
            let id = plugin.id.clone();
            let scope = plugin.install_scope;

            match self.service.toggle_plugin(plugin) {
                Ok(new_state) => {
                    if let Some(p) = self.plugins.iter_mut().find(|p| p.id == id) {
                        // Update the appropriate enabled field based on scope
                        match scope {
                            Scope::User => p.enabled_user = new_state,
                            Scope::Local => p.enabled_local = new_state,
                        }
                    }
                    self.message = Some(StatusMessage::info(format!(
                        "{} {} in {} scope",
                        id,
                        if new_state { "enabled" } else { "disabled" },
                        scope
                    )));
                }
                Err(e) => {
                    self.message = Some(StatusMessage::error(format!("Failed to toggle: {}", e)));
                }
            }
        }
    }

    pub fn enable_selected_plugin(&mut self) {
        if let Some(plugin) = self.selected_plugin() {
            if plugin.is_enabled() {
                self.message = Some(StatusMessage::info("Plugin already enabled"));
                return;
            }

            let id = plugin.id.clone();
            let scope = plugin.install_scope;

            match self.service.enable_plugin(&id, scope) {
                Ok(()) => {
                    if let Some(p) = self.plugins.iter_mut().find(|p| p.id == id) {
                        match scope {
                            Scope::User => p.enabled_user = true,
                            Scope::Local => p.enabled_local = true,
                        }
                    }
                    self.message = Some(StatusMessage::info(format!("Enabled {}", id)));
                }
                Err(e) => {
                    self.message = Some(StatusMessage::error(format!("Failed to enable: {}", e)));
                }
            }
        }
    }

    pub fn disable_selected_plugin(&mut self) {
        if let Some(plugin) = self.selected_plugin() {
            if !plugin.is_enabled() {
                self.message = Some(StatusMessage::info("Plugin already disabled"));
                return;
            }

            let id = plugin.id.clone();
            let scope = plugin.install_scope;

            match self.service.disable_plugin(&id, scope) {
                Ok(()) => {
                    if let Some(p) = self.plugins.iter_mut().find(|p| p.id == id) {
                        match scope {
                            Scope::User => p.enabled_user = false,
                            Scope::Local => p.enabled_local = false,
                        }
                    }
                    self.message = Some(StatusMessage::info(format!("Disabled {}", id)));
                }
                Err(e) => {
                    self.message = Some(StatusMessage::error(format!("Failed to disable: {}", e)));
                }
            }
        }
    }

    pub fn show_help(&mut self) {
        self.mode = AppMode::Help;
    }

    pub fn hide_help(&mut self) {
        self.mode = AppMode::Normal;
    }

    pub fn show_detail_modal(&mut self) {
        if self.selected_plugin().is_some() {
            self.mode = AppMode::DetailModal;
        }
    }

    pub fn hide_detail_modal(&mut self) {
        self.mode = AppMode::Normal;
    }

    pub fn start_search(&mut self) {
        self.mode = AppMode::Search;
    }

    pub fn end_search(&mut self) {
        self.mode = AppMode::Normal;
    }

    pub fn confirm_remove(&mut self) {
        if self.selected_plugin().is_some() {
            self.mode = AppMode::Confirm(ConfirmAction::Remove);
        }
    }

    pub fn cancel_confirm(&mut self) {
        self.mode = AppMode::Normal;
    }

    pub fn execute_confirm(&mut self) {
        if let AppMode::Confirm(ConfirmAction::Remove) = self.mode {
            // Remove functionality placeholder
            self.message = Some(StatusMessage::info("Remove not yet implemented"));
        }
        self.mode = AppMode::Normal;
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn plugin_count(&self) -> (usize, usize) {
        let enabled = self.plugins.iter().filter(|p| p.is_enabled()).count();
        (enabled, self.plugins.len())
    }
}
