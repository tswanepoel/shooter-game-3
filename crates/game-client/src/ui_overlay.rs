//! Single egui pipeline for product chrome and optional debug shell.

use game_net::RosterEntry;

use crate::mp::{self, MpPhase, DEFAULT_ROOM_CODE};

#[cfg(feature = "debug-tools")]
use crate::debug::DebugShell;

pub struct OverlayGpu<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub view: &'a wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Default, Clone)]
pub struct ProductActions {
    pub join: Option<(String, String)>,
    pub spawn: bool,
    pub leave: bool,
}

pub struct UiOverlay {
    egui_ctx: egui::Context,
    egui_renderer: egui_wgpu::Renderer,
    pending_events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
    room_code: String,
    display_name: String,
    status: String,
    ui_wants_pointer: bool,
}

impl UiOverlay {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let egui_renderer = egui_wgpu::Renderer::new(device, surface_format, None, 1, false);
        let egui_ctx = egui::Context::default();
        egui_ctx.set_visuals(egui::Visuals::dark());
        egui_ctx.style_mut(|style| {
            style.spacing.item_spacing = egui::vec2(4.0, 2.0);
            style.spacing.window_margin = egui::Margin::same(6);
            let mono = egui::FontId::new(12.0, egui::FontFamily::Monospace);
            let body = egui::FontId::new(12.0, egui::FontFamily::Proportional);
            style.text_styles.insert(egui::TextStyle::Body, body);
            style
                .text_styles
                .insert(egui::TextStyle::Monospace, mono.clone());
            style
                .text_styles
                .insert(egui::TextStyle::Button, mono.clone());
        });
        let name = mp::load_display_name_cookie().unwrap_or_default();
        Self {
            egui_ctx,
            egui_renderer,
            pending_events: Vec::new(),
            modifiers: egui::Modifiers::default(),
            room_code: DEFAULT_ROOM_CODE.into(),
            display_name: name,
            status: String::new(),
            ui_wants_pointer: false,
        }
    }

    pub fn push_event(&mut self, event: egui::Event) {
        self.pending_events.push(event);
    }

    pub fn set_modifiers(&mut self, modifiers: egui::Modifiers) {
        self.modifiers = modifiers;
    }

    pub fn modifiers(&self) -> egui::Modifiers {
        self.modifiers
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
    }

    pub fn wants_ui_input(&self, phase: MpPhase) -> bool {
        matches!(phase, MpPhase::Solo | MpPhase::Connecting | MpPhase::Joined)
    }

    pub fn blocks_pointer_lock(&self, phase: MpPhase) -> bool {
        phase.forces_free_cursor() || self.ui_wants_pointer
    }

    pub fn take_raw_input(&mut self, screen_w: f32, screen_h: f32, time: f64) -> egui::RawInput {
        let events = std::mem::take(&mut self.pending_events);
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(screen_w, screen_h),
            )),
            time: Some(time),
            modifiers: self.modifiers,
            events,
            ..Default::default()
        }
    }

    pub fn run(
        &mut self,
        raw_input: egui::RawInput,
        pixels_per_point: f32,
        phase: MpPhase,
        roster: &[RosterEntry],
        connecting: bool,
        mut debug: DebugDraw<'_>,
    ) -> (Option<egui::FullOutput>, ProductActions) {
        self.egui_ctx.set_pixels_per_point(pixels_per_point);

        let show_join = matches!(phase, MpPhase::Solo | MpPhase::Connecting);
        let show_spawn = phase == MpPhase::Joined;
        let show_score = phase.in_room();

        let room = &mut self.room_code;
        let name = &mut self.display_name;
        let status = self.status.clone();
        let mut actions = ProductActions::default();

        let full = self.egui_ctx.run(raw_input, |ctx| {
            if show_score && !roster.is_empty() {
                egui::Area::new(egui::Id::new("score_roster"))
                    .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 12.0))
                    .show(ctx, |ui| {
                        egui::Frame::NONE
                            .fill(egui::Color32::from_rgba_unmultiplied(8, 10, 14, 200))
                            .inner_margin(egui::Margin::same(8))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("FFA")
                                        .strong()
                                        .color(egui::Color32::from_rgb(180, 220, 160)),
                                );
                                for e in roster {
                                    let mark = if e.living { "●" } else { "○" };
                                    ui.label(format!("{mark} {}  {}", e.display_name, e.score));
                                }
                            });
                    });
            }

            if show_join {
                egui::Window::new("Join room")
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        egui::Grid::new("join_grid")
                            .num_columns(2)
                            .spacing([8.0, 8.0])
                            .show(ui, |ui| {
                                ui.label("Room");
                                ui.add(
                                    egui::TextEdit::singleline(room)
                                        .desired_width(180.0)
                                        .hint_text(DEFAULT_ROOM_CODE),
                                );
                                ui.end_row();
                                ui.label("Name");
                                ui.add(
                                    egui::TextEdit::singleline(name)
                                        .desired_width(180.0)
                                        .hint_text("display name"),
                                );
                                ui.end_row();
                            });
                        ui.add_space(10.0);
                        let join_btn = ui.add_enabled(
                            !connecting,
                            egui::Button::new(if connecting { "Joining…" } else { "Join" }),
                        );
                        if join_btn.clicked() {
                            actions.join = Some((room.clone(), name.clone()));
                        }
                        if !status.is_empty() {
                            ui.add_space(8.0);
                            ui.colored_label(egui::Color32::from_rgb(220, 140, 120), &status);
                        }
                        ui.add_space(4.0);
                        ui.small("Type freely. Empty canvas locks for solo (Esc unlocks).");
                    });
            }

            if show_spawn {
                egui::Window::new("Match")
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Free-for-all · empty map");
                        ui.add_space(12.0);
                        if ui
                            .button(egui::RichText::new("  Spawn  ").size(18.0))
                            .clicked()
                        {
                            actions.spawn = true;
                        }
                        ui.add_space(8.0);
                        if ui.button("Leave").clicked() {
                            actions.leave = true;
                        }
                    });
            }

            debug.draw(ctx);
        });

        self.ui_wants_pointer = self.egui_ctx.wants_pointer_input()
            || self.egui_ctx.is_pointer_over_area()
            || self.egui_ctx.wants_keyboard_input();

        let any = show_join || show_spawn || show_score || debug.active();
        if any {
            (Some(full), actions)
        } else {
            (None, actions)
        }
    }

    pub fn render(&mut self, gpu: OverlayGpu<'_>, full: egui::FullOutput) {
        let pixels_per_point = full.pixels_per_point;
        for (id, delta) in &full.textures_delta.set {
            self.egui_renderer
                .update_texture(gpu.device, gpu.queue, *id, delta);
        }
        for id in &full.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        let tris = self.egui_ctx.tessellate(full.shapes, pixels_per_point);
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [gpu.width, gpu.height],
            pixels_per_point,
        };

        self.egui_renderer.update_buffers(
            gpu.device,
            gpu.queue,
            gpu.encoder,
            &tris,
            &screen_descriptor,
        );

        {
            let pass = gpu.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui-overlay-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: gpu.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.egui_renderer
                .render(&mut pass.forget_lifetime(), &tris, &screen_descriptor);
        }
    }
}

pub enum DebugDraw<'a> {
    /// No debug chrome (production, or debug build with nothing to draw).
    None(std::marker::PhantomData<&'a ()>),
    #[cfg(feature = "debug-tools")]
    Shell {
        shell: &'a mut DebugShell,
        hud: Option<&'a str>,
    },
}

impl DebugDraw<'_> {
    pub fn none() -> Self {
        Self::None(std::marker::PhantomData)
    }

    fn active(&self) -> bool {
        match self {
            Self::None(_) => false,
            #[cfg(feature = "debug-tools")]
            Self::Shell { shell, hud } => shell.wants_draw(*hud),
        }
    }

    fn draw(&mut self, ctx: &egui::Context) {
        #[cfg(feature = "debug-tools")]
        if let Self::Shell { shell, hud } = self {
            shell.ui(ctx, *hud);
        }
        #[cfg(not(feature = "debug-tools"))]
        {
            let _ = (self, ctx);
        }
    }
}
