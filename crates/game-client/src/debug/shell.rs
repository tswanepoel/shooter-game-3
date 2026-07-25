//! Debug console widgets (drawn into the shared ui overlay).

const MAX_LOG_LINES: usize = 200;

pub struct DebugShell {
    pub open: bool,
    pub focus_input: bool,
    input: String,
    log: Vec<String>,
    history: Vec<String>,
    history_cursor: Option<usize>,
    pending_command: Option<String>,
}

impl DebugShell {
    pub fn new() -> Self {
        Self {
            open: false,
            focus_input: false,
            input: String::new(),
            log: Vec::new(),
            history: Vec::new(),
            history_cursor: None,
            pending_command: None,
        }
    }

    pub fn take_pending_command(&mut self) -> Option<String> {
        self.pending_command.take()
    }

    pub fn push_log(&mut self, line: impl Into<String>) {
        for part in line.into().split('\n') {
            self.log.push(part.to_string());
        }
        if self.log.len() > MAX_LOG_LINES {
            let drain = self.log.len() - MAX_LOG_LINES;
            self.log.drain(0..drain);
        }
    }

    pub fn wants_draw(&self, hud_line: Option<&str>) -> bool {
        self.open || hud_line.is_some()
    }

    /// Draw console + optional HUD into the shared egui context.
    pub fn ui(&mut self, ctx: &egui::Context, hud_line: Option<&str>) {
        if !self.wants_draw(hud_line) {
            return;
        }

        let mut run_line: Option<String> = None;
        let mut history_delta: i32 = 0;
        let mut close = false;

        let console_open = self.open;
        let focus_input = self.focus_input;
        let log = self.log.clone();
        let mut input = std::mem::take(&mut self.input);
        let hud = hud_line.map(|s| s.to_string());

        if console_open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close = true;
        }

        if let Some(ref line) = hud {
            egui::TopBottomPanel::top("net_hud")
                .exact_height(22.0)
                .frame(
                    egui::Frame::NONE
                        .fill(egui::Color32::from_rgba_unmultiplied(8, 10, 14, 200))
                        .inner_margin(egui::Margin::symmetric(8, 3)),
                )
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new(line)
                            .monospace()
                            .size(12.0)
                            .color(egui::Color32::from_rgb(180, 220, 160)),
                    );
                });
        }

        if console_open {
            let screen = ctx.screen_rect();
            let panel_h = (screen.height() * 0.32).clamp(120.0, 280.0);

            egui::TopBottomPanel::bottom("debug_console")
                .exact_height(panel_h)
                .frame(
                    egui::Frame::NONE
                        .fill(egui::Color32::from_rgba_unmultiplied(12, 14, 18, 230))
                        .inner_margin(egui::Margin::same(8))
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            egui::Color32::from_rgb(60, 70, 80),
                        )),
                )
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .auto_shrink([false, false])
                        .max_height(ui.available_height() - 26.0)
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            for line in &log {
                                ui.label(
                                    egui::RichText::new(line)
                                        .monospace()
                                        .color(egui::Color32::from_rgb(210, 215, 220)),
                                );
                            }
                        });

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(">").monospace());
                        let te = egui::TextEdit::singleline(&mut input)
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace)
                            .lock_focus(true);
                        let response = ui.add(te);
                        if focus_input {
                            response.request_focus();
                        }

                        if response.has_focus() {
                            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                                history_delta = -1;
                            }
                            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                                history_delta = 1;
                            }
                        }

                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !input.trim().is_empty()
                        {
                            run_line = Some(input.clone());
                        }
                    });
                });
        }

        self.focus_input = false;
        self.input = input;

        if close {
            self.open = false;
        }

        if history_delta != 0 {
            self.nudge_history(history_delta);
        }

        if let Some(line) = run_line {
            let trimmed = line.trim().to_string();
            self.push_log(format!("> {trimmed}"));
            self.history.push(trimmed.clone());
            self.history_cursor = None;
            self.input.clear();
            self.pending_command = Some(trimmed);
            self.focus_input = true;
        }
    }

    fn nudge_history(&mut self, delta: i32) {
        if self.history.is_empty() {
            return;
        }
        let len = self.history.len();
        let idx = match self.history_cursor {
            None if delta < 0 => len - 1,
            None => return,
            Some(i) => {
                let n = i as i32 + delta;
                if n < 0 {
                    self.history_cursor = None;
                    self.input.clear();
                    return;
                }
                if n as usize >= len {
                    return;
                }
                n as usize
            }
        };
        self.history_cursor = Some(idx);
        self.input = self.history[idx].clone();
    }
}

impl Default for DebugShell {
    fn default() -> Self {
        Self::new()
    }
}
