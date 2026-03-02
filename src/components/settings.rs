use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap};

use crate::action::Action;
use crate::config::hotkeys::HotkeyBinding;
use crate::db::config_repo::AccountRow;
use crate::widgets::dropdown::{Dropdown, DropdownAction};
use crate::widgets::editable_field::{EditFieldAction, EditableField};
use crate::widgets::modal;

use super::Component;

const LLM_PROVIDERS: &[&str] = &["claude", "openai", "ollama"];

const CLAUDE_MODELS: &[&str] = &["claude-sonnet-4-5-20250514", "claude-haiku-4-5-20251001", "custom..."];
const OPENAI_MODELS: &[&str] = &["gpt-4o", "gpt-4o-mini", "gpt-4.1", "custom..."];
const OLLAMA_MODELS: &[&str] = &["llama3.2", "mistral", "deepseek-r1", "custom..."];

fn models_for_provider(provider: &str) -> &'static [&'static str] {
    match provider {
        "claude" => CLAUDE_MODELS,
        "openai" => OPENAI_MODELS,
        "ollama" => OLLAMA_MODELS,
        _ => CLAUDE_MODELS,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
    General,
    Hotkeys,
    Theme,
    Accounts,
    Llm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccountsMode {
    List,
    Add,
    Edit,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccountFormField {
    Name,
    ApiKey,
    Provider,
    Model,
    OllamaUrl,
}

pub struct Settings {
    visible: bool,
    tab: SettingsTab,
    hotkeys: Vec<HotkeyBinding>,
    hotkey_state: ListState,
    themes: Vec<String>,
    theme_state: ListState,

    // -- Accounts tab --
    linear_accounts: Vec<AccountRow>,
    linear_list_state: ListState,
    accounts_mode: AccountsMode,
    account_name_field: EditableField,
    account_key_field: EditableField,
    account_editing_id: Option<String>,
    account_form_focus: AccountFormField,

    // -- LLM tab --
    llm_accounts: Vec<AccountRow>,
    llm_list_state: ListState,
    llm_mode: AccountsMode,
    llm_name_field: EditableField,
    llm_key_field: EditableField,
    llm_model_dropdown: Dropdown,
    llm_model_custom_field: EditableField,
    llm_model_is_custom: bool,
    llm_ollama_url_field: EditableField,
    llm_provider_idx: usize,
    llm_editing_id: Option<String>,
    llm_form_focus: AccountFormField,

    // Delete confirmation
    delete_target_id: Option<String>,
    delete_target_name: String,
}

fn mask_api_key(key: &str) -> String {
    if key.is_empty() {
        "(no key)".into()
    } else if key.len() > 6 {
        let prefix = &key[..3];
        let suffix = &key[key.len() - 3..];
        format!("{prefix}...{suffix}")
    } else {
        "(set)".into()
    }
}

impl Settings {
    pub fn new() -> Self {
        Self {
            visible: false,
            tab: SettingsTab::General,
            hotkeys: Vec::new(),
            hotkey_state: ListState::default(),
            themes: vec![
                "dark".into(),
                "light".into(),
                "solarized".into(),
                "gruvbox".into(),
            ],
            theme_state: {
                let mut s = ListState::default();
                s.select(Some(0));
                s
            },

            linear_accounts: Vec::new(),
            linear_list_state: ListState::default(),
            accounts_mode: AccountsMode::List,
            account_name_field: EditableField::new("Name", "", false),
            account_key_field: EditableField::new("API Key", "", false),
            account_editing_id: None,
            account_form_focus: AccountFormField::Name,

            llm_accounts: Vec::new(),
            llm_list_state: ListState::default(),
            llm_mode: AccountsMode::List,
            llm_name_field: EditableField::new("Name", "", false),
            llm_key_field: EditableField::new("API Key", "", false),
            llm_model_dropdown: Dropdown::new("Model", CLAUDE_MODELS.iter().map(|s| s.to_string()).collect(), 0),
            llm_model_custom_field: EditableField::new("Model", "", false),
            llm_model_is_custom: false,
            llm_ollama_url_field: EditableField::new("Ollama URL", "", false),
            llm_provider_idx: 0,
            llm_editing_id: None,
            llm_form_focus: AccountFormField::Name,

            delete_target_id: None,
            delete_target_name: String::new(),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn show(&mut self, hotkeys: Vec<HotkeyBinding>) {
        self.hotkeys = hotkeys;
        self.tab = SettingsTab::General;
        self.accounts_mode = AccountsMode::List;
        self.llm_mode = AccountsMode::List;
        self.visible = true;
        if !self.hotkeys.is_empty() {
            self.hotkey_state.select(Some(0));
        }
    }

    pub fn set_accounts(&mut self, accounts: Vec<AccountRow>) {
        self.linear_accounts = accounts.iter().filter(|a| a.provider == "linear").cloned().collect();
        self.llm_accounts = accounts.iter().filter(|a| a.provider != "linear").cloned().collect();

        // Reset list states
        if !self.linear_accounts.is_empty() {
            let sel = self.linear_list_state.selected().unwrap_or(0).min(self.linear_accounts.len().saturating_sub(1));
            self.linear_list_state.select(Some(sel));
        } else {
            self.linear_list_state.select(None);
        }
        if !self.llm_accounts.is_empty() {
            let sel = self.llm_list_state.selected().unwrap_or(0).min(self.llm_accounts.len().saturating_sub(1));
            self.llm_list_state.select(Some(sel));
        } else {
            self.llm_list_state.select(None);
        }
    }

    pub fn hide(&mut self) {
        self.visible = false;
        // Stop any active editing
        self.account_name_field.stop_editing();
        self.account_key_field.stop_editing();
        self.llm_name_field.stop_editing();
        self.llm_key_field.stop_editing();
        self.llm_model_custom_field.stop_editing();
        self.llm_ollama_url_field.stop_editing();
    }

    fn next_tab(&mut self) {
        self.tab = match self.tab {
            SettingsTab::General => SettingsTab::Hotkeys,
            SettingsTab::Hotkeys => SettingsTab::Theme,
            SettingsTab::Theme => SettingsTab::Accounts,
            SettingsTab::Accounts => SettingsTab::Llm,
            SettingsTab::Llm => SettingsTab::General,
        };
    }

    fn prev_tab(&mut self) {
        self.tab = match self.tab {
            SettingsTab::General => SettingsTab::Llm,
            SettingsTab::Hotkeys => SettingsTab::General,
            SettingsTab::Theme => SettingsTab::Hotkeys,
            SettingsTab::Accounts => SettingsTab::Theme,
            SettingsTab::Llm => SettingsTab::Accounts,
        };
    }

    fn tab_index(&self) -> usize {
        match self.tab {
            SettingsTab::General => 0,
            SettingsTab::Hotkeys => 1,
            SettingsTab::Theme => 2,
            SettingsTab::Accounts => 3,
            SettingsTab::Llm => 4,
        }
    }

    // --- Account form helpers ---

    fn enter_add_mode_linear(&mut self) {
        self.accounts_mode = AccountsMode::Add;
        self.account_editing_id = None;
        self.account_name_field.set_value("");
        self.account_key_field.set_value("");
        self.account_form_focus = AccountFormField::Name;
    }

    fn enter_edit_mode_linear(&mut self) {
        if let Some(idx) = self.linear_list_state.selected() {
            if let Some(acct) = self.linear_accounts.get(idx) {
                self.accounts_mode = AccountsMode::Edit;
                self.account_editing_id = Some(acct.id.clone());
                self.account_name_field.set_value(&acct.name);
                self.account_key_field.set_value(&acct.api_key);
                self.account_form_focus = AccountFormField::Name;
            }
        }
    }

    fn enter_confirm_delete_linear(&mut self) {
        if let Some(idx) = self.linear_list_state.selected() {
            if let Some(acct) = self.linear_accounts.get(idx) {
                self.accounts_mode = AccountsMode::Confirm;
                self.delete_target_id = Some(acct.id.clone());
                self.delete_target_name = acct.name.clone();
            }
        }
    }

    fn save_linear_account(&mut self) -> Option<Action> {
        let name = self.account_name_field.value().to_string();
        let api_key = self.account_key_field.value().to_string();
        if name.is_empty() {
            return Some(Action::StatusMessage("Name is required".into()));
        }
        self.account_name_field.stop_editing();
        self.account_key_field.stop_editing();
        self.accounts_mode = AccountsMode::List;
        Some(Action::SaveAccount {
            id: self.account_editing_id.take(),
            name,
            provider: "linear".into(),
            api_key,
            model: None,
            ollama_url: None,
        })
    }

    fn enter_add_mode_llm(&mut self) {
        self.llm_mode = AccountsMode::Add;
        self.llm_editing_id = None;
        self.llm_name_field.set_value("");
        self.llm_key_field.set_value("");
        self.llm_ollama_url_field.set_value("");
        self.llm_provider_idx = 0;
        self.llm_form_focus = AccountFormField::Name;
        // Reset model dropdown for default provider
        let models = models_for_provider(self.current_llm_provider());
        self.llm_model_dropdown.set_items(models.iter().map(|s| s.to_string()).collect(), 0);
        self.llm_model_is_custom = false;
        self.llm_model_custom_field.set_value("");
    }

    fn enter_edit_mode_llm(&mut self) {
        if let Some(idx) = self.llm_list_state.selected() {
            if let Some(acct) = self.llm_accounts.get(idx).cloned() {
                self.llm_mode = AccountsMode::Edit;
                self.llm_editing_id = Some(acct.id.clone());
                self.llm_name_field.set_value(&acct.name);
                self.llm_key_field.set_value(&acct.api_key);
                self.llm_provider_idx = LLM_PROVIDERS.iter().position(|p| *p == acct.provider).unwrap_or(0);
                self.llm_ollama_url_field.set_value(acct.ollama_url.as_deref().unwrap_or(""));
                self.llm_form_focus = AccountFormField::Name;

                // Populate model dropdown for this provider
                let models = models_for_provider(&acct.provider);
                self.llm_model_dropdown.set_items(models.iter().map(|s| s.to_string()).collect(), 0);

                // Try to find stored model in the known list
                if let Some(stored_model) = &acct.model {
                    if let Some(pos) = models.iter().position(|m| *m == stored_model.as_str()) {
                        self.llm_model_dropdown.set_selected(pos);
                        self.llm_model_is_custom = false;
                        self.llm_model_custom_field.set_value("");
                    } else {
                        // Not in known list — select "custom..." and populate custom field
                        let custom_idx = models.len().saturating_sub(1);
                        self.llm_model_dropdown.set_selected(custom_idx);
                        self.llm_model_is_custom = true;
                        self.llm_model_custom_field.set_value(stored_model);
                    }
                } else {
                    self.llm_model_is_custom = false;
                    self.llm_model_custom_field.set_value("");
                }
            }
        }
    }

    fn enter_confirm_delete_llm(&mut self) {
        if let Some(idx) = self.llm_list_state.selected() {
            if let Some(acct) = self.llm_accounts.get(idx) {
                self.llm_mode = AccountsMode::Confirm;
                self.delete_target_id = Some(acct.id.clone());
                self.delete_target_name = acct.name.clone();
            }
        }
    }

    fn save_llm_account(&mut self) -> Option<Action> {
        let name = self.llm_name_field.value().to_string();
        let api_key = self.llm_key_field.value().to_string();
        let provider = LLM_PROVIDERS[self.llm_provider_idx].to_string();
        let model = if self.llm_model_is_custom {
            let v = self.llm_model_custom_field.value().to_string();
            if v.is_empty() { None } else { Some(v) }
        } else {
            self.llm_model_dropdown.selected_value()
                .filter(|v| *v != "custom...")
                .map(|v| v.to_string())
        };
        let ollama_url = if provider == "ollama" {
            let v = self.llm_ollama_url_field.value().to_string();
            if v.is_empty() { None } else { Some(v) }
        } else {
            None
        };
        if name.is_empty() {
            return Some(Action::StatusMessage("Name is required".into()));
        }
        self.llm_name_field.stop_editing();
        self.llm_key_field.stop_editing();
        self.llm_model_custom_field.stop_editing();
        self.llm_ollama_url_field.stop_editing();
        self.llm_mode = AccountsMode::List;
        Some(Action::SaveAccount {
            id: self.llm_editing_id.take(),
            name,
            provider,
            api_key,
            model,
            ollama_url,
        })
    }

    fn current_llm_provider(&self) -> &str {
        LLM_PROVIDERS[self.llm_provider_idx]
    }

    fn llm_form_fields(&self) -> Vec<AccountFormField> {
        let mut fields = vec![AccountFormField::Name, AccountFormField::Provider, AccountFormField::ApiKey, AccountFormField::Model];
        if self.current_llm_provider() == "ollama" {
            fields.push(AccountFormField::OllamaUrl);
        }
        fields
    }

    fn next_linear_field(&mut self) {
        self.account_form_focus = match self.account_form_focus {
            AccountFormField::Name => AccountFormField::ApiKey,
            _ => AccountFormField::Name,
        };
    }

    fn prev_linear_field(&mut self) {
        self.next_linear_field(); // only 2 fields, same as next
    }

    fn next_llm_field(&mut self) {
        let fields = self.llm_form_fields();
        if let Some(pos) = fields.iter().position(|f| *f == self.llm_form_focus) {
            self.llm_form_focus = fields[(pos + 1) % fields.len()];
        }
    }

    fn prev_llm_field(&mut self) {
        let fields = self.llm_form_fields();
        if let Some(pos) = fields.iter().position(|f| *f == self.llm_form_focus) {
            self.llm_form_focus = fields[(pos + fields.len() - 1) % fields.len()];
        }
    }

    fn any_linear_field_editing(&self) -> bool {
        self.account_name_field.is_editing() || self.account_key_field.is_editing()
    }

    fn any_llm_field_editing(&self) -> bool {
        self.llm_name_field.is_editing()
            || self.llm_key_field.is_editing()
            || self.llm_model_dropdown.is_open()
            || self.llm_model_custom_field.is_editing()
            || self.llm_ollama_url_field.is_editing()
    }

    fn stop_all_linear_editing(&mut self) {
        self.account_name_field.stop_editing();
        self.account_key_field.stop_editing();
    }

    fn stop_all_llm_editing(&mut self) {
        self.llm_name_field.stop_editing();
        self.llm_key_field.stop_editing();
        self.llm_model_custom_field.stop_editing();
        self.llm_ollama_url_field.stop_editing();
    }

    // --- Key handling for Accounts tab ---

    fn handle_accounts_key(&mut self, key: KeyEvent) -> Option<Action> {
        match self.accounts_mode {
            AccountsMode::List => self.handle_accounts_list_key(key),
            AccountsMode::Add | AccountsMode::Edit => self.handle_accounts_form_key(key),
            AccountsMode::Confirm => self.handle_accounts_confirm_key(key),
        }
    }

    fn handle_accounts_list_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                None
            }
            KeyCode::Tab => { self.next_tab(); None }
            KeyCode::BackTab => { self.prev_tab(); None }
            KeyCode::Char('j') | KeyCode::Down => {
                let len = self.linear_accounts.len();
                if len > 0 {
                    let i = self.linear_list_state.selected().unwrap_or(0);
                    if i < len.saturating_sub(1) {
                        self.linear_list_state.select(Some(i + 1));
                    }
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let i = self.linear_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.linear_list_state.select(Some(i - 1));
                }
                None
            }
            KeyCode::Enter => {
                if let Some(idx) = self.linear_list_state.selected() {
                    if let Some(acct) = self.linear_accounts.get(idx) {
                        return Some(Action::SwitchAccount(acct.id.clone()));
                    }
                }
                None
            }
            KeyCode::Char('a') => { self.enter_add_mode_linear(); None }
            KeyCode::Char('e') => { self.enter_edit_mode_linear(); None }
            KeyCode::Char('d') => { self.enter_confirm_delete_linear(); None }
            _ => None,
        }
    }

    fn handle_accounts_form_key(&mut self, key: KeyEvent) -> Option<Action> {
        // If a field is actively editing, delegate to it
        if self.any_linear_field_editing() {
            let result = match self.account_form_focus {
                AccountFormField::Name => self.account_name_field.handle_key(key),
                AccountFormField::ApiKey => self.account_key_field.handle_key(key),
                _ => EditFieldAction::None,
            };
            return match result {
                EditFieldAction::Submit | EditFieldAction::Cancel => None,
                EditFieldAction::None => None,
            };
        }

        // Form navigation
        match key.code {
            KeyCode::Esc => {
                self.stop_all_linear_editing();
                self.accounts_mode = AccountsMode::List;
                None
            }
            KeyCode::Tab | KeyCode::Char('j') | KeyCode::Down => {
                self.next_linear_field();
                None
            }
            KeyCode::BackTab | KeyCode::Char('k') | KeyCode::Up => {
                self.prev_linear_field();
                None
            }
            KeyCode::Enter => {
                // Start editing focused field
                match self.account_form_focus {
                    AccountFormField::Name => self.account_name_field.start_editing(),
                    AccountFormField::ApiKey => self.account_key_field.start_editing(),
                    _ => {}
                }
                None
            }
            KeyCode::Char('s') => self.save_linear_account(),
            _ => None,
        }
    }

    fn handle_accounts_confirm_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                let id = self.delete_target_id.take().unwrap_or_default();
                self.accounts_mode = AccountsMode::List;
                if !id.is_empty() {
                    Some(Action::DeleteAccount(id))
                } else {
                    None
                }
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.accounts_mode = AccountsMode::List;
                self.delete_target_id = None;
                None
            }
            _ => None,
        }
    }

    // --- Key handling for LLM tab ---

    fn handle_llm_key(&mut self, key: KeyEvent) -> Option<Action> {
        match self.llm_mode {
            AccountsMode::List => self.handle_llm_list_key(key),
            AccountsMode::Add | AccountsMode::Edit => self.handle_llm_form_key(key),
            AccountsMode::Confirm => self.handle_llm_confirm_key(key),
        }
    }

    fn handle_llm_list_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                None
            }
            KeyCode::Tab => { self.next_tab(); None }
            KeyCode::BackTab => { self.prev_tab(); None }
            KeyCode::Char('j') | KeyCode::Down => {
                let len = self.llm_accounts.len();
                if len > 0 {
                    let i = self.llm_list_state.selected().unwrap_or(0);
                    if i < len.saturating_sub(1) {
                        self.llm_list_state.select(Some(i + 1));
                    }
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let i = self.llm_list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.llm_list_state.select(Some(i - 1));
                }
                None
            }
            KeyCode::Enter => {
                if let Some(idx) = self.llm_list_state.selected() {
                    if let Some(acct) = self.llm_accounts.get(idx) {
                        return Some(Action::SwitchAccount(acct.id.clone()));
                    }
                }
                None
            }
            KeyCode::Char('a') => { self.enter_add_mode_llm(); None }
            KeyCode::Char('e') => { self.enter_edit_mode_llm(); None }
            KeyCode::Char('d') => { self.enter_confirm_delete_llm(); None }
            _ => None,
        }
    }

    fn handle_llm_form_key(&mut self, key: KeyEvent) -> Option<Action> {
        // If the model dropdown is open, delegate to it
        if self.llm_model_dropdown.is_open() {
            match self.llm_model_dropdown.handle_key(key) {
                DropdownAction::Selected(_idx) => {
                    if self.llm_model_dropdown.selected_value() == Some("custom...") {
                        self.llm_model_is_custom = true;
                        self.llm_model_custom_field.set_value("");
                        self.llm_model_custom_field.start_editing();
                    } else {
                        self.llm_model_is_custom = false;
                    }
                }
                DropdownAction::Cancel => {}
                DropdownAction::None => {}
            }
            return None;
        }

        // If the custom model field is editing, delegate to it
        if self.llm_model_is_custom && self.llm_model_custom_field.is_editing() {
            let result = self.llm_model_custom_field.handle_key(key);
            return match result {
                EditFieldAction::Submit | EditFieldAction::Cancel => None,
                EditFieldAction::None => None,
            };
        }

        // If another field is actively editing, delegate to it
        if self.any_llm_field_editing() {
            let result = match self.llm_form_focus {
                AccountFormField::Name => self.llm_name_field.handle_key(key),
                AccountFormField::ApiKey => self.llm_key_field.handle_key(key),
                AccountFormField::OllamaUrl => self.llm_ollama_url_field.handle_key(key),
                _ => EditFieldAction::None,
            };
            return match result {
                EditFieldAction::Submit | EditFieldAction::Cancel => None,
                EditFieldAction::None => None,
            };
        }

        // Form navigation
        match key.code {
            KeyCode::Esc => {
                self.stop_all_llm_editing();
                self.llm_mode = AccountsMode::List;
                None
            }
            KeyCode::Tab | KeyCode::Char('j') | KeyCode::Down => {
                self.next_llm_field();
                None
            }
            KeyCode::BackTab | KeyCode::Char('k') | KeyCode::Up => {
                self.prev_llm_field();
                None
            }
            KeyCode::Enter => {
                match self.llm_form_focus {
                    AccountFormField::Provider => {
                        self.llm_provider_idx = (self.llm_provider_idx + 1) % LLM_PROVIDERS.len();
                        // Rebuild model dropdown for new provider
                        let models = models_for_provider(self.current_llm_provider());
                        self.llm_model_dropdown.set_items(models.iter().map(|s| s.to_string()).collect(), 0);
                        self.llm_model_is_custom = false;
                        self.llm_model_custom_field.set_value("");
                        // If we moved away from ollama and focus was on OllamaUrl, move it
                        if self.current_llm_provider() != "ollama" && self.llm_form_focus == AccountFormField::OllamaUrl {
                            self.llm_form_focus = AccountFormField::Model;
                        }
                    }
                    AccountFormField::Name => self.llm_name_field.start_editing(),
                    AccountFormField::ApiKey => self.llm_key_field.start_editing(),
                    AccountFormField::Model => {
                        if self.llm_model_is_custom {
                            self.llm_model_custom_field.start_editing();
                        } else {
                            self.llm_model_dropdown.toggle();
                        }
                    }
                    AccountFormField::OllamaUrl => self.llm_ollama_url_field.start_editing(),
                }
                None
            }
            KeyCode::Char('s') => self.save_llm_account(),
            _ => None,
        }
    }

    fn handle_llm_confirm_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                let id = self.delete_target_id.take().unwrap_or_default();
                self.llm_mode = AccountsMode::List;
                if !id.is_empty() {
                    Some(Action::DeleteAccount(id))
                } else {
                    None
                }
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.llm_mode = AccountsMode::List;
                self.delete_target_id = None;
                None
            }
            _ => None,
        }
    }

    // --- Rendering ---

    fn render_accounts_tab(&self, frame: &mut Frame, area: Rect) {
        match self.accounts_mode {
            AccountsMode::List => self.render_accounts_list(frame, area),
            AccountsMode::Add | AccountsMode::Edit => self.render_accounts_form(frame, area),
            AccountsMode::Confirm => {
                self.render_accounts_list(frame, area);
                self.render_confirm_dialog(frame, area);
            }
        }
    }

    fn render_accounts_list(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(2), // header
            Constraint::Min(1),   // list
            Constraint::Length(1), // help
        ])
        .split(area);

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "  Linear Accounts",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ))),
            chunks[0],
        );

        if self.linear_accounts.is_empty() {
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::from("  No accounts configured."),
                    Line::from("  Press 'a' to add one."),
                ]),
                chunks[1],
            );
        } else {
            let items: Vec<ListItem> = self
                .linear_accounts
                .iter()
                .map(|a| {
                    let active = if a.is_active { " * " } else { "   " };
                    ListItem::new(Line::from(vec![
                        Span::styled(active, Style::default().fg(Color::Green)),
                        Span::styled(
                            format!("{:<20}", a.name),
                            Style::default().fg(Color::White),
                        ),
                        Span::styled(
                            mask_api_key(&a.api_key),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]))
                })
                .collect();

            let list = List::new(items)
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            frame.render_stateful_widget(list, chunks[1], &mut self.linear_list_state.clone());
        }

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " a: add | e: edit | d: delete | Enter: switch | Tab: next section",
                Style::default().fg(Color::DarkGray),
            ))),
            chunks[2],
        );
    }

    fn render_accounts_form(&self, frame: &mut Frame, area: Rect) {
        let title = if self.accounts_mode == AccountsMode::Add {
            "Add Linear Account"
        } else {
            "Edit Linear Account"
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let form_width = 50u16.min(area.width.saturating_sub(4));
        let form_height = 8u16.min(area.height.saturating_sub(4));
        let x = area.x + (area.width.saturating_sub(form_width)) / 2;
        let y = area.y + (area.height.saturating_sub(form_height)) / 2;
        let form_area = Rect::new(x, y, form_width, form_height);

        frame.render_widget(Clear, form_area);
        frame.render_widget(block, form_area);

        let inner = Rect::new(
            form_area.x + 2,
            form_area.y + 1,
            form_area.width.saturating_sub(4),
            form_area.height.saturating_sub(2),
        );

        let field_chunks = Layout::vertical([
            Constraint::Length(1), // Name
            Constraint::Length(1), // API Key
            Constraint::Length(1), // spacer
            Constraint::Length(1), // help
        ])
        .split(inner);

        self.render_form_field(frame, field_chunks[0], "Name", self.account_name_field.value(),
            self.account_form_focus == AccountFormField::Name, self.account_name_field.is_editing());
        self.render_form_field(frame, field_chunks[1], "API Key", self.account_key_field.value(),
            self.account_form_focus == AccountFormField::ApiKey, self.account_key_field.is_editing());

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "j/k: navigate | Enter: edit | s: save | Esc: cancel",
                Style::default().fg(Color::DarkGray),
            ))),
            field_chunks[3],
        );
    }

    fn render_llm_tab(&self, frame: &mut Frame, area: Rect) {
        match self.llm_mode {
            AccountsMode::List => self.render_llm_list(frame, area),
            AccountsMode::Add | AccountsMode::Edit => self.render_llm_form(frame, area),
            AccountsMode::Confirm => {
                self.render_llm_list(frame, area);
                self.render_confirm_dialog(frame, area);
            }
        }
    }

    fn render_llm_list(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Length(3), // header
            Constraint::Min(1),   // list
            Constraint::Length(1), // help
        ])
        .split(area);

        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    "  LLM Configuration",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    "  Used for transcript summarization.",
                    Style::default().fg(Color::DarkGray),
                )),
            ]),
            chunks[0],
        );

        if self.llm_accounts.is_empty() {
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(""),
                    Line::from("  No LLM configs."),
                    Line::from("  Press 'a' to add one."),
                ]),
                chunks[1],
            );
        } else {
            let items: Vec<ListItem> = self
                .llm_accounts
                .iter()
                .map(|a| {
                    let active = if a.is_active { " * " } else { "   " };
                    let key_display = if a.api_key.is_empty() && a.provider == "ollama" {
                        "(no key)".to_string()
                    } else {
                        mask_api_key(&a.api_key)
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(active, Style::default().fg(Color::Green)),
                        Span::styled(
                            format!("{:<8}", a.provider),
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::styled(
                            format!("{:<16}", a.name),
                            Style::default().fg(Color::White),
                        ),
                        Span::styled(key_display, Style::default().fg(Color::DarkGray)),
                    ]))
                })
                .collect();

            let list = List::new(items)
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            frame.render_stateful_widget(list, chunks[1], &mut self.llm_list_state.clone());
        }

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " a: add | e: edit | d: delete | Enter: switch | Tab: next section",
                Style::default().fg(Color::DarkGray),
            ))),
            chunks[2],
        );
    }

    fn render_llm_form(&self, frame: &mut Frame, area: Rect) {
        let title = if self.llm_mode == AccountsMode::Add {
            "Add LLM Config"
        } else {
            "Edit LLM Config"
        };

        let is_ollama = self.current_llm_provider() == "ollama";
        let custom_row = if self.llm_model_is_custom { 1u16 } else { 0 };
        let ollama_row = if is_ollama { 1u16 } else { 0 };
        let form_height = 10u16 + custom_row + ollama_row;

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let form_width = 55u16.min(area.width.saturating_sub(4));
        let fh = form_height.min(area.height.saturating_sub(4));
        let x = area.x + (area.width.saturating_sub(form_width)) / 2;
        let y = area.y + (area.height.saturating_sub(fh)) / 2;
        let form_area = Rect::new(x, y, form_width, fh);

        frame.render_widget(Clear, form_area);
        frame.render_widget(block, form_area);

        let inner = Rect::new(
            form_area.x + 2,
            form_area.y + 1,
            form_area.width.saturating_sub(4),
            form_area.height.saturating_sub(2),
        );

        let mut constraints = vec![
            Constraint::Length(1), // Name
            Constraint::Length(1), // Provider
            Constraint::Length(1), // API Key
            Constraint::Length(1), // Model dropdown
        ];
        if self.llm_model_is_custom {
            constraints.push(Constraint::Length(1)); // Custom model text field
        }
        if is_ollama {
            constraints.push(Constraint::Length(1)); // Ollama URL
        }
        constraints.push(Constraint::Length(1)); // spacer
        constraints.push(Constraint::Length(1)); // help

        let field_chunks = Layout::vertical(constraints).split(inner);

        self.render_form_field(frame, field_chunks[0], "Name", self.llm_name_field.value(),
            self.llm_form_focus == AccountFormField::Name, self.llm_name_field.is_editing());

        // Provider field (cycle on Enter)
        let provider_focused = self.llm_form_focus == AccountFormField::Provider;
        let provider_style = if provider_focused {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let arrow = if provider_focused { "▶ " } else { "  " };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(arrow),
                Span::styled("Provider: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(self.current_llm_provider(), provider_style),
                Span::styled(" (Enter to cycle)", Style::default().fg(Color::DarkGray)),
            ])),
            field_chunks[1],
        );

        self.render_form_field(frame, field_chunks[2], "API Key", self.llm_key_field.value(),
            self.llm_form_focus == AccountFormField::ApiKey, self.llm_key_field.is_editing());

        // Model dropdown row
        let model_focused = self.llm_form_focus == AccountFormField::Model;
        let model_arrow = if model_focused { "▶ " } else { "  " };
        let model_row = Rect::new(
            field_chunks[3].x + model_arrow.len() as u16,
            field_chunks[3].y,
            field_chunks[3].width.saturating_sub(model_arrow.len() as u16),
            1,
        );
        frame.render_widget(Paragraph::new(Span::raw(model_arrow)), field_chunks[3]);
        self.llm_model_dropdown.render_inline(frame, model_row);

        let mut next_field_idx = 4;

        // Custom model field (if in custom mode)
        if self.llm_model_is_custom {
            let custom_area = Rect::new(
                field_chunks[next_field_idx].x + 4, // indent
                field_chunks[next_field_idx].y,
                field_chunks[next_field_idx].width.saturating_sub(4),
                1,
            );
            self.llm_model_custom_field.render(frame, custom_area);
            next_field_idx += 1;
        }

        if is_ollama {
            self.render_form_field(frame, field_chunks[next_field_idx], "Ollama URL", self.llm_ollama_url_field.value(),
                self.llm_form_focus == AccountFormField::OllamaUrl, self.llm_ollama_url_field.is_editing());
        }

        let help_idx = field_chunks.len() - 1;
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "j/k: navigate | Enter: edit/cycle | s: save | Esc: cancel",
                Style::default().fg(Color::DarkGray),
            ))),
            field_chunks[help_idx],
        );

        // Render dropdown popup overlay (must be last so it draws on top)
        if self.llm_model_dropdown.is_open() {
            let popup_area = Rect::new(
                model_row.x,
                model_row.y + 1,
                model_row.width,
                area.height.saturating_sub(model_row.y + 1),
            );
            self.llm_model_dropdown.render_popup(frame, popup_area);
        }
    }

    fn render_form_field(&self, frame: &mut Frame, area: Rect, label: &str, value: &str, focused: bool, editing: bool) {
        let arrow = if focused { "▶ " } else { "  " };
        let val_style = if editing {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };
        let display_val = if value.is_empty() && !editing {
            "(empty)".to_string()
        } else {
            value.to_string()
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(arrow),
                Span::styled(format!("{label}: "), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(display_val, val_style),
            ])),
            area,
        );
    }

    fn render_confirm_dialog(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Confirm Delete")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red))
            .style(Style::default().bg(Color::Black));

        let w = 40u16.min(area.width.saturating_sub(4));
        let h = 6u16.min(area.height.saturating_sub(4));
        let x = area.x + (area.width.saturating_sub(w)) / 2;
        let y = area.y + (area.height.saturating_sub(h)) / 2;
        let dialog_area = Rect::new(x, y, w, h);

        frame.render_widget(Clear, dialog_area);
        frame.render_widget(block, dialog_area);

        let inner = Rect::new(dialog_area.x + 2, dialog_area.y + 1, dialog_area.width.saturating_sub(4), dialog_area.height.saturating_sub(2));
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(format!(" Delete \"{}\"?", self.delete_target_name)),
                Line::from(Span::styled(" This cannot be undone.", Style::default().fg(Color::DarkGray))),
                Line::from(""),
                Line::from(Span::styled(" y: confirm | Esc: cancel", Style::default().fg(Color::DarkGray))),
            ]).wrap(Wrap { trim: false }),
            inner,
        );
    }
}

impl Component for Settings {
    fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.visible {
            return None;
        }

        match self.tab {
            SettingsTab::Accounts => return self.handle_accounts_key(key),
            SettingsTab::Llm => return self.handle_llm_key(key),
            _ => {}
        }

        // General/Hotkeys/Theme tabs (original behavior)
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                None
            }
            KeyCode::Tab => {
                self.next_tab();
                None
            }
            KeyCode::BackTab => {
                self.prev_tab();
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                match self.tab {
                    SettingsTab::Hotkeys => {
                        let i = self.hotkey_state.selected().unwrap_or(0);
                        if i < self.hotkeys.len().saturating_sub(1) {
                            self.hotkey_state.select(Some(i + 1));
                        }
                    }
                    SettingsTab::Theme => {
                        let i = self.theme_state.selected().unwrap_or(0);
                        if i < self.themes.len().saturating_sub(1) {
                            self.theme_state.select(Some(i + 1));
                        }
                    }
                    _ => {}
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                match self.tab {
                    SettingsTab::Hotkeys => {
                        let i = self.hotkey_state.selected().unwrap_or(0);
                        if i > 0 {
                            self.hotkey_state.select(Some(i - 1));
                        }
                    }
                    SettingsTab::Theme => {
                        let i = self.theme_state.selected().unwrap_or(0);
                        if i > 0 {
                            self.theme_state.select(Some(i - 1));
                        }
                    }
                    _ => {}
                }
                None
            }
            _ => None,
        }
    }

    fn update(&mut self, action: &Action) -> Option<Action> {
        if let Action::AccountsLoaded(accounts) = action {
            self.set_accounts(accounts.clone());
        }
        None
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let inner = modal::render_modal(frame, area, "Settings", 70, 70);

        let chunks = Layout::vertical([
            Constraint::Length(1), // Tabs
            Constraint::Length(1), // Spacer
            Constraint::Min(1),   // Content
            Constraint::Length(1), // Help
        ])
        .split(inner);

        // Tab bar
        let tabs = Tabs::new(vec!["General", "Hotkeys", "Theme", "Accounts", "LLM"])
            .select(self.tab_index())
            .style(Style::default().fg(Color::DarkGray))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_widget(tabs, chunks[0]);

        // Content based on tab
        match self.tab {
            SettingsTab::General => {
                frame.render_widget(
                    Paragraph::new(vec![
                        Line::from(Span::styled(
                            "  General Settings",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )),
                        Line::from(""),
                        Line::from("  Sync interval: 120s"),
                        Line::from("  Transcript capture: 3s"),
                        Line::from("  Summary interval: 10min"),
                    ]),
                    chunks[2],
                );
            }
            SettingsTab::Hotkeys => {
                let items: Vec<ListItem> = self
                    .hotkeys
                    .iter()
                    .map(|h| {
                        ListItem::new(Line::from(vec![
                            Span::styled(
                                format!("{:<20}", h.action),
                                Style::default().fg(Color::Cyan),
                            ),
                            Span::styled(
                                format!("{:<10}", h.key_binding),
                                Style::default().fg(Color::Yellow),
                            ),
                            Span::styled(
                                h.description.as_deref().unwrap_or(""),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]))
                    })
                    .collect();

                let list = List::new(items)
                    .highlight_style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("▶ ");

                frame.render_stateful_widget(
                    list,
                    chunks[2],
                    &mut self.hotkey_state.clone(),
                );
            }
            SettingsTab::Theme => {
                let items: Vec<ListItem> = self
                    .themes
                    .iter()
                    .map(|t| ListItem::new(format!("  {t}")))
                    .collect();

                let list = List::new(items)
                    .highlight_style(
                        Style::default()
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("▶ ");

                frame.render_stateful_widget(
                    list,
                    chunks[2],
                    &mut self.theme_state.clone(),
                );
            }
            SettingsTab::Accounts => {
                self.render_accounts_tab(frame, chunks[2]);
            }
            SettingsTab::Llm => {
                self.render_llm_tab(frame, chunks[2]);
            }
        }

        // Help (only for non-interactive tabs; interactive tabs have their own)
        match self.tab {
            SettingsTab::General | SettingsTab::Hotkeys | SettingsTab::Theme => {
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        " Tab/S-Tab: sections | j/k: navigate | Esc: close",
                        Style::default().fg(Color::DarkGray),
                    ))),
                    chunks[3],
                );
            }
            _ => {} // Accounts/LLM tabs render their own help
        }
    }
}
