//! Single egui pipeline for product chrome and optional debug shell.

use game_net::{character_catalog, NetRole, RosterEntry, DEFAULT_CHARACTER};
use game_sim::{ActiveWeapon, WeaponClass};

use crate::mp::{self, MpPhase, ProductSurfaceKind, StagedLoadout, JOIN_ROOM_PREFILL};

#[cfg(feature = "debug-tools")]
use crate::debug::DebugShell;

mod theme {
    use egui::Color32;

    pub const GATE_FIELD: Color32 = Color32::from_rgb(235, 244, 255);
    pub const PANEL_CARD: Color32 = Color32::from_rgb(255, 255, 252);
    pub const PANEL_SCRIM: Color32 = Color32::from_rgba_premultiplied(10, 15, 23, 100);
    pub const SKY_PRIMARY: Color32 = Color32::from_rgb(52, 148, 230);
    pub const LIME_SECONDARY: Color32 = Color32::from_rgb(118, 205, 88);
    pub const CORAL_DANGER: Color32 = Color32::from_rgb(255, 108, 88);
    pub const TEXT_DARK: Color32 = Color32::from_rgb(28, 38, 54);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(88, 102, 120);
    pub const TEXT_ON_FILL: Color32 = Color32::WHITE;
    pub const CHROME_BG: Color32 = Color32::from_rgba_premultiplied(255, 255, 252, 215);
    pub const CHROME_STROKE: Color32 = Color32::from_rgba_premultiplied(52, 148, 230, 80);
    pub const NAME_ALLY: Color32 = Color32::from_rgb(52, 148, 230);
    pub const NAME_OPPONENT: Color32 = Color32::from_rgb(255, 108, 88);
    pub const NAME_OUTLINE: Color32 = Color32::from_rgb(255, 255, 252);
}

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
    pub stage_primary: Option<Option<u8>>,
    pub stage_secondary: Option<Option<u8>>,
    pub stage_active: Option<ActiveWeapon>,
    pub spawn: bool,
    pub leave: bool,
}

#[derive(Debug, Clone)]
pub struct FloatingNameLabel {
    pub pos: egui::Pos2,
    pub name: String,
    pub ally: bool,
    pub font_size: f32,
}

/// Per-frame product inputs for the overlay (phase, roster, loadout bench).
pub struct ProductSession<'a> {
    pub phase: MpPhase,
    pub roster: &'a [RosterEntry],
    pub connecting: bool,
    pub character: u8,
    pub staged: StagedLoadout,
    pub floating_names: &'a [FloatingNameLabel],
}

pub struct UiOverlay {
    egui_ctx: egui::Context,
    egui_renderer: egui_wgpu::Renderer,
    pending_events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
    room_code: String,
    display_name: String,
    status: String,
    pick_character: u8,
}

impl UiOverlay {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let egui_renderer = egui_wgpu::Renderer::new(device, surface_format, None, 1, false);
        let egui_ctx = egui::Context::default();
        apply_product_theme(&egui_ctx);
        let name = mp::load_display_name_cookie().unwrap_or_default();
        Self {
            egui_ctx,
            egui_renderer,
            pending_events: Vec::new(),
            modifiers: egui::Modifiers::default(),
            room_code: JOIN_ROOM_PREFILL.into(),
            display_name: name,
            status: String::new(),
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

    pub fn wants_ui_input(&self, phase: MpPhase) -> bool {
        phase
            .surface_kind()
            .is_some_and(ProductSurfaceKind::arms_soft_pointer)
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
        session: ProductSession<'_>,
        soft_cursor: Option<egui::Pos2>,
        mut debug: DebugDraw<'_>,
    ) -> (Option<egui::FullOutput>, ProductActions) {
        self.egui_ctx.set_pixels_per_point(pixels_per_point);

        let phase = session.phase;
        let show_score = phase.in_room();
        let has_names = !session.floating_names.is_empty();
        let mut actions = ProductActions::default();
        let status = self.status.clone();

        let full = self.egui_ctx.run(raw_input, |ctx| {
            if show_score {
                draw_score(ctx, session.roster, phase, &mut actions);
            }
            if has_names {
                draw_floating_names(ctx, session.floating_names);
            }

            match phase {
                MpPhase::Lobby | MpPhase::Connecting => {
                    draw_join(
                        ctx,
                        &mut self.room_code,
                        &mut self.display_name,
                        session.connecting,
                        &status,
                        &mut actions,
                    );
                }
                MpPhase::Role => draw_role(ctx, &mut actions),
                MpPhase::Character => {
                    draw_character(ctx, &mut self.pick_character, &mut actions);
                }
                MpPhase::Ready => {
                    draw_loadout(ctx, session.character, session.staged, &mut actions)
                }
                MpPhase::Spectating => draw_spectate_chrome(ctx, &mut actions),
                MpPhase::Living => {}
            }

            debug.draw(ctx);

            if let Some(pos) = soft_cursor {
                draw_soft_cursor(ctx, pos);
            }
        });

        let any = phase != MpPhase::Living
            || show_score
            || debug.active()
            || has_names
            || soft_cursor.is_some();
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

fn apply_product_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.override_text_color = Some(theme::TEXT_DARK);
    visuals.widgets.noninteractive.fg_stroke.color = theme::TEXT_DARK;
    visuals.widgets.inactive.fg_stroke.color = theme::TEXT_ON_FILL;
    visuals.widgets.inactive.bg_fill = theme::SKY_PRIMARY;
    visuals.widgets.hovered.fg_stroke.color = theme::TEXT_ON_FILL;
    visuals.widgets.hovered.bg_fill = theme::SKY_PRIMARY.gamma_multiply(0.92);
    visuals.widgets.active.fg_stroke.color = theme::TEXT_ON_FILL;
    visuals.widgets.active.bg_fill = theme::SKY_PRIMARY.gamma_multiply(0.85);
    visuals.selection.bg_fill = theme::LIME_SECONDARY.gamma_multiply(0.35);
    visuals.selection.stroke.color = theme::SKY_PRIMARY;
    visuals.window_fill = theme::PANEL_CARD;
    visuals.panel_fill = theme::GATE_FIELD;
    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.window_margin = egui::Margin::same(12);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        let body = egui::FontId::new(16.0, egui::FontFamily::Proportional);
        let button = egui::FontId::new(17.0, egui::FontFamily::Proportional);
        let heading = egui::FontId::new(24.0, egui::FontFamily::Proportional);
        let mono = egui::FontId::new(14.0, egui::FontFamily::Monospace);
        style.text_styles.insert(egui::TextStyle::Body, body);
        style.text_styles.insert(egui::TextStyle::Button, button);
        style.text_styles.insert(egui::TextStyle::Heading, heading);
        style.text_styles.insert(egui::TextStyle::Monospace, mono);
    });
}

fn primary_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(
        egui::RichText::new(text)
            .color(theme::TEXT_ON_FILL)
            .strong(),
    )
    .fill(theme::SKY_PRIMARY)
    .min_size(egui::vec2(180.0, 44.0))
}

fn secondary_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(text).color(theme::TEXT_DARK).strong())
        .fill(theme::LIME_SECONDARY)
        .min_size(egui::vec2(160.0, 40.0))
}

fn chrome_frame() -> egui::Frame {
    egui::Frame::NONE
        .fill(theme::CHROME_BG)
        .stroke(egui::Stroke::new(1.5_f32, theme::CHROME_STROKE))
        .inner_margin(egui::Margin::same(10))
        .corner_radius(8.0)
}

fn gate_narrow_shell(ctx: &egui::Context, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(theme::GATE_FIELD))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.14);
                ui.allocate_ui_with_layout(
                    egui::vec2(380.0, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.set_max_width(380.0);
                        ui.heading(title);
                        ui.add_space(20.0);
                        add(ui);
                    },
                );
            });
        });
}

fn gate_field_shell(ctx: &egui::Context, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::CentralPanel::default()
        .frame(
            egui::Frame::NONE
                .fill(theme::GATE_FIELD)
                .inner_margin(egui::Margin::symmetric(32, 28)),
        )
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(title);
                ui.add_space(16.0);
                add(ui);
            });
        });
}

fn panel_card_shell(ctx: &egui::Context, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(theme::PANEL_SCRIM))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let max_w = 720.0_f32.min(ui.available_width() * 0.92);
                let card_h = ui.available_height() * 0.82;
                egui::Frame::NONE
                    .fill(theme::PANEL_CARD)
                    .stroke(egui::Stroke::new(2.0_f32, theme::CHROME_STROKE))
                    .inner_margin(egui::Margin::symmetric(28, 24))
                    .corner_radius(12.0)
                    .show(ui, |ui| {
                        ui.set_max_width(max_w);
                        ui.set_min_width(max_w.min(480.0));
                        ui.heading(title);
                        ui.add_space(12.0);
                        egui::ScrollArea::vertical()
                            .max_height(card_h - 56.0)
                            .show(ui, |ui| {
                                add(ui);
                            });
                    });
            });
        });
}

fn draw_soft_cursor(ctx: &egui::Context, pos: egui::Pos2) {
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("soft_pointer"),
    ));
    let fill = egui::Color32::from_rgb(255, 255, 252);
    let outline = theme::TEXT_DARK;
    let tip = pos;
    let base_l = pos + egui::vec2(0.0, 16.0);
    let base_r = pos + egui::vec2(11.0, 11.0);
    let notch = pos + egui::vec2(4.0, 11.0);
    let points = vec![tip, base_l, notch, base_r];
    painter.add(egui::Shape::convex_polygon(
        points.clone(),
        fill,
        egui::Stroke::NONE,
    ));
    painter.add(egui::Shape::closed_line(
        points,
        egui::Stroke::new(1.5_f32, outline),
    ));
}

fn draw_floating_names(ctx: &egui::Context, labels: &[FloatingNameLabel]) {
    const OUTLINE_OFFSETS: [(f32, f32); 8] = [
        (-1.0, 0.0),
        (1.0, 0.0),
        (0.0, -1.0),
        (0.0, 1.0),
        (-1.0, -1.0),
        (1.0, -1.0),
        (-1.0, 1.0),
        (1.0, 1.0),
    ];
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("floating_names"),
    ));
    let screen = ctx.screen_rect();
    for label in labels {
        if !screen.contains(label.pos) {
            continue;
        }
        let fill = if label.ally {
            theme::NAME_ALLY
        } else {
            theme::NAME_OPPONENT
        };
        let font = egui::FontId::new(label.font_size, egui::FontFamily::Proportional);
        for (ox, oy) in OUTLINE_OFFSETS {
            painter.text(
                label.pos + egui::vec2(ox, oy),
                egui::Align2::CENTER_BOTTOM,
                &label.name,
                font.clone(),
                theme::NAME_OUTLINE,
            );
        }
        painter.text(
            label.pos,
            egui::Align2::CENTER_BOTTOM,
            &label.name,
            font,
            fill,
        );
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
            chrome_frame().show(ui, |ui| {
                ui.label(
                    egui::RichText::new("FFA")
                        .strong()
                        .color(theme::SKY_PRIMARY),
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
                    if ui
                        .add(
                            egui::Button::new("Spectate")
                                .fill(theme::LIME_SECONDARY)
                                .min_size(egui::vec2(80.0, 28.0)),
                        )
                        .clicked()
                    {
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
    gate_narrow_shell(ctx, "Join room", |ui| {
        egui::Grid::new("join_grid")
            .num_columns(2)
            .spacing([12.0, 12.0])
            .show(ui, |ui| {
                ui.label("Room");
                ui.add(
                    egui::TextEdit::singleline(room)
                        .desired_width(220.0)
                        .hint_text("room code"),
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
            primary_button(if connecting { "Joining…" } else { "Join" }),
        );
        if join_btn.clicked() {
            actions.join = Some((room.clone(), name.clone()));
        }
        if !status.is_empty() {
            ui.add_space(12.0);
            ui.colored_label(theme::CORAL_DANGER, status);
        }
    });
}

fn draw_role(ctx: &egui::Context, actions: &mut ProductActions) {
    gate_narrow_shell(ctx, "Choose role", |ui| {
        ui.label(egui::RichText::new("Free-for-all · empty map").color(theme::TEXT_MUTED));
        ui.add_space(20.0);
        if ui.add(primary_button("Play")).clicked() {
            actions.play = true;
        }
        ui.add_space(12.0);
        if ui.add(secondary_button("Spectate")).clicked() {
            actions.spectate = true;
        }
        ui.add_space(24.0);
        if ui.button("Leave").clicked() {
            actions.leave = true;
        }
    });
}

fn draw_character(ctx: &egui::Context, pick: &mut u8, actions: &mut ProductActions) {
    gate_field_shell(ctx, "Character", |ui| {
        ui.label(
            egui::RichText::new("Pick a body kit (shared kits allowed)").color(theme::TEXT_MUTED),
        );
        ui.add_space(16.0);
        egui::Frame::NONE
            .fill(theme::PANEL_CARD)
            .stroke(egui::Stroke::new(2.5_f32, theme::SKY_PRIMARY))
            .inner_margin(egui::Margin::symmetric(56, 36))
            .corner_radius(10.0)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{}", *pick as char))
                            .size(88.0)
                            .strong()
                            .color(theme::SKY_PRIMARY),
                    );
                });
            });
        ui.add_space(16.0);
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
        ui.add_space(16.0);
        if ui.add(primary_button("Confirm")).clicked() {
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

fn draw_loadout(
    ctx: &egui::Context,
    character: u8,
    staged: StagedLoadout,
    actions: &mut ProductActions,
) {
    panel_card_shell(ctx, "Loadout", |ui| {
        ui.label(format!(
            "Character {} · pick primary / secondary / hand, then Spawn",
            character as char
        ));
        ui.add_space(12.0);

        ui.label(egui::RichText::new("Primary").strong());
        ui.horizontal_wrapped(|ui| {
            let empty_sel = staged.primary.is_none();
            if ui.selectable_label(empty_sel, "  empty  ").clicked() {
                actions.stage_primary = Some(None);
            }
            for letter in b'a'..=b'r' {
                let sel = staged.primary == Some(letter);
                let label = format!("  {}  ", letter as char);
                if ui.selectable_label(sel, label).clicked() {
                    actions.stage_primary = Some(Some(letter));
                }
            }
        });

        ui.add_space(10.0);
        ui.label(egui::RichText::new("Secondary (launcher / pistol)").strong());
        ui.horizontal_wrapped(|ui| {
            let empty_sel = staged.secondary.is_none();
            if ui.selectable_label(empty_sel, "  empty  ").clicked() {
                actions.stage_secondary = Some(None);
            }
            for letter in b'a'..=b'r' {
                let Some(class) = WeaponClass::from_letter(letter) else {
                    continue;
                };
                if !class.allowed_in_secondary() {
                    continue;
                }
                let sel = staged.secondary == Some(letter);
                let label = format!("  {}  ", letter as char);
                if ui.selectable_label(sel, label).clicked() {
                    actions.stage_secondary = Some(Some(letter));
                }
            }
        });

        ui.add_space(10.0);
        ui.label(egui::RichText::new("Active hand").strong());
        ui.horizontal(|ui| {
            let p = staged.active == ActiveWeapon::Primary;
            let s = staged.active == ActiveWeapon::Secondary;
            if ui.selectable_label(p, "  Primary  ").clicked() {
                actions.stage_active = Some(ActiveWeapon::Primary);
            }
            if ui.selectable_label(s, "  Secondary  ").clicked() {
                actions.stage_active = Some(ActiveWeapon::Secondary);
            }
        });

        ui.add_space(8.0);
        let summary = format!(
            "Staged: primary {} · secondary {} · hand {}",
            slot_label(staged.primary),
            slot_label(staged.secondary),
            match staged.active {
                ActiveWeapon::Primary => "primary",
                ActiveWeapon::Secondary => "secondary",
            }
        );
        ui.label(summary);

        ui.add_space(16.0);
        if ui.add(primary_button("Spawn")).clicked() {
            actions.spawn = true;
        }
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            if ui.button("Spectate").clicked() {
                actions.spectate = true;
            }
            if ui.button("Leave").clicked() {
                actions.leave = true;
            }
        });
    });
}

fn slot_label(letter: Option<u8>) -> String {
    letter
        .map(|c| (c as char).to_string())
        .unwrap_or_else(|| "empty".into())
}

fn draw_spectate_chrome(ctx: &egui::Context, actions: &mut ProductActions) {
    egui::Area::new(egui::Id::new("spectate_chrome"))
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(12.0, -12.0))
        .show(ctx, |ui| {
            chrome_frame().show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Spectating · WASD fly · click to look")
                        .color(theme::TEXT_MUTED),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.add(primary_button("Play")).clicked() {
                        actions.play = true;
                    }
                    if ui.button("Leave").clicked() {
                        actions.leave = true;
                    }
                });
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
