use pyo3::prelude::*;
use std::sync::Arc;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use azalea::prelude::*;
use azalea::{ClientBuilder, Account, WalkDirection, SprintDirection, BlockPos};
use azalea::pathfinder::goals::{BlockPosGoal, RadiusGoal};
use azalea_client::Client;
use azalea_client::local_player::{LocalGameMode, PermissionLevel};
use azalea_core::game_type::GameMode;

use crate::state::PyGameState;
use crate::RUNTIME;

#[pyclass]
pub struct PyBot {
    inner: Arc<Mutex<Option<Client>>>,
    connected: Arc<AtomicBool>,
    username: String,
}

#[pymethods]
impl PyBot {
    #[getter]
    fn username(&self) -> String {
        self.username.clone()
    }

    #[getter]
    fn connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    fn is_in_game(&self) -> bool {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            // if we can access position, we're in game
            if let Ok(_) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _ = client.position();
                let _ = client.health();
            })) {
                return true;
            }
        }
        false
    }

    fn get_state(&self) -> PyGameState {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            let mut state = PyGameState::default();

            // get position
            if let Ok(pos) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                client.position()
            })) {
                state.x = pos.x;
                state.y = pos.y;
                state.z = pos.z;
            }

            // get health
            if let Ok(health) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                client.health()
            })) {
                state.health = health;
            }

            // get hunger
            if let Ok(hunger) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                client.hunger()
            })) {
                state.food = hunger.food as u32;
            }

            return state;
        }
        PyGameState::default()
    }

    fn walk(&self, direction: &str) -> PyResult<()> {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            let dir = match direction {
                "forward" => WalkDirection::Forward,
                "backward" => WalkDirection::Backward,
                "left" => WalkDirection::Left,
                "right" => WalkDirection::Right,
                "forward_left" => WalkDirection::ForwardLeft,
                "forward_right" => WalkDirection::ForwardRight,
                "backward_left" => WalkDirection::BackwardLeft,
                "backward_right" => WalkDirection::BackwardRight,
                _ => WalkDirection::None,
            };
            client.walk(dir);
        }
        Ok(())
    }

    fn move_forward(&self) -> PyResult<()> {
        self.walk("forward")
    }

    fn move_backward(&self) -> PyResult<()> {
        self.walk("backward")
    }

    fn move_left(&self) -> PyResult<()> {
        self.walk("left")
    }

    fn move_right(&self) -> PyResult<()> {
        self.walk("right")
    }

    fn stop(&self) -> PyResult<()> {
        self.walk("none")
    }

    fn jump(&self) -> PyResult<()> {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            client.set_jumping(true);
        }
        Ok(())
    }

    fn sprint(&self) -> PyResult<()> {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            client.sprint(SprintDirection::Forward);
        }
        Ok(())
    }

    /// set look direction (yaw = pitch in degrees)
    fn set_look(&self, yaw: f32, pitch: f32) -> PyResult<()> {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            client.set_direction(yaw, pitch);
        }
        Ok(())
    }

    /// look at a position in world
    fn look_at(&self, x: f64, y: f64, z: f64) -> PyResult<()> {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            client.look_at(azalea::Vec3::new(x, y, z));
        }
        Ok(())
    }

    fn chat(&self, message: &str) -> PyResult<()> {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            client.chat(message);
        }
        Ok(())
    }

    fn attack_player(&self, username: &str) -> PyResult<bool> {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            // get uuid
            if let Some(uuid) = client.player_uuid_by_username(username) {
                // get ECS entity
                if let Some(entity) = client.entity_by_uuid(uuid) {
                    client.attack(entity);
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// check cooldown (returns value 0.0-1.0, 1.0 = ready)
    fn attack_cooldown(&self) -> f32 {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            if client.has_attack_cooldown() {
                // Still on cooldown - estimate based on remaining ticks
                let remaining = client.attack_cooldown_remaining_ticks();
                // Sword cooldown is ~12 ticks (0.6s), so normalize
                return 1.0 - (remaining as f32 / 12.0).min(1.0);
            }
            return 1.0;
        }
        1.0
    }

    fn get_players(&self) -> Vec<String> {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            if let Ok(players) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                client.tab_list()
                    .values()
                    .map(|info| info.profile.name.clone())
                    .collect()
            })) {
                return players;
            }
        }
        vec![]
    }

    fn get_player_position(&self, username: &str) -> Option<(f64, f64, f64)> {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            let username = username.to_string();
            if let Ok(result) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if let Some(uuid) = client.player_uuid_by_username(&username) {
                    if let Some(entity) = client.entity_by_uuid(uuid) {
                        let ecs = client.ecs.lock();
                        if let Some(pos) = ecs.get::<azalea_entity::Position>(entity) {
                            return Some((pos.x, pos.y, pos.z));
                        }
                    }
                }
                None
            })) {
                return result;
            }
        }
        None
    }

    /// pathfind
    fn goto(&self, x: i32, y: i32, z: i32) -> PyResult<()> {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            let goal = BlockPosGoal(BlockPos::new(x, y, z));
            client.start_goto(goal);
        }
        Ok(())
    }

    /// pathfind to radius
    fn goto_radius(&self, x: f64, y: f64, z: f64, radius: f32) -> PyResult<()> {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            let goal = RadiusGoal {
                pos: azalea::Vec3::new(x, y, z),
                radius,
            };
            client.start_goto(goal);
        }
        Ok(())
    }

    /// pathfind to player
    fn goto_player(&self, username: &str, radius: f32) -> PyResult<bool> {
        if let Some((x, y, z)) = self.get_player_position(username) {
            self.goto_radius(x, y, z, radius)?;
            return Ok(true);
        }
        Ok(false)
    }

    /// cancel pathfind
    fn stop_pathfinding(&self) -> PyResult<()> {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            client.stop_pathfinding();
        }
        Ok(())
    }

    fn set_hotbar_slot(&self, slot: u8) -> PyResult<()> {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            if slot < 9 {
                client.set_selected_hotbar_slot(slot);
            }
        }
        Ok(())
    }

    fn get_hotbar_slot(&self) -> u8 {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            if let Ok(slot) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                client.selected_hotbar_slot()
            })) {
                return slot;
            }
        }
        0
    }

    fn use_held_item(&self) -> PyResult<()> {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            client.start_use_item();
        }
        Ok(())
    }

    fn is_creative(&self) -> bool {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            if let Some(game_mode) = client.get_component::<LocalGameMode>() {
                return game_mode.current == GameMode::Creative;
            }
        }
        false
    }

    fn game_mode(&self) -> String {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            if let Some(game_mode) = client.get_component::<LocalGameMode>() {
                return match game_mode.current {
                    GameMode::Survival => "survival".to_string(),
                    GameMode::Creative => "creative".to_string(),
                    GameMode::Adventure => "adventure".to_string(),
                    GameMode::Spectator => "spectator".to_string(),
                };
            }
        }
        "unknown".to_string()
    }

    /// get permission level (0-4, 2+ is op)
    fn permission_level(&self) -> u8 {
        let guard = self.inner.lock();
        if let Some(ref client) = *guard {
            if let Some(perm) = client.get_component::<PermissionLevel>() {
                return *perm;
            }
        }
        0
    }

    fn is_op(&self) -> bool {
        self.permission_level() >= 2
    }

    fn disconnect(&self) -> PyResult<()> {
        let mut guard = self.inner.lock();
        if let Some(ref client) = *guard {
            client.disconnect();
        }
        *guard = None;
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn tick(&self) -> PyResult<PyGameState> {
        RUNTIME.block_on(async {
            // wait one tick (50ms = 20 TPS)
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        });
        Ok(self.get_state())
    }

    /// gym-style interface
    fn step(&self, action: &Bound<'_, pyo3::types::PyDict>) -> PyResult<PyGameState> {
        // Parse movement - convert to walk direction
        let forward = action.get_item("forward")?.map(|v| v.extract::<bool>().unwrap_or(false)).unwrap_or(false);
        let backward = action.get_item("backward")?.map(|v| v.extract::<bool>().unwrap_or(false)).unwrap_or(false);
        let left = action.get_item("left")?.map(|v| v.extract::<bool>().unwrap_or(false)).unwrap_or(false);
        let right = action.get_item("right")?.map(|v| v.extract::<bool>().unwrap_or(false)).unwrap_or(false);
        let jump = action.get_item("jump")?.map(|v| v.extract::<bool>().unwrap_or(false)).unwrap_or(false);
        let sprint = action.get_item("sprint")?.map(|v| v.extract::<bool>().unwrap_or(false)).unwrap_or(false);

        // determine walk direction
        let direction = match (forward, backward, left, right) {
            (true, false, false, false) => "forward",
            (false, true, false, false) => "backward",
            (false, false, true, false) => "left",
            (false, false, false, true) => "right",
            (true, false, true, false) => "forward_left",
            (true, false, false, true) => "forward_right",
            (false, true, true, false) => "backward_left",
            (false, true, false, true) => "backward_right",
            _ => "none",
        };
        self.walk(direction)?;

        if jump {
            self.jump()?;
        }

        if sprint {
            self.sprint()?;
        }

        // look_at
        if let (Some(x), Some(y), Some(z)) = (
            action.get_item("look_x")?.and_then(|v| v.extract::<f64>().ok()),
            action.get_item("look_y")?.and_then(|v| v.extract::<f64>().ok()),
            action.get_item("look_z")?.and_then(|v| v.extract::<f64>().ok()),
        ) {
            self.look_at(x, y, z)?;
        }

        // yaw/pitch
        if let (Some(yaw), Some(pitch)) = (
            action.get_item("yaw")?.and_then(|v| v.extract::<f32>().ok()),
            action.get_item("pitch")?.and_then(|v| v.extract::<f32>().ok()),
        ) {
            self.set_look(yaw, pitch)?;
        }

        self.tick()
    }

    fn __repr__(&self) -> String {
        format!("PyBot(username={}, connected={})", self.username, self.connected())
    }
}

impl PyBot {
    pub fn connect(host: &str, port: u16, username: &str) -> PyResult<Self> {
        let client_holder: Arc<Mutex<Option<Client>>> = Arc::new(Mutex::new(None));
        let connected = Arc::new(AtomicBool::new(false));

        let client_holder_clone = client_holder.clone();
        let connected_clone = connected.clone();
        let address = format!("{}:{}", host, port);
        let username_owned = username.to_string();

        // spawn bot connection in a separate thread with its own runtime
        // Azalea uses LocalSet which needs a single-threaded runtime
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            rt.block_on(async move {
                let account = Account::offline(&username_owned);

                // event handler component
                #[derive(Clone, Component, Default)]
                struct BotState {
                    client_holder: Option<Arc<Mutex<Option<Client>>>>,
                    connected: Option<Arc<AtomicBool>>,
                }

                async fn handle(bot: Client, event: Event, state: BotState) -> anyhow::Result<()> {
                    match event {
                        Event::Init => {
                            println!("Bot initialized and connected!");
                            // client reference
                            if let Some(ref holder) = state.client_holder {
                                *holder.lock() = Some(bot.clone());
                            }
                            if let Some(ref connected) = state.connected {
                                connected.store(true, Ordering::SeqCst);
                            }
                        }
                        Event::Chat(m) => {
                            println!("Chat: {}", m.message().to_ansi());
                        }
                        Event::Death(_) => {
                            println!("Bot died!");
                        }
                        _ => {}
                    }
                    Ok(())
                }

                println!("Connecting to {}...", address);

                let mut bot_state = BotState::default();
                bot_state.client_holder = Some(client_holder_clone);
                bot_state.connected = Some(connected_clone);

                let result = ClientBuilder::new()
                    .set_handler(handle)
                    .set_state(bot_state)
                    .start(account, address.as_str())
                    .await;

                match result {
                    AppExit::Success => {
                        println!("Bot disconnected normally");
                    }
                    AppExit::Error(e) => {
                        eprintln!("Bot error: {:?}", e);
                    }
                }
            });
        });

        for _ in 0..100 {
            if connected.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        Ok(Self {
            inner: client_holder,
            connected,
            username: username.to_string(),
        })
    }
}
