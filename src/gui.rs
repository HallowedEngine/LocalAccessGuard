use crate::VERSION;
use eframe::egui::{
    self, Align, Button, Color32, CornerRadius, FontId, Frame, Layout, Margin, RichText, Stroke,
    TextEdit, Vec2,
};
use std::env;
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusLevel {
    Idle,
    Ok,
    Warning,
    Failed,
    Info,
}

impl StatusLevel {
    fn label(self) -> &'static str {
        match self {
            Self::Idle => "IDLE",
            Self::Ok => "OK",
            Self::Warning => "WARNING",
            Self::Failed => "FAILED",
            Self::Info => "INFO",
        }
    }

    fn color(self) -> Color32 {
        match self {
            Self::Idle => Color32::from_rgb(127, 140, 158),
            Self::Ok => Color32::from_rgb(63, 216, 145),
            Self::Warning => Color32::from_rgb(239, 177, 72),
            Self::Failed => Color32::from_rgb(241, 86, 86),
            Self::Info => Color32::from_rgb(91, 171, 255),
        }
    }

    fn fill(self) -> Color32 {
        match self {
            Self::Idle => Color32::from_rgb(33, 42, 55),
            Self::Ok => Color32::from_rgb(16, 63, 50),
            Self::Warning => Color32::from_rgb(76, 54, 21),
            Self::Failed => Color32::from_rgb(77, 28, 34),
            Self::Info => Color32::from_rgb(21, 50, 80),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Dashboard,
    Reports,
    Advanced,
    About,
}

#[derive(Debug, Clone)]
struct DashboardCard {
    title: &'static str,
    status: StatusLevel,
    detail: String,
}

impl DashboardCard {
    fn new(title: &'static str) -> Self {
        Self {
            title,
            status: StatusLevel::Idle,
            detail: "Waiting for data.".to_string(),
        }
    }

    fn set(&mut self, status: StatusLevel, detail: impl Into<String>) {
        self.status = status;
        self.detail = detail.into();
    }
}

#[derive(Debug, Clone)]
struct CommandSpec {
    label: String,
    args: Vec<String>,
}

impl CommandSpec {
    fn new(label: &str, args: &[&str]) -> Self {
        Self {
            label: label.to_string(),
            args: args.iter().map(|arg| arg.to_string()).collect(),
        }
    }
}

#[derive(Debug)]
struct CommandResult {
    label: String,
    output: String,
    success: bool,
}

#[derive(Debug)]
struct WorkerResult {
    label: String,
    results: Vec<CommandResult>,
}

pub fn launch() {
    let title = format!("LocalAccessGuard {}", VERSION);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title.clone())
            .with_inner_size(Vec2::new(1180.0, 720.0))
            .with_min_inner_size(Vec2::new(980.0, 620.0)),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        &title,
        options,
        Box::new(|cc| Ok(Box::new(LocalAccessGuardApp::new(cc)))),
    ) {
        eprintln!("[FAILED] Failed to open GUI: {}", err);
    }
}

struct LocalAccessGuardApp {
    overall_status: StatusLevel,
    cards: Vec<DashboardCard>,
    is_running: bool,
    running_command: String,
    latest_output: String,
    latest_report_path: Option<String>,
    receiver: Option<Receiver<WorkerResult>>,
    screen: Screen,
    findings: Vec<String>,
    suggestions: Vec<String>,
}

impl LocalAccessGuardApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut style = (*cc.egui_ctx.global_style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.window_fill = Color32::from_rgb(7, 10, 17);
        style.visuals.panel_fill = Color32::from_rgb(7, 10, 17);
        style.visuals.extreme_bg_color = Color32::from_rgb(5, 8, 13);
        style.visuals.override_text_color = Some(Color32::from_rgb(228, 234, 244));
        style.spacing.item_spacing = Vec2::new(7.0, 7.0);
        style.spacing.button_padding = Vec2::new(8.0, 5.0);
        cc.egui_ctx.set_global_style(style);

        Self {
            overall_status: StatusLevel::Idle,
            cards: vec![
                DashboardCard::new("Proxy"),
                DashboardCard::new("PAC"),
                DashboardCard::new("WinHTTP"),
                DashboardCard::new("UDP"),
                DashboardCard::new("WARP"),
                DashboardCard::new("Firewall"),
                DashboardCard::new("Services"),
                DashboardCard::new("Profiles"),
                DashboardCard::new("Reports"),
            ],
            is_running: false,
            running_command: String::new(),
            latest_output: "Ready. Run Health Check or choose a check.".to_string(),
            latest_report_path: None,
            receiver: None,
            screen: Screen::Dashboard,
            findings: vec!["Run Health Check to populate findings.".to_string()],
            suggestions: vec![
                "Generate a report before and after changes.".to_string(),
                "Restore remains CLI-only for safety.".to_string(),
            ],
        }
    }

    fn start_commands(&mut self, label: &str, commands: Vec<CommandSpec>) {
        if self.is_running {
            return;
        }

        let (sender, receiver) = mpsc::channel();
        let worker_label = label.to_string();

        self.is_running = true;
        self.running_command = worker_label.clone();
        self.latest_output = format!("Running: {}...", worker_label);
        self.receiver = Some(receiver);

        thread::spawn(move || {
            let results = commands
                .iter()
                .map(|command| run_command(&command.label, &command.args))
                .collect();
            let _ = sender.send(WorkerResult {
                label: worker_label,
                results,
            });
        });
    }

    fn run_full_scan(&mut self) {
        self.start_commands(
            "Health Check",
            vec![
                CommandSpec::new("Status", &["status"]),
                CommandSpec::new("Config", &["config"]),
                CommandSpec::new("Profiles", &["profiles"]),
                CommandSpec::new("Groups", &["groups"]),
                CommandSpec::new("Netinfo", &["netinfo"]),
                CommandSpec::new("UDP", &["udpcheck"]),
                CommandSpec::new("Firewall", &["firewall-check"]),
                CommandSpec::new("Services", &["services"]),
                CommandSpec::new("System Doctor", &["doctor-system"]),
            ],
        );
    }

    fn run_single(&mut self, label: &str, args: &[&str]) {
        self.start_commands(label, vec![CommandSpec::new(label, args)]);
    }

    fn poll_worker(&mut self, ctx: &egui::Context) {
        let Some(receiver) = self.receiver.take() else {
            return;
        };

        match receiver.try_recv() {
            Ok(result) => {
                self.apply_worker_result(result);
                self.is_running = false;
                self.running_command.clear();
                ctx.request_repaint();
            }
            Err(mpsc::TryRecvError::Empty) => {
                self.receiver = Some(receiver);
                ctx.request_repaint_after(Duration::from_millis(80));
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.latest_output = "[FAILED] Background worker disconnected.".to_string();
                self.overall_status = StatusLevel::Failed;
                self.is_running = false;
                self.running_command.clear();
            }
        }
    }

    fn apply_worker_result(&mut self, worker_result: WorkerResult) {
        let mut combined = String::new();
        let mut any_failure = false;

        for result in &worker_result.results {
            any_failure |= !result.success;
            append_result(&mut combined, result);

            if result.label.contains("Report") && result.success {
                if let Some(path) = extract_report_path(&result.output) {
                    self.latest_report_path = Some(path.clone());
                    self.set_card("Reports", StatusLevel::Ok, format!("Saved: {}", path));
                }
            }
        }

        self.latest_output = combined;
        self.update_cards_from_output();
        self.update_findings_and_suggestions();

        self.overall_status = if any_failure || self.latest_output.contains("[FAILED]") {
            StatusLevel::Failed
        } else if contains_warning_signal(&self.latest_output) {
            StatusLevel::Warning
        } else {
            StatusLevel::Ok
        };

        if worker_result.label == "Health Check" {
            self.set_card(
                "Profiles",
                StatusLevel::Info,
                "Profiles and groups inspected.",
            );
        }
    }

    fn update_findings_and_suggestions(&mut self) {
        let text = &self.latest_output;
        let mut findings = Vec::new();
        let mut suggestions = Vec::new();

        if text.contains("Cloudflare WARP service is running") {
            findings.push("Cloudflare WARP is running.".to_string());
            suggestions.push(
                "If you are not using Cloudflare WARP, close it from the official app.".to_string(),
            );
        }

        if text.contains("Action: Block") {
            findings.push("Firewall block rules were found.".to_string());
        }

        if text.contains("ProxyEnable: Disabled") && text.contains("ProxyServer: Not set") {
            findings.push("Proxy settings look clean.".to_string());
        }

        if text.contains("AutoConfigURL: Not set") && text.contains("Direct access") {
            findings.push("PAC and WinHTTP settings look clean.".to_string());
        }

        if text.contains("UDP socket bind: OK") && text.contains("UDP connect test: OK") {
            findings.push("UDP check passed.".to_string());
        }

        if findings.is_empty() && !text.contains("[FAILED]") {
            findings.push("No major issue found.".to_string());
        }

        if text.contains("[FAILED]") {
            suggestions.push("Open Advanced and review the raw command output.".to_string());
        }

        suggestions.push("Generate a report before and after changes.".to_string());
        suggestions.push("Use compare from the CLI to check what changed.".to_string());
        suggestions.push("Restore remains CLI-only for safety.".to_string());

        self.findings = findings;
        self.suggestions = suggestions;
    }

    fn update_cards_from_output(&mut self) {
        let text = self.latest_output.clone();

        if text.contains("ProxyEnable: Disabled") && text.contains("ProxyServer: Not set") {
            self.set_card("Proxy", StatusLevel::Ok, "Disabled and not set.");
        } else if text.contains("ProxyEnable: Enabled")
            || text.contains("Windows proxy is currently active")
        {
            self.set_card("Proxy", StatusLevel::Warning, "Proxy appears active.");
        }

        if text.contains("AutoConfigURL: Not set") {
            self.set_card("PAC", StatusLevel::Ok, "No PAC URL set.");
        } else if text.contains("PAC proxy config exists") || text.contains("AutoConfigURL:") {
            self.set_card("PAC", StatusLevel::Warning, "PAC may be configured.");
        }

        if text.contains("Direct access") {
            self.set_card("WinHTTP", StatusLevel::Ok, "Direct access.");
        } else if text.contains("WinHTTP") && text.contains("Error") {
            self.set_card("WinHTTP", StatusLevel::Failed, "Read failed.");
        }

        if text.contains("UDP socket bind: OK") && text.contains("UDP connect test: OK") {
            self.set_card("UDP", StatusLevel::Ok, "Bind and connect OK.");
        } else if text.contains("UDP socket bind: FAILED")
            || text.contains("UDP connect test: FAILED")
        {
            self.set_card("UDP", StatusLevel::Failed, "UDP check failed.");
        }

        if text.contains("Cloudflare WARP service is running") {
            self.set_card("WARP", StatusLevel::Warning, "WARP service running.");
        } else if text.contains("warp-svc.exe") || text.contains("CloudflareWARP") {
            self.set_card("WARP", StatusLevel::Info, "WARP inspected.");
        }

        if text.contains("Action: Block") {
            self.set_card("Firewall", StatusLevel::Warning, "Block rule found.");
        } else if text.contains("Firewall") || text.contains("firewall") {
            self.set_card("Firewall", StatusLevel::Info, "Rules inspected.");
        }

        if text.contains("CloudflareWARP") && text.contains("Running") {
            self.set_card("Services", StatusLevel::Warning, "CloudflareWARP running.");
        } else if text.contains("Services") || text.contains("service") {
            self.set_card("Services", StatusLevel::Info, "Services inspected.");
        }

        if text.contains("DNS ") || text.contains("TCP 443") || text.contains("groups") {
            self.set_card("Profiles", StatusLevel::Info, "Profile data inspected.");
        }
    }

    fn set_card(&mut self, title: &str, status: StatusLevel, detail: impl Into<String>) {
        if let Some(card) = self.cards.iter_mut().find(|card| card.title == title) {
            card.set(status, detail);
        }
    }

    fn draw_top_bar(&self, ui: &mut egui::Ui) {
        Frame::new()
            .fill(Color32::from_rgb(12, 18, 30))
            .stroke(Stroke::new(1.0, Color32::from_rgb(38, 53, 75)))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("LocalAccessGuard")
                            .font(FontId::proportional(19.0))
                            .strong(),
                    );
                    ui.label(RichText::new(VERSION).color(Color32::from_rgb(91, 171, 255)));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if self.is_running {
                            ui.label(
                                RichText::new(format!("Running: {}...", self.running_command))
                                    .color(Color32::from_rgb(239, 177, 72)),
                            );
                        }
                        status_pill(ui, "Overall", self.overall_status);
                    });
                });
            });
    }

    fn draw_sidebar(&mut self, ui: &mut egui::Ui) {
        Frame::new()
            .fill(Color32::from_rgb(10, 15, 25))
            .stroke(Stroke::new(1.0, Color32::from_rgb(34, 48, 69)))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                ui.set_width(198.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("LocalAccessGuard")
                            .font(FontId::proportional(17.0))
                            .strong(),
                    );
                    ui.label(
                        RichText::new("Network Guard Console")
                            .font(FontId::proportional(12.0))
                            .color(Color32::from_rgb(143, 160, 187)),
                    );
                    ui.label(
                        RichText::new(VERSION)
                            .font(FontId::proportional(12.0))
                            .color(Color32::from_rgb(91, 171, 255)),
                    );
                });

                ui.add_space(10.0);
                egui::ScrollArea::vertical()
                    .id_salt("actions_scroll_area")
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.draw_navigation(ui));
            });
    }

    fn draw_navigation(&mut self, ui: &mut egui::Ui) {
        action_group(ui, "Navigation", |ui| {
            nav_button(ui, "Dashboard", &mut self.screen, Screen::Dashboard);
            nav_button(ui, "Reports", &mut self.screen, Screen::Reports);
            nav_button(ui, "Advanced", &mut self.screen, Screen::Advanced);
            nav_button(ui, "About", &mut self.screen, Screen::About);
        });

        action_group(ui, "Quick", |ui| {
            if action_button(ui, "Run Health Check", self.is_running).clicked() {
                self.run_full_scan();
            }
            if action_button(ui, "Quick Report", self.is_running).clicked() {
                self.run_single("TXT Report", &["report"]);
                self.screen = Screen::Reports;
            }
        });
    }

    fn draw_dashboard(&mut self, ui: &mut egui::Ui) {
        self.draw_cards(ui);
        ui.add_space(8.0);

        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() * 0.48).max(220.0));
                panel_list(ui, "Findings", &self.findings);
            });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                panel_list(ui, "Suggestions", &self.suggestions);
            });
        });

        ui.add_space(8.0);
        panel_frame(ui, |ui| {
            ui.label(
                RichText::new("Actions")
                    .font(FontId::proportional(16.0))
                    .strong(),
            );
            ui.horizontal_wrapped(|ui| {
                if action_button(ui, "Run Health Check", self.is_running).clicked() {
                    self.run_full_scan();
                }
                if action_button(ui, "Generate Quick Report", self.is_running).clicked() {
                    self.run_single("TXT Report", &["report"]);
                    self.screen = Screen::Reports;
                }
                if action_button(ui, "Check Discord", self.is_running).clicked() {
                    self.run_single("Check Discord", &["doctor", "discord"]);
                }
                if action_button(ui, "Check Roblox", self.is_running).clicked() {
                    self.run_single("Check Roblox", &["doctor", "roblox"]);
                }
            });
        });
    }

    fn draw_reports(&mut self, ui: &mut egui::Ui) {
        panel_frame(ui, |ui| {
            ui.label(
                RichText::new("Reports")
                    .font(FontId::proportional(17.0))
                    .strong(),
            );
            ui.label("TXT = human readable");
            ui.label("JSON = machine readable");
            ui.label("MD = GitHub/Markdown readable");
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if action_button(ui, "Generate TXT", self.is_running).clicked() {
                    self.run_single("TXT Report", &["report"]);
                }
                if action_button(ui, "Generate JSON", self.is_running).clicked() {
                    self.run_single("JSON Report", &["report", "--format", "json"]);
                }
                if action_button(ui, "Generate Markdown", self.is_running).clicked() {
                    self.run_single("MD Report", &["report", "--format", "md"]);
                }
            });
            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    self.latest_report_path
                        .as_deref()
                        .unwrap_or("No report generated in this GUI session."),
                )
                .color(Color32::from_rgb(132, 151, 178)),
            );
        });
        ui.add_space(8.0);
        self.draw_output(ui);
    }

    fn draw_advanced(&mut self, ui: &mut egui::Ui) {
        panel_frame(ui, |ui| {
            ui.label(
                RichText::new("Advanced")
                    .font(FontId::proportional(17.0))
                    .strong(),
            );
            ui.label(
                RichText::new(
                    "Advanced tools are read-only except restore, which is CLI-only and not available in the GUI.",
                )
                .color(Color32::from_rgb(239, 177, 72)),
            );
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                for (label, args) in [
                    ("Status", &["status"][..]),
                    ("Config", &["config"]),
                    ("Profiles", &["profiles"]),
                    ("Groups", &["groups"]),
                    ("Check Gaming Group", &["doctor-group", "gaming"]),
                    ("Netinfo", &["netinfo"]),
                    ("UDP Check", &["udpcheck"]),
                    ("Firewall Check", &["firewall-check"]),
                    ("Services", &["services"]),
                    ("System Doctor", &["doctor-system"]),
                    ("Help", &["help"]),
                ] {
                    if action_button(ui, label, self.is_running).clicked() {
                        self.run_single(label, args);
                    }
                }
            });
        });
        ui.add_space(8.0);
        self.draw_output(ui);
    }

    fn draw_about(&self, ui: &mut egui::Ui) {
        panel_frame(ui, |ui| {
            ui.label(
                RichText::new("About LocalAccessGuard")
                    .font(FontId::proportional(18.0))
                    .strong(),
            );
            ui.label("LocalAccessGuard is a Windows network diagnostic tool.");
            ui.add_space(8.0);
            ui.label(RichText::new("It can inspect:").strong());
            for item in [
                "Proxy / PAC / WinHTTP",
                "DNS / TCP / UDP",
                "Firewall rules",
                "Windows services",
                "Known network tools",
                "Discord / Roblox profiles",
                "Reports and logs",
            ] {
                ui.label(format!("- {}", item));
            }
            ui.add_space(8.0);
            ui.label(RichText::new("It is not:").strong());
            for item in [
                "VPN",
                "proxy engine",
                "DPI bypass tool",
                "packet manipulation tool",
            ] {
                ui.label(format!("- {}", item));
            }
        });
    }

    fn draw_cards(&self, ui: &mut egui::Ui) {
        let available_width = ui.available_width().max(1.0);
        let columns: usize = if available_width >= 1000.0 {
            3
        } else if available_width >= 650.0 {
            2
        } else {
            1
        };
        let spacing = 8.0;
        let total_spacing = spacing * (columns.saturating_sub(1) as f32);
        let card_width = ((available_width - total_spacing) / columns as f32).max(1.0);

        egui::Grid::new("dashboard_cards")
            .num_columns(columns)
            .spacing(Vec2::new(spacing, spacing))
            .show(ui, |ui| {
                for (index, card) in self.cards.iter().enumerate() {
                    draw_card(ui, card, card_width);
                    if (index + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
            });
    }

    fn draw_output(&mut self, ui: &mut egui::Ui) {
        let width = ui.available_width().max(1.0);
        ui.allocate_ui_with_layout(
            Vec2::new(width, 292.0),
            Layout::top_down(Align::Min),
            |ui| {
                Frame::new()
                    .fill(Color32::from_rgb(10, 15, 25))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(34, 48, 69)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        let inner_width = (width - 22.0).max(1.0);
                        ui.set_width(inner_width);
                        ui.label(
                            RichText::new("Latest Output")
                                .font(FontId::proportional(17.0))
                                .strong(),
                        );
                        let text = self
                            .latest_report_path
                            .as_deref()
                            .unwrap_or("No report generated in this GUI session.");
                        ui.add(
                            egui::Label::new(
                                RichText::new(text)
                                    .font(FontId::proportional(11.0))
                                    .color(Color32::from_rgb(132, 151, 178)),
                            )
                            .truncate(),
                        );
                        ui.add_space(6.0);
                        egui::ScrollArea::vertical()
                            .id_salt("output_scroll_area")
                            .auto_shrink([false, false])
                            .max_height(245.0)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.add(
                                    TextEdit::multiline(&mut self.latest_output)
                                        .id_salt("latest_output_text")
                                        .font(egui::TextStyle::Monospace)
                                        .desired_rows(13)
                                        .desired_width(ui.available_width())
                                        .interactive(false),
                                );
                            });
                    });
            },
        );
    }
}

impl eframe::App for LocalAccessGuardApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_worker(ui.ctx());

        egui::Panel::top("top_header_panel")
            .exact_size(54.0)
            .frame(Frame::new().fill(Color32::from_rgb(7, 10, 17)))
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                self.draw_top_bar(ui);
            });

        egui::Panel::left("left_sidebar_panel")
            .exact_size(220.0)
            .resizable(false)
            .frame(Frame::new().fill(Color32::from_rgb(7, 10, 17)))
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                self.draw_sidebar(ui);
            });

        egui::CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(Color32::from_rgb(7, 10, 17))
                    .inner_margin(Margin::same(8)),
            )
            .show_inside(ui, |ui| {
                ui.set_width(ui.available_width());
                match self.screen {
                    Screen::Dashboard => self.draw_dashboard(ui),
                    Screen::Reports => self.draw_reports(ui),
                    Screen::Advanced => self.draw_advanced(ui),
                    Screen::About => self.draw_about(ui),
                }
            });
    }
}

fn action_group(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.label(
        RichText::new(title)
            .font(FontId::proportional(12.0))
            .strong()
            .color(Color32::from_rgb(91, 171, 255)),
    );
    add_contents(ui);
    ui.add_space(6.0);
}

fn action_button(ui: &mut egui::Ui, label: &str, disabled: bool) -> egui::Response {
    ui.add_enabled(
        !disabled,
        Button::new(RichText::new(label).color(Color32::from_rgb(229, 236, 247)))
            .min_size(Vec2::new(ui.available_width().min(180.0), 28.0))
            .fill(Color32::from_rgb(20, 30, 45))
            .stroke(Stroke::new(1.0, Color32::from_rgb(45, 64, 91)))
            .corner_radius(CornerRadius::same(6)),
    )
}

fn nav_button(ui: &mut egui::Ui, label: &str, screen: &mut Screen, target: Screen) {
    let selected = *screen == target;
    let fill = if selected {
        Color32::from_rgb(28, 57, 88)
    } else {
        Color32::from_rgb(20, 30, 45)
    };

    if ui
        .add(
            Button::new(RichText::new(label).color(Color32::from_rgb(229, 236, 247)))
                .min_size(Vec2::new(ui.available_width().min(180.0), 28.0))
                .fill(fill)
                .stroke(Stroke::new(1.0, Color32::from_rgb(45, 64, 91)))
                .corner_radius(CornerRadius::same(6)),
        )
        .clicked()
    {
        *screen = target;
    }
}

fn panel_frame(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width().max(1.0);
    ui.allocate_ui_with_layout(Vec2::new(width, 1.0), Layout::top_down(Align::Min), |ui| {
        Frame::new()
            .fill(Color32::from_rgb(10, 15, 25))
            .stroke(Stroke::new(1.0, Color32::from_rgb(34, 48, 69)))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                ui.set_width((width - 22.0).max(1.0));
                add_contents(ui);
            });
    });
}

fn panel_list(ui: &mut egui::Ui, title: &str, items: &[String]) {
    panel_frame(ui, |ui| {
        ui.label(
            RichText::new(title)
                .font(FontId::proportional(16.0))
                .strong(),
        );
        ui.add_space(4.0);
        for item in items {
            ui.label(format!("- {}", item));
        }
    });
}

fn draw_card(ui: &mut egui::Ui, card: &DashboardCard, width: f32) {
    ui.allocate_ui_with_layout(Vec2::new(width, 86.0), Layout::top_down(Align::Min), |ui| {
        Frame::new()
            .fill(Color32::from_rgb(13, 21, 34))
            .stroke(Stroke::new(1.0, Color32::from_rgb(39, 56, 80)))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(10))
            .show(ui, |ui| {
                let inner_width = (width - 22.0).max(1.0);
                ui.set_min_size(Vec2::new(inner_width, 64.0));
                ui.set_max_width(inner_width);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(card.title)
                            .font(FontId::proportional(16.0))
                            .strong(),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        status_pill(ui, "", card.status);
                    });
                });
                ui.add_space(7.0);
                ui.label(
                    RichText::new(&card.detail)
                        .font(FontId::proportional(12.0))
                        .color(Color32::from_rgb(166, 181, 203)),
                );
            });
    });
}

fn status_pill(ui: &mut egui::Ui, prefix: &str, status: StatusLevel) {
    let label = if prefix.is_empty() {
        status.label().to_string()
    } else {
        format!("{}: {}", prefix, status.label())
    };

    Frame::new()
        .fill(status.fill())
        .stroke(Stroke::new(1.0, status.color()))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(label)
                    .font(FontId::proportional(12.0))
                    .strong()
                    .color(status.color()),
            );
        });
}

fn run_command(label: &str, args: &[String]) -> CommandResult {
    let exe = match env::current_exe() {
        Ok(path) => path,
        Err(err) => {
            return CommandResult {
                label: label.to_string(),
                output: format!("[FAILED] Could not resolve current executable: {}", err),
                success: false,
            };
        }
    };

    match Command::new(exe).args(args).output() {
        Ok(output) => {
            let mut text = String::new();
            text.push_str(&String::from_utf8_lossy(&output.stdout));

            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.trim().is_empty() {
                if !text.ends_with('\n') {
                    text.push('\n');
                }
                text.push_str("[stderr]\n");
                text.push_str(&stderr);
            }

            CommandResult {
                label: label.to_string(),
                output: text,
                success: output.status.success(),
            }
        }
        Err(err) => CommandResult {
            label: label.to_string(),
            output: format!("[FAILED] Could not run command {:?}: {}", args, err),
            success: false,
        },
    }
}

fn append_result(buffer: &mut String, result: &CommandResult) {
    buffer.push_str("=== ");
    buffer.push_str(&result.label);
    buffer.push_str(" ===\n");
    buffer.push_str(&result.output);
    if !result.output.ends_with('\n') {
        buffer.push('\n');
    }
    buffer.push('\n');
}

fn contains_warning_signal(text: &str) -> bool {
    text.contains("WARNING")
        || text.contains("Warning:")
        || text.contains("Action: Block")
        || text.contains("Cloudflare WARP service is running")
}

fn extract_report_path(output: &str) -> Option<String> {
    let mut previous_was_saved = false;

    for line in output.lines() {
        let trimmed = line.trim();

        if previous_was_saved && !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }

        previous_was_saved = trimmed == "[OK] Report saved:" || trimmed.ends_with("Report saved:");
    }

    None
}
