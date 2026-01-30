use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use tokio::fs;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::ui::Ui;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppState {
    Welcome,
    SelectingAurPackages,
    SelectingOfficialPackages,
    SelectingIgnoredPackages,
    SelectingDotfiles,
    SelectingHomeDotfiles,
    Summary,
    Building,
    Complete,
    Error,
}

pub struct App {
    state: AppState,
    config: Config,
    dev_mode: bool,
    ui: Ui,
    should_quit: bool,
    error_message: Option<String>,
    build_started: bool,
    build_output: Vec<String>,
    output_rx: Option<mpsc::UnboundedReceiver<String>>,
    build_handle: Option<tokio::task::JoinHandle<(Config, Result<()>)>>,
}

impl App {
    pub fn new(dev_mode: bool) -> Result<Self> {
        Ok(Self {
            state: AppState::Welcome,
            config: Config::new(dev_mode),
            dev_mode,
            ui: Ui::new(),
            should_quit: false,
            error_message: None,
            build_started: false,
            build_output: Vec::new(),
            output_rx: None,
            build_handle: None,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_loop(&mut terminal).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        // Abort build if active
        if let Some(handle) = &mut self.build_handle {
            handle.abort();
            let _ = handle.await;
        }

        // Cleanup build directory if it exists (e.g. forced quit during build)
        if self.config.work_dir.exists() {
            fs::remove_dir_all(&self.config.work_dir).await.ok();
        }

        result
    }

    async fn run_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
        loop {
            // Receive any pending output messages
            if let Some(rx) = &mut self.output_rx {
                while let Ok(line) = rx.try_recv() {
                    self.build_output.push(line);
                }
            }

            terminal.draw(|f| {
                self.ui.render(
                    f,
                    &self.state,
                    &self.config,
                    &self.error_message,
                    &self.build_output,
                )
            })?;

            if self.should_quit {
                break;
            }

            // If we're in Building state and haven't started the build yet, start it now
            if self.state == AppState::Building && !self.build_started {
                self.build_started = true;

                // Create channel for output streaming
                let (tx, rx) = mpsc::unbounded_channel();
                self.output_rx = Some(rx);

                // Take the config and spawn the build in a separate task
                let mut config = std::mem::replace(&mut self.config, Config::new(self.dev_mode));
                let handle = tokio::spawn(async move {
                    let result = config.build_iso(tx).await;
                    (config, result)
                });
                self.build_handle = Some(handle);
            }

            // Check if build is complete
            if self.state == AppState::Building
                && self.build_started
                && let Some(handle) = &mut self.build_handle
                && handle.is_finished()
            {
                let (config, result) = handle.await.unwrap();
                self.config = config; // Restore the config
                self.build_handle = None;

                match result {
                    Ok(_) => self.state = AppState::Complete,
                    Err(e) => {
                        self.error_message = Some(e.to_string());
                        self.state = AppState::Error;
                    }
                }
            }

            if event::poll(std::time::Duration::from_millis(100))?
                && let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                self.handle_input(key.code).await?;
            }
        }

        Ok(())
    }

    async fn handle_input(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('Q') | KeyCode::Esc => {
                // Capital Q (Shift+Q) or Esc to quit
                self.should_quit = true;
            }
            KeyCode::Enter => {
                self.handle_enter().await?;
            }
            KeyCode::Up => {
                self.ui.handle_up(&self.state);
            }
            KeyCode::Down => {
                self.ui.handle_down(&self.state, &self.config);
            }
            KeyCode::Char(' ') => {
                self.ui.handle_space(&self.state, &mut self.config);
            }
            KeyCode::Char(c) => {
                self.ui.handle_char(c, &self.state);
            }
            KeyCode::Backspace => {
                self.ui.handle_backspace(&self.state);
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_enter(&mut self) -> Result<()> {
        match self.state {
            AppState::Welcome => {
                self.state = AppState::SelectingAurPackages;
                self.ui.reset_state();
                self.config.scan_aur_packages().await?;
            }
            AppState::SelectingAurPackages => {
                self.config.finalize_aur_selection(&self.ui);
                self.state = AppState::SelectingOfficialPackages;
                self.ui.reset_state();
                self.config.scan_official_packages().await?;
            }
            AppState::SelectingOfficialPackages => {
                self.config.finalize_official_selection(&self.ui);
                self.state = AppState::SelectingIgnoredPackages;
                self.ui.reset_state();
                self.config.scan_omarchy_packages().await?;
            }
            AppState::SelectingIgnoredPackages => {
                self.config.finalize_ignored_selection(&self.ui);
                self.state = AppState::SelectingDotfiles;
                self.ui.reset_state();
                self.config.scan_dotfiles().await?;
            }
            AppState::SelectingDotfiles => {
                self.config.finalize_dotfiles_selection(&self.ui);
                self.state = AppState::SelectingHomeDotfiles;
                self.ui.reset_state();
                self.config.scan_home_dotfiles().await?;
            }
            AppState::SelectingHomeDotfiles => {
                self.config.finalize_home_dotfiles_selection(&self.ui);
                self.state = AppState::Summary;
                self.ui.reset_state();
            }
            AppState::Summary => {
                // Transition to Building state
                // The actual build will start in the next loop iteration
                self.state = AppState::Building;
            }
            AppState::Complete | AppState::Error => {
                self.should_quit = true;
            }
            _ => {}
        }
        Ok(())
    }
}
