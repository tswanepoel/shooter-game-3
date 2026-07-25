//! Single egui pipeline for product chrome and optional debug shell.

use game_net::{character_catalog, NetRole, RosterEntry, DEFAULT_CHARACTER};

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
    pub play: bool,
    pub spectate: bool,
    pub confirm_character: Option<u8>,
    pub back_to_role: bool,
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
    pick_character: u8,
}

impl UiOverlay {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let egui_renderer = egui_wgpu::Renderer::new(device, surface_format, None, 1, false);
        let egui_ctx = egui::Context::default();
        egui_ctx.set_visuals(egui::Visuals::dark());
        egui_ctx.style_mut(|style| {
            style.spacing.item_spacing = egui::vec2(8.0, 6.0);
            style.spacing.window_margin = egui::Margin::same(12);
            let mono = egui::FontId::new(14.0, egui::FontFamily::Monospace);
            let body = egui::FontId::new(16.0, egui::FontFamily::Proportional);
            style.text_styles.insert(egui::TextStyle::Body, body);
            style
                .text_styles
                .insert(egui::TextStyle::Monospace, mono.clone());
            style
                .text_styles
                .insert(egui::TextStyle::Button, mono.clone());
            style.text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::new(22.0, egui::FontFamily::Proportional),
            );
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
            pick_character: DEFAULT_CHARACTER,
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

    pub fn sync_pick_character(&mut self, character: u8) {
        self.pick_character = character;
    }

    /// Full-frame gates take keyboard; living/spectate chrome uses pointer-over only.
    pub fn wants_ui_input(&self, phase: MpPhase) -> bool {
        phase.forces_free_cursor()
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

        let show_score = phase.in_room();
        let mut actions = ProductActions::default();
        let status = self.status.clone();
        let pick = self.pick_character;

        let full = self.egui_ctx.run(raw_input, |ctx| {
            if show_score {
                draw_score(ctx, roster, phase, &mut actions);
            }

            match phase {
                MpPhase::Lobby | MpPhase::Connecting => {
                    draw_join(
                        ctx,
                        &mut self.room_code,
                        &mut self.display_name,
                        connecting,
                        &status,
                        &mut actions,
                    );
                }
                MpPhase::Role => draw_role(ctx, &mut actions),
                MpPhase::Character => {
                    draw_character(ctx, &mut self.pick_character, &mut actions);
                }
                MpPhase::Ready => draw_ready(ctx, pick, &mut actions),
                MpPhase::Spectating => draw_spectate_chrome(ctx, &mut actions),
                MpPhase::Living => {}
            }

            debug.draw(ctx);
        });

        self.ui_wants_pointer = self.egui_ctx.wants_pointer_input()
            || self.egui_ctx.is_pointer_over_area()
            || self.egui_ctx.wants_keyboard_input();

        let any = phase != MpPhase::Living || show_score || debug.active();
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

fn draw_score(
    ctx: &egui::Context,
    roster: &[RosterEntry],
    phase: MpPhase,
    actions: &mut ProductActions,
) {
    if roster.is_empty() && phase != MpPhase::Living {
        return;
    }
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
                        let mark = if e.living {
                            "●"
                        } else if e.role == NetRole::Spectator {
                            "◎"
                        } else {
                            "○"
                        };
                        let kit = e.character as char;
                        ui.label(format!("{mark} {}  {}  [{kit}]", e.display_name, e.score));
                    }
                    if phase == MpPhase::Living {
                        ui.add_space(6.0);
                        if ui.small_button("Spectate").clicked() {
                            actions.spectate = true;
                        }
                    }
                });
        });
}

fn draw_join(
    ctx: &egui::Context,
    room: &mut String,
    name: &mut String,
    connecting: bool,
    status: &str,
    actions: &mut ProductActions,
) {
    full_frame_shell(ctx, "Join room", |ui| {
        egui::Grid::new("join_grid")
            .num_columns(2)
            .spacing([12.0, 12.0])
            .show(ui, |ui| {
                ui.label("Room");
                ui.add(
                    egui::TextEdit::singleline(room)
                        .desired_width(220.0)
                        .hint_text(DEFAULT_ROOM_CODE),
                );
                ui.end_row();
                ui.label("Name");
                ui.add(
                    egui::TextEdit::singleline(name)
                        .desired_width(220.0)
                        .hint_text("display name"),
                );
                ui.end_row();
            });
        ui.add_space(16.0);
        let join_btn = ui.add_enabled(
            !connecting,
            egui::Button::new(if connecting { "Joining…" } else { "Join" })
                .min_size(egui::vec2(160.0, 36.0)),
        );
        if join_btn.clicked() {
            actions.join = Some((room.clone(), name.clone()));
        }
        if !status.is_empty() {
            ui.add_space(12.0);
            ui.colored_label(egui::Color32::from_rgb(220, 140, 120), status);
        }
    });
}

fn draw_role(ctx: &egui::Context, actions: &mut ProductActions) {
    full_frame_shell(ctx, "Choose role", |ui| {
        ui.label("Free-for-all · empty map");
        ui.add_space(20.0);
        if ui
            .add(
                egui::Button::new(egui::RichText::new("  Play  ").size(20.0))
                    .min_size(egui::vec2(200.0, 44.0)),
            )
            .clicked()
        {
            actions.play = true;
        }
        ui.add_space(12.0);
        if ui
            .add(
                egui::Button::new(egui::RichText::new("  Spectate  ").size(18.0))
                    .min_size(egui::vec2(200.0, 40.0)),
            )
            .clicked()
        {
            actions.spectate = true;
        }
        ui.add_space(24.0);
        if ui.button("Leave").clicked() {
            actions.leave = true;
        }
    });
}

fn draw_character(ctx: &egui::Context, pick: &mut u8, actions: &mut ProductActions) {
    full_frame_shell(ctx, "Character", |ui| {
        ui.label("Pick a body kit (shared kits allowed)");
        ui.add_space(12.0);
        egui::ScrollArea::vertical()
            .max_height(280.0)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for id in character_catalog() {
                        let label = format!("  {}  ", id as char);
                        let selected = *pick == id;
                        let btn = ui.selectable_label(selected, label);
                        if btn.clicked() {
                            *pick = id;
                        }
                    }
                });
            });
        ui.add_space(16.0);
        ui.label(format!("Selected: {}", *pick as char));
        ui.add_space(12.0);
        if ui
            .add(
                egui::Button::new(egui::RichText::new("  Confirm  ").size(18.0))
                    .min_size(egui::vec2(180.0, 40.0)),
            )
            .clicked()
        {
            actions.confirm_character = Some(*pick);
        }
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                actions.back_to_role = true;
            }
            if ui.button("Spectate").clicked() {
                actions.spectate = true;
            }
            if ui.button("Leave").clicked() {
                actions.leave = true;
            }
        });
    });
}

fn draw_ready(ctx: &egui::Context, pick: u8, actions: &mut ProductActions) {
    full_frame_shell(ctx, "Ready", |ui| {
        ui.label(format!(
            "Character {} · default loadout (p / b)",
            pick as char
        ));
        ui.add_space(20.0);
        if ui
            .add(
                egui::Button::new(egui::RichText::new("  Spawn  ").size(20.0))
                    .min_size(egui::vec2(200.0, 44.0)),
            )
            .clicked()
        {
            actions.spawn = true;
        }
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            if ui.button("Change character").clicked() {
                actions.play = true;
            }
            if ui.button("Spectate").clicked() {
                actions.spectate = true;
            }
            if ui.button("Leave").clicked() {
                actions.leave = true;
            }
        });
    });
}

fn draw_spectate_chrome(ctx: &egui::Context, actions: &mut ProductActions) {
    egui::Area::new(egui::Id::new("spectate_chrome"))
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(12.0, -12.0))
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(egui::Color32::from_rgba_unmultiplied(8, 10, 14, 210))
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Spectating · WASD fly · click to look")
                            .color(egui::Color32::from_rgb(180, 200, 220)),
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("Play").clicked() {
                            actions.play = true;
                        }
                        if ui.button("Leave").clicked() {
                            actions.leave = true;
                        }
                    });
                });
        });
}

fn full_frame_shell(ctx: &egui::Context, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::CentralPanel::default()
        .frame(
            egui::Frame::NONE
                .fill(egui::Color32::from_rgba_unmultiplied(6, 8, 12, 230))
                .inner_margin(egui::Margin::symmetric(48, 36)),
        )
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.12);
                ui.heading(title);
                ui.add_space(20.0);
                add(ui);
            });
        });
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
            let _ = ctx;
        }
    }
}
