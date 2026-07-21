//! egui console shell — thin transport over the registry.

const MAX_LOG_LINES: usize = 200;
const CONSOLE_FONT_SIZE: f32 = 12.0;

pub struct DebugShell {
    pub open: bool,
    pub focus_input: bool,
    input: String,
    log: Vec<String>,
    history: Vec<String>,
    history_cursor: Option<usize>,
    /// Console line submitted this frame (run by ClientInner after egui).
    pending_command: Option<String>,
    egui_ctx: egui::Context,
    egui_renderer: egui_wgpu::Renderer,
}

impl DebugShell {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let egui_renderer = egui_wgpu::Renderer::new(device, surface_format, None, 1, false);
        let egui_ctx = egui::Context::default();
        egui_ctx.set_visuals(egui::Visuals::dark());
        egui_ctx.style_mut(|style| {
            style.spacing.item_spacing = egui::vec2(4.0, 2.0);
            style.spacing.window_margin = egui::Margin::same(6);
            let mono = egui::FontId::new(CONSOLE_FONT_SIZE, egui::FontFamily::Monospace);
            let body = egui::FontId::new(CONSOLE_FONT_SIZE, egui::FontFamily::Proportional);
            style.text_styles.insert(egui::TextStyle::Body, body);
            style
                .text_styles
                .insert(egui::TextStyle::Monospace, mono.clone());
            style
                .text_styles
                .insert(egui::TextStyle::Button, mono.clone());
            style.text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::new(14.0, egui::FontFamily::Proportional),
            );
        });

        Self {
            open: false,
            focus_input: false,
            input: String::new(),
            log: Vec::new(),
            history: Vec::new(),
            history_cursor: None,
            pending_command: None,
            egui_ctx,
            egui_renderer,
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

    /// Run egui for the console and optional top HUD banner.
    /// Submitted lines land in [`take_pending_command`] for the host to run.
    pub fn run_frame(
        &mut self,
        raw_input: egui::RawInput,
        pixels_per_point: f32,
        hud_line: Option<&str>,
    ) -> Option<egui::FullOutput> {
        if !self.open && hud_line.is_none() {
            return None;
        }

        self.egui_ctx.set_pixels_per_point(pixels_per_point);

        let mut run_line: Option<String> = None;
        let mut history_delta: i32 = 0;
        let mut close = false;

        let console_open = self.open;
        let focus_input = self.focus_input;
        let log = self.log.clone();
        let mut input = std::mem::take(&mut self.input);
        let hud = hud_line.map(|s| s.to_string());

        let full = self.egui_ctx.run(raw_input, |ctx| {
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

            if !console_open {
                return;
            }

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
        });

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

        // Still return output after Escape close so texture deltas are applied.
        Some(full)
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

    pub fn render_overlay(&mut self, gpu: OverlayGpu<'_>, full: egui::FullOutput) {
        let OverlayGpu {
            device,
            queue,
            encoder,
            view,
            width,
            height,
            pixels_per_point,
        } = gpu;

        let textures_delta = full.textures_delta;
        for (id, delta) in &textures_delta.set {
            self.egui_renderer.update_texture(device, queue, *id, delta);
        }
        for id in &textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        let tris = self.egui_ctx.tessellate(full.shapes, pixels_per_point);

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point,
        };

        self.egui_renderer
            .update_buffers(device, queue, encoder, &tris, &screen_descriptor);

        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            // egui-wgpu expects a 'static pass lifetime (wgpu 24).
            self.egui_renderer
                .render(&mut pass.forget_lifetime(), &tris, &screen_descriptor);
        }
    }
}

/// GPU handles for painting the egui overlay onto the swapchain.
pub struct OverlayGpu<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub view: &'a wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub pixels_per_point: f32,
}
