use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::app::AppState;
use crate::config::Config;

pub struct Ui {
    pub selected_index: usize,
    pub filter_text: String,
    pub error_scroll_offset: usize,
}

const LOGO: &str = r#"
                 ▄▄▄                                                             
 ▄█████▄    ▄███████████▄    ▄███████   ▄███████   ▄███████   ▄█   █▄    ▄█   █▄ 
███   ███  ███   ███   ███  ███   ███  ███   ███  ███   ███  ███   ███  ███   ███
███   ███  ███   ███   ███  ███   ███  ███   ███  ███   █▀   ███   ███  ███   ███
███   ███  ███   ███   ███ ▄███▄▄▄███ ▄███▄▄▄██▀  ███       ▄███▄▄▄███▄ ███▄▄▄███
███   ███  ███   ███   ███ ▀███▀▀▀███ ▀███▀▀▀▀    ███      ▀▀███▀▀▀███  ▀▀▀▀▀▀███
███   ███  ███   ███   ███  ███   ███ ██████████  ███   █▄   ███   ███  ▄██   ███
███   ███  ███   ███   ███  ███   ███  ███   ███  ███   ███  ███   ███  ███   ███
 ▀█████▀    ▀█   ███   █▀   ███   █▀   ███   ███  ███████▀   ███   █▀    ▀█████▀ 
                                       ███   █▀              OMARCHYiso v1.0.0   
"#;

impl Ui {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            filter_text: String::new(),
            error_scroll_offset: 0,
        }
    }

    pub fn reset_state(&mut self) {
        self.selected_index = 0;
        self.filter_text.clear();
        self.error_scroll_offset = 0;
    }

    pub fn render(
        &mut self,
        f: &mut Frame,
        state: &AppState,
        config: &Config,
        error_message: &Option<String>,
        build_output: &[String],
    ) {
        let size = f.area();

        match state {
            AppState::Welcome => self.render_welcome(f, size, config),
            AppState::SelectingAurPackages => self.render_package_selection(
                f,
                size,
                "AUR Packages",
                &config.aur_packages,
                &config.selected_aur,
                1,
                "packages",
            ),
            AppState::SelectingOfficialPackages => self.render_package_selection(
                f,
                size,
                "Official Packages",
                &config.official_packages,
                &config.selected_official,
                2,
                "packages",
            ),
            AppState::SelectingIgnoredPackages => self.render_package_selection(
                f,
                size,
                "Packages to Exclude",
                &config.omarchy_packages,
                &config.selected_ignored,
                3,
                "packages",
            ),
            AppState::SelectingDotfiles => {
                self.render_dotfiles_selection(f, size, &config.dotfiles, &config.selected_dotfiles)
            }
            AppState::SelectingHomeDotfiles => {
                self.render_home_dotfiles_selection(f, size, &config.home_dotfiles, &config.selected_home_dotfiles)
            }
            AppState::Summary => self.render_summary(f, size, config),
            AppState::Building => self.render_building(f, size, build_output),
            AppState::Complete => self.render_complete(f, size, config),
            AppState::Error => self.render_error(f, size, error_message, build_output),
        }
    }

    fn get_step_indicator(&self, current_step: usize) -> Line<'_> {
        let steps = [
            ("1", "AUR"),
            ("2", "Official"),
            ("3", "Exclude"),
            ("4", ".config"),
            ("5", "Home"),
            ("6", "Build"),
        ];

        let mut spans = vec![Span::raw("Steps: ")];

        for (i, (num, label)) in steps.iter().enumerate() {
            let step_num = i + 1;
            let style = if step_num < current_step {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::DIM)
            } else if step_num == current_step {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            spans.push(Span::styled(format!("[{}:{}]", num, label), style));
            if i < steps.len() - 1 {
                spans.push(Span::raw(" → "));
            }
        }

        Line::from(spans)
    }

    fn render_welcome(&self, f: &mut Frame, area: Rect, config: &Config) {
        // Main container
        let main_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = main_block.inner(area);
        f.render_widget(main_block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(11), // Logo (10 lines + 1 for spacing)
                Constraint::Length(1),  // Spacer
                Constraint::Min(8),     // Description
                Constraint::Length(1),  // Spacer
                Constraint::Length(3),  // Controls
            ])
            .split(inner);

        // Logo
        let logo = Paragraph::new(LOGO)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center);
        f.render_widget(logo, chunks[0]);

        // Description box
        let current_dir = config
            .work_dir
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or(".");
        
        let desc_text = vec![
            Line::from(Span::styled(
                "Create Your Custom Omarchy Linux ISO",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("✓ ", Style::default().fg(Color::Green)),
                Span::raw("Include your personal dotfiles"),
            ]),
            Line::from(vec![
                Span::styled("✓ ", Style::default().fg(Color::Green)),
                Span::raw("Add your installed AUR & official Arch packages"),
            ]),
            Line::from(vec![
                Span::styled("✓ ", Style::default().fg(Color::Green)),
                Span::raw("Exclude unwanted default packages installed by Omarchy"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("ISO will be created in: "),
                Span::styled(
                    current_dir,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
        ];

        let desc_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Blue));

        let description = Paragraph::new(desc_text)
            .block(desc_block)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true });

        f.render_widget(description, chunks[2]);

        // Controls
        let controls = Paragraph::new(vec![Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::White)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to begin  |  ", Style::default().fg(Color::White)),
            Span::styled(
                "Q",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to quit", Style::default().fg(Color::White)),
        ])])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

        f.render_widget(controls, chunks[4]);
    }

    fn render_package_selection(
        &mut self,
        f: &mut Frame,
        area: Rect,
        title: &str,
        packages: &[String],
        selected: &[bool],
        step: usize,
        item_label: &str,
    ) {
        // Main container
        let main_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = main_block.inner(area);
        f.render_widget(main_block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Header with step indicator
                Constraint::Length(3), // Search bar
                Constraint::Min(10),   // Package list
                Constraint::Length(3), // Status bar
                Constraint::Length(3), // Controls
            ])
            .split(inner);

        // Header with step indicator
        let step_line = self.get_step_indicator(step);
        let header_text = vec![
            Line::from(Span::styled(
                title,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            step_line,
        ];
        let header = Paragraph::new(header_text)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Blue)),
            );
        f.render_widget(header, chunks[0]);

        // Search bar
        let search_text = if !self.filter_text.is_empty() {
            format!("{}_", self.filter_text)
        } else {
            "Type to search...".to_string()
        };
        let search_style = if !self.filter_text.is_empty() {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let search = Paragraph::new(search_text)
            .style(search_style)
            .alignment(Alignment::Left)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Magenta))
                    .title(" Search "),
            );
        f.render_widget(search, chunks[1]);

        // Filter packages based on filter text
        let filtered: Vec<(usize, &String)> = packages
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                self.filter_text.is_empty()
                    || p.to_lowercase().contains(&self.filter_text.to_lowercase())
            })
            .collect();

        // Package list with better styling
        let selected_count = filtered
            .iter()
            .filter(|(idx, _)| selected.get(*idx).copied().unwrap_or(false))
            .count();

        let items: Vec<ListItem> = filtered
            .iter()
            .map(|(idx, pkg)| {
                let is_selected = selected.get(*idx).copied().unwrap_or(false);
                let checkbox = if is_selected { "✓" } else { " " };
                let checkbox_style = if is_selected {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let line = Line::from(vec![
                    Span::raw("["),
                    Span::styled(checkbox, checkbox_style),
                    Span::raw("] "),
                    Span::raw(pkg.as_str()),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list_title = format!(" {} / {} selected ", selected_count, filtered.len());
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Green))
                    .title(list_title),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(40, 40, 60))
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        let mut list_state = ListState::default();
        list_state.select(Some(
            self.selected_index.min(filtered.len().saturating_sub(1)),
        ));
        f.render_stateful_widget(list, chunks[2], &mut list_state);

        // Status bar
        let status_text = if !self.filter_text.is_empty() {
            format!("Showing {} of {} {}", filtered.len(), packages.len(), item_label)
        } else {
            format!("Total: {} {}", packages.len(), item_label)
        };
        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        f.render_widget(status, chunks[3]);

        // Controls
        let controls = Paragraph::new(vec![Line::from(vec![
            Span::styled(
                "Space",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" toggle  ", Style::default().fg(Color::White)),
            Span::styled(
                "↑↓",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" navigate  ", Style::default().fg(Color::White)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" continue  ", Style::default().fg(Color::White)),
            Span::styled(
                "Backspace",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" clear  ", Style::default().fg(Color::White)),
            Span::styled(
                "Q",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" quit", Style::default().fg(Color::White)),
        ])])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        f.render_widget(controls, chunks[4]);
    }

    fn render_dotfiles_selection(
        &mut self,
        f: &mut Frame,
        area: Rect,
        dotfiles: &[String],
        selected: &[bool],
    ) {
        self.render_package_selection(f, area, ".config Files & Folders", dotfiles, selected, 4, "items");
    }

    fn render_home_dotfiles_selection(
        &mut self,
        f: &mut Frame,
        area: Rect,
        dotfiles: &[String],
        selected: &[bool],
    ) {
        self.render_package_selection(f, area, "Home Dotfiles", dotfiles, selected, 5, "items");
    }

    fn render_summary(&self, f: &mut Frame, area: Rect, config: &Config) {
        // Main container
        let main_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = main_block.inner(area);
        f.render_widget(main_block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(10),   // Summary content
                Constraint::Length(6), // Controls
            ])
            .split(inner);

        // Header
        let step_line = self.get_step_indicator(6);
        let header_text = vec![
            Line::from(Span::styled(
                "Build Summary",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            step_line,
        ];
        let header = Paragraph::new(header_text)
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Blue)),
            );
        f.render_widget(header, chunks[0]);

        // Summary content with sections
        let selected_dotfiles = config.get_selected_dotfiles();
        let selected_official = config.get_selected_official();
        let selected_aur = config.get_selected_aur();
        let selected_ignored = config.get_selected_ignored();
        let selected_home_dotfiles = config.get_selected_home_dotfiles();

        let mut text = vec![Line::from("")];

        // Dotfiles section
        let total_dotfiles = selected_dotfiles.len() + selected_home_dotfiles.len();
        text.push(Line::from(vec![
            Span::styled(
                "Dotfiles: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}", total_dotfiles),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        
        // Show .config dotfiles
        if !selected_dotfiles.is_empty() {
            text.push(Line::from(Span::styled(
                "   .config:",
                Style::default().fg(Color::Cyan),
            )));
            for item in selected_dotfiles.iter().take(3) {
                text.push(Line::from(format!("     • {}", item)));
            }
            if selected_dotfiles.len() > 3 {
                text.push(Line::from(Span::styled(
                    format!("     ... and {} more", selected_dotfiles.len() - 3),
                    Style::default().fg(Color::DarkGray).italic(),
                )));
            }
        }
        
        // Show home dir dotfiles
        if !selected_home_dotfiles.is_empty() {
            text.push(Line::from(Span::styled(
                "   Home:",
                Style::default().fg(Color::Cyan),
            )));
            for item in selected_home_dotfiles.iter().take(3) {
                text.push(Line::from(format!("     • {}", item)));
            }
            if selected_home_dotfiles.len() > 3 {
                text.push(Line::from(Span::styled(
                    format!("     ... and {} more", selected_home_dotfiles.len() - 3),
                    Style::default().fg(Color::DarkGray).italic(),
                )));
            }
        }
        
        text.push(Line::from(""));

        // Official packages section
        text.push(Line::from(vec![
            Span::styled(
                "Official Packages: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}", selected_official.len()),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        for pkg in selected_official.iter().take(5) {
            text.push(Line::from(format!("   • {}", pkg)));
        }
        if selected_official.len() > 5 {
            text.push(Line::from(Span::styled(
                format!("   ... and {} more", selected_official.len() - 5),
                Style::default().fg(Color::DarkGray).italic(),
            )));
        }
        text.push(Line::from(""));

        // AUR packages section
        text.push(Line::from(vec![
            Span::styled(
                "AUR Packages: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}", selected_aur.len()),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        for pkg in selected_aur.iter().take(5) {
            text.push(Line::from(format!("   • {}", pkg)));
        }
        if selected_aur.len() > 5 {
            text.push(Line::from(Span::styled(
                format!("   ... and {} more", selected_aur.len() - 5),
                Style::default().fg(Color::DarkGray).italic(),
            )));
        }
        text.push(Line::from(""));

        // Excluded packages section
        text.push(Line::from(vec![
            Span::styled(
                "Excluded Packages: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{}", selected_ignored.len()),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
        ]));

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Green))
                    .title(" Review Your Configuration "),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, chunks[1]);

        // Controls
        let controls = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "WARNING: ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Build will take 15-60 minutes!",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press ", Style::default().fg(Color::White)),
                Span::styled(
                    "Enter",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                        .add_modifier(Modifier::UNDERLINED),
                ),
                Span::styled(" to START BUILD  |  ", Style::default().fg(Color::White)),
                Span::styled(
                    "Q",
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD)
                        .add_modifier(Modifier::UNDERLINED),
                ),
                Span::styled(" to QUIT", Style::default().fg(Color::White)),
            ]),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Color::Yellow)),
        );
        f.render_widget(controls, chunks[2]);
    }

    fn render_building(&self, f: &mut Frame, area: Rect, build_output: &[String]) {
        // Main container
        let main_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Yellow));

        let inner = main_block.inner(area);
        f.render_widget(main_block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(10),   // Output log
                Constraint::Length(3), // Status bar
            ])
            .split(inner);

        // Title
        let spinner = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";
        let spinner_idx = (build_output.len() / 2) % spinner.len();
        let spinner_char = spinner.chars().nth(spinner_idx).unwrap_or('⠋');

        let title = Paragraph::new(vec![Line::from(vec![
            Span::styled(
                format!("{} ", spinner_char),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "Building ISO",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {}", spinner_char),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ])])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Color::Yellow)),
        );
        f.render_widget(title, chunks[0]);

        // Build output log
        let output_text: Vec<Line> = if build_output.is_empty() {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Initializing build process...",
                    Style::default().fg(Color::Cyan),
                )),
            ]
        } else {
            // Show last N lines that fit in the area
            let available_height = chunks[1].height.saturating_sub(2) as usize; // Account for borders
            let start_idx = build_output.len().saturating_sub(available_height);

            build_output[start_idx..]
                .iter()
                .map(|line| {
                    if line.starts_with('✓') {
                        Line::from(Span::styled(
                            line.as_str(),
                            Style::default().fg(Color::Green),
                        ))
                    } else if line.starts_with('❌') || line.starts_with('⚠') {
                        Line::from(Span::styled(line.as_str(), Style::default().fg(Color::Red)))
                    } else if line.starts_with('🔧')
                        || line.starts_with('📝')
                        || line.starts_with('📁')
                        || line.starts_with('🔨')
                        || line.starts_with('🚀')
                        || line.starts_with('📦')
                    {
                        Line::from(Span::styled(
                            line.as_str(),
                            Style::default().fg(Color::Cyan),
                        ))
                    } else if line.starts_with('⏱') {
                        Line::from(Span::styled(
                            line.as_str(),
                            Style::default().fg(Color::Yellow),
                        ))
                    } else {
                        Line::from(line.as_str())
                    }
                })
                .collect()
        };

        let output_log = Paragraph::new(output_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Blue))
                    .title(format!(" Build Log ({} lines) ", build_output.len())),
            )
            .wrap(Wrap { trim: false });
        f.render_widget(output_log, chunks[1]);

        // Status bar
        let status = Paragraph::new(vec![Line::from(vec![
            Span::styled(
                "Do not interrupt this process  |  ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Press ", Style::default().fg(Color::White)),
            Span::styled(
                "Q",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to force quit (will terminate build)",
                Style::default().fg(Color::White),
            ),
        ])])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        f.render_widget(status, chunks[2]);
    }

    fn render_complete(&self, f: &mut Frame, area: Rect, config: &Config) {
        // Main container
        let main_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Green));

        let inner = main_block.inner(area);
        f.render_widget(main_block, area);

        let centered = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Length(18),
                Constraint::Percentage(30),
            ])
            .split(inner);

        let content_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(5), // Success banner
                Constraint::Length(1), // Spacer
                Constraint::Length(5), // ISO location
                Constraint::Length(1), // Spacer
                Constraint::Length(3), // Controls
            ])
            .split(centered[1]);

        // Success banner
        let success_art = vec![
            Line::from(Span::styled(
                "╔═══════════════════════════════════╗",
                Style::default().fg(Color::Green),
            )),
            Line::from(vec![
                Span::styled("║  ", Style::default().fg(Color::Green)),
                Span::styled(
                    "✓ ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "BUILD SUCCESSFUL",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("           ║", Style::default().fg(Color::Green)),
            ]),
            Line::from(Span::styled(
                "╚═══════════════════════════════════╝",
                Style::default().fg(Color::Green),
            )),
        ];
        let success = Paragraph::new(success_art).alignment(Alignment::Center);
        f.render_widget(success, content_chunks[0]);

        // ISO location
        let iso_path = config.iso_path.as_deref().unwrap_or("Current folder");
        let iso_text = vec![
            Line::from(vec![
                Span::styled(
                    "ISO Location: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    iso_path,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
        ];
        let iso_info = Paragraph::new(iso_text).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Blue)),
        );
        f.render_widget(iso_info, content_chunks[2]);

        // Controls
        let controls = Paragraph::new(vec![Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::White)),
            Span::styled(
                "Q",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to quit", Style::default().fg(Color::White)),
        ])])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        f.render_widget(controls, content_chunks[4]);
    }

    fn render_error(
        &self,
        f: &mut Frame,
        area: Rect,
        error_message: &Option<String>,
        build_output: &[String],
    ) {
        // Main container
        let main_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Red));

        let inner = main_block.inner(area);
        f.render_widget(main_block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(5), // Error banner
                Constraint::Length(4), // Error message
                Constraint::Min(10),   // Build log
                Constraint::Length(3), // Controls
            ])
            .split(inner);

        // Error banner
        let error_art = vec![
            Line::from(Span::styled(
                "╔═══════════════════════════════════╗",
                Style::default().fg(Color::Red),
            )),
            Line::from(vec![
                Span::styled("║  ", Style::default().fg(Color::Red)),
                Span::styled(
                    "✗ ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "BUILD FAILED",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled("                 ║", Style::default().fg(Color::Red)),
            ]),
            Line::from(Span::styled(
                "╚═══════════════════════════════════╝",
                Style::default().fg(Color::Red),
            )),
        ];
        let error_banner = Paragraph::new(error_art).alignment(Alignment::Center);
        f.render_widget(error_banner, chunks[0]);

        // Error message
        let error_text = error_message.as_deref().unwrap_or("Unknown error occurred");
        let error_lines = vec![Line::from(vec![
            Span::styled("⚠ ", Style::default().fg(Color::Yellow)),
            Span::styled(
                "Error: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(error_text, Style::default().fg(Color::White)),
        ])];
        let error_info = Paragraph::new(error_lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Yellow)),
            );
        f.render_widget(error_info, chunks[1]);

        // Build log (show the output so user can troubleshoot)
        let output_text: Vec<Line> = if build_output.is_empty() {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No build output available",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        } else {
            // Calculate visible range with scroll offset
            let available_height = chunks[2].height.saturating_sub(2) as usize;
            let total_lines = build_output.len();
            let max_scroll = total_lines.saturating_sub(available_height);

            // Default to showing the end of the log (where errors usually are)
            // But allow user to scroll
            let default_scroll = max_scroll;
            let scroll_offset = if self.error_scroll_offset == 0 && max_scroll > 0 {
                default_scroll
            } else {
                self.error_scroll_offset.min(max_scroll)
            };
            
            let end_idx = (scroll_offset + available_height).min(total_lines);

            build_output[scroll_offset..end_idx]
                .iter()
                .map(|line| {
                    // Color coded based on content - highlight errors
                    if line.starts_with('✓') {
                        Line::from(Span::styled(
                            line.as_str(),
                            Style::default().fg(Color::Green),
                        ))
                    } else if line.starts_with('❌') {
                        Line::from(Span::styled(
                            line.as_str(),
                            Style::default()
                                .fg(Color::Red)
                                .add_modifier(Modifier::BOLD),
                        ))
                    } else if line.starts_with('⚠') || line.contains("error") || line.contains("Error") || line.contains("ERROR") || line.contains("failed") || line.contains("Failed") {
                        Line::from(Span::styled(
                            line.as_str(),
                            Style::default()
                                .fg(Color::Red)
                                .add_modifier(Modifier::BOLD),
                        ))
                    } else if line.starts_with('🔧')
                        || line.starts_with('📝')
                        || line.starts_with('📁')
                        || line.starts_with('🔨')
                        || line.starts_with('🚀')
                        || line.starts_with('📦')
                    {
                        Line::from(Span::styled(
                            line.as_str(),
                            Style::default().fg(Color::Cyan),
                        ))
                    } else if line.starts_with('⏱') {
                        Line::from(Span::styled(
                            line.as_str(),
                            Style::default().fg(Color::Yellow),
                        ))
                    } else {
                        Line::from(line.as_str())
                    }
                })
                .collect()
        };

        let scroll_info = if build_output.len() > chunks[2].height.saturating_sub(2) as usize {
            format!(
                " Build Log ({} lines) - Scroll with ↑↓ - Showing last lines ",
                build_output.len()
            )
        } else {
            format!(" Build Log ({} lines) ", build_output.len())
        };

        let output_log = Paragraph::new(output_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Red))
                    .title(scroll_info),
            )
            .wrap(Wrap { trim: false });
        f.render_widget(output_log, chunks[2]);

        // Controls
        let controls = Paragraph::new(vec![Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::White)),
            Span::styled(
                "Q",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to quit  |  ", Style::default().fg(Color::White)),
            Span::styled(
                "Check the build log above for error details",
                Style::default().fg(Color::Yellow),
            ),
        ])])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        f.render_widget(controls, chunks[3]);
    }

    pub fn handle_up(&mut self, state: &AppState) {
        match state {
            AppState::Error => {
                if self.error_scroll_offset > 0 {
                    self.error_scroll_offset -= 1;
                }
            }
            _ => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
        }
    }

    pub fn handle_down(&mut self, state: &AppState, config: &Config) {
        match state {
            AppState::Error => {
                // Allow scrolling down in error log
                self.error_scroll_offset += 1;
            }
            _ => {
                let max_index = match state {
                    AppState::SelectingAurPackages => {
                        self.get_filtered_indices(&config.aur_packages).len()
                    }
                    AppState::SelectingOfficialPackages => {
                        self.get_filtered_indices(&config.official_packages).len()
                    }
                    AppState::SelectingIgnoredPackages => {
                        self.get_filtered_indices(&config.omarchy_packages).len()
                    }
                    AppState::SelectingDotfiles => {
                        self.get_filtered_indices(&config.dotfiles).len()
                    }
                    AppState::SelectingHomeDotfiles => {
                        self.get_filtered_indices(&config.home_dotfiles).len()
                    }
                    _ => 0,
                };

                if max_index > 0 && self.selected_index < max_index - 1 {
                    self.selected_index += 1;
                }
            }
        }
    }

    pub fn handle_space(&mut self, state: &AppState, config: &mut Config) {
        match state {
            AppState::SelectingAurPackages => {
                let filtered = self.get_filtered_indices(&config.aur_packages);
                if let Some(&actual_idx) = filtered.get(self.selected_index)
                    && actual_idx < config.selected_aur.len()
                {
                    config.selected_aur[actual_idx] = !config.selected_aur[actual_idx];
                }
            }
            AppState::SelectingOfficialPackages => {
                let filtered = self.get_filtered_indices(&config.official_packages);
                if let Some(&actual_idx) = filtered.get(self.selected_index)
                    && actual_idx < config.selected_official.len()
                {
                    config.selected_official[actual_idx] = !config.selected_official[actual_idx];
                }
            }
            AppState::SelectingIgnoredPackages => {
                let filtered = self.get_filtered_indices(&config.omarchy_packages);
                if let Some(&actual_idx) = filtered.get(self.selected_index)
                    && actual_idx < config.selected_ignored.len()
                {
                    config.selected_ignored[actual_idx] = !config.selected_ignored[actual_idx];
                }
            }
            AppState::SelectingDotfiles => {
                let filtered = self.get_filtered_indices(&config.dotfiles);
                if let Some(&actual_idx) = filtered.get(self.selected_index)
                    && actual_idx < config.selected_dotfiles.len()
                {
                    config.selected_dotfiles[actual_idx] = !config.selected_dotfiles[actual_idx];
                }
            }
            AppState::SelectingHomeDotfiles => {
                let filtered = self.get_filtered_indices(&config.home_dotfiles);
                if let Some(&actual_idx) = filtered.get(self.selected_index)
                    && actual_idx < config.selected_home_dotfiles.len()
                {
                    config.selected_home_dotfiles[actual_idx] = !config.selected_home_dotfiles[actual_idx];
                }
            }
            _ => {}
        }
    }

    fn get_filtered_indices(&self, items: &[String]) -> Vec<usize> {
        items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                self.filter_text.is_empty()
                    || item
                        .to_lowercase()
                        .contains(&self.filter_text.to_lowercase())
            })
            .map(|(idx, _)| idx)
            .collect()
    }

    pub fn handle_char(&mut self, c: char, state: &AppState) {
        match state {
            AppState::SelectingAurPackages
            | AppState::SelectingOfficialPackages
            | AppState::SelectingIgnoredPackages
            | AppState::SelectingDotfiles
            | AppState::SelectingHomeDotfiles => {
                self.filter_text.push(c);
                self.selected_index = 0;
            }
            _ => {}
        }
    }

    pub fn handle_backspace(&mut self, state: &AppState) {
        match state {
            AppState::SelectingAurPackages
            | AppState::SelectingOfficialPackages
            | AppState::SelectingIgnoredPackages
            | AppState::SelectingDotfiles
            | AppState::SelectingHomeDotfiles => {
                self.filter_text.pop();
                self.selected_index = 0;
            }
            _ => {}
        }
    }
}
