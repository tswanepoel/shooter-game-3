//! Client runtime state and frame ownership.

mod frame;
mod impact;
pub(crate) mod load;

use std::collections::HashMap;

use game_net::PlayerId;
#[cfg(feature = "debug-tools")]
use game_sim::equip_blaster_letter;
use game_sim::{FireState, MapWorld, PlayerHealth, ProjectileWorld, SelfState};
use web_sys::HtmlCanvasElement;

use crate::blaster_drop_present::BlasterDropPresent;
use crate::corpse_present::CorpsePresent;
#[cfg(feature = "debug-tools")]
use crate::debug::DebugTools;
use crate::emote_wheel::EmoteWheel;
use crate::hit_marker::HitMarker;
use crate::input::{InputSession, MoveInput, SoftPointer};
#[cfg(feature = "debug-tools")]
use crate::lineup::LineupState;
use crate::map_present::MapPresentState;
use crate::mp;
use crate::preferences::MouseSensitivity;
use crate::remote_present::RemotePresent;
use crate::renderer::Renderer;
use crate::self_present::SelfPresentState;
use crate::sfx::SfxState;
use crate::ui_overlay::UiOverlay;
use crate::view::{FlyInput, ViewController};
use crate::world_loot::WorldLoot;
pub(crate) struct ClientInner {
    renderer: Renderer,
    pub(crate) canvas: HtmlCanvasElement,
    /// Buffer / CSS size (HiDPI).
    pixels_per_point: f32,
    pub(crate) self_state: SelfState,
    pub(crate) view: ViewController,
    pub(crate) session: InputSession,
    pub(crate) soft_pointer: SoftPointer,
    pub(crate) mouse_sens: MouseSensitivity,
    pub(crate) move_input: MoveInput,
    emote_wheel: EmoteWheel,
    hit_marker: HitMarker,
    self_present: SelfPresentState,
    remote_present: RemotePresent,
    corpse_present: CorpsePresent,
    blaster_drop_present: BlasterDropPresent,
    fire: FireState,
    projectiles: ProjectileWorld,
    health_by_id: HashMap<PlayerId, PlayerHealth>,
    world_loot: WorldLoot,
    next_local_drop_id: u64,
    map_present: MapPresentState,
    /// Sim collide/support for the loaded map (066). Empty until map ready.
    map_world: MapWorld,
    pub(crate) sfx: SfxState,
    /// Edge detect local death dump (059).
    was_alive: bool,
    last_frame_secs: f64,
    #[cfg(feature = "debug-tools")]
    fps_ema: f32,
    pub(crate) mp: mp::MpClient,
    pub(crate) ui: UiOverlay,
    #[cfg(feature = "debug-tools")]
    pub(crate) debug: DebugTools,
    pub(crate) fly_input: FlyInput,
    #[cfg(feature = "debug-tools")]
    lineup: LineupState,
}

impl ClientInner {
    pub(crate) fn new(
        renderer: Renderer,
        canvas: HtmlCanvasElement,
        pixels_per_point: f32,
        self_state: SelfState,
        view: ViewController,
        ui: UiOverlay,
        #[cfg(feature = "debug-tools")] debug: DebugTools,
    ) -> Self {
        Self {
            renderer,
            canvas,
            pixels_per_point,
            self_state,
            view,
            session: InputSession::new(),
            soft_pointer: SoftPointer::new(),
            mouse_sens: MouseSensitivity::new(),
            move_input: MoveInput::default(),
            emote_wheel: EmoteWheel::new(),
            hit_marker: HitMarker::new(),
            self_present: SelfPresentState::Idle,
            remote_present: RemotePresent::new(),
            corpse_present: CorpsePresent::new(),
            blaster_drop_present: BlasterDropPresent::new(),
            fire: FireState::new(),
            projectiles: ProjectileWorld::new(),
            health_by_id: HashMap::new(),
            world_loot: WorldLoot::default(),
            next_local_drop_id: 1,
            map_present: MapPresentState::Idle,
            map_world: MapWorld::empty(),
            sfx: SfxState::Idle,
            was_alive: true,
            last_frame_secs: 0.0,
            #[cfg(feature = "debug-tools")]
            fps_ema: 0.0,
            mp: mp::MpClient::new(),
            ui,
            #[cfg(feature = "debug-tools")]
            debug,
            fly_input: FlyInput::default(),
            #[cfg(feature = "debug-tools")]
            lineup: LineupState::Idle,
        }
    }

    #[cfg(feature = "debug-tools")]
    fn drain_debug_host_requests(&mut self) {
        use crate::debug::DebugHostRequest;
        for req in self.debug.take_host_requests() {
            match req {
                DebugHostRequest::Screenshot => {}
                DebugHostRequest::MpJoin => {
                    self.mp.begin_join();
                    self.debug.shell.push_log("mp: joining…");
                    self.ui.set_status("joining…");
                }
                DebugHostRequest::MpLeave => {
                    self.mp.leave();
                    self.remote_present.clear();
                    self.corpse_present.clear();
                    self.blaster_drop_present.clear();
                    self.world_loot.clear();
                    self.health_by_id.clear();
                    self.debug.shell.push_log("mp: left (lobby)");
                    self.ui.set_status(String::new());
                }
                DebugHostRequest::MpStatus => {
                    self.debug.shell.push_log(self.mp.status_line());
                }
                DebugHostRequest::Blaster(letter) => {
                    let msg = self.equip_blaster_cmd(letter);
                    self.debug.shell.push_log(msg);
                }
            }
        }
        // Optional debug tracers (038).
        self.renderer.fire_fx.show_tracers = self.debug.draw_tracers();
    }

    #[cfg(feature = "debug-tools")]
    pub(crate) fn debug_execute(&mut self, line: &str) -> String {
        let mouse_sens = &mut self.mouse_sens;
        self.debug.execute(line, mouse_sens)
    }

    #[cfg(feature = "debug-tools")]
    fn equip_blaster_cmd(&mut self, letter: u8) -> String {
        if self.fire.blocks_weapon_side() {
            return "blaster: wait for burst to finish".into();
        }
        match equip_blaster_letter(&mut self.self_state, letter) {
            Ok(_) => {
                self.fire.pay_ready(letter);
                let need_reload = match &self.self_present {
                    SelfPresentState::Ready(gpu) => !gpu.has_blaster_letter(letter),
                    _ => true,
                };
                if need_reload {
                    self.self_present = SelfPresentState::Idle;
                }
                format!(
                    "blaster {} (active={:?})",
                    letter as char,
                    self.self_state.active_blaster().map(|l| l as char)
                )
            }
            Err(e) => format!("blaster: {e}"),
        }
    }

    /// Soft pointer owns session mouse while product Gate/Panel chrome or debug console is up (061).
    pub(crate) fn soft_pointer_armed(&self) -> bool {
        if !self.session.is_active() {
            return false;
        }
        #[cfg(feature = "debug-tools")]
        if self.debug.is_open() {
            return true;
        }
        self.ui.wants_ui_input(self.mp.phase())
    }
}
