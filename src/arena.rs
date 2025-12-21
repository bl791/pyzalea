//! Headless PvP arena simulation
//!
//! No network/server for speed

use pyo3::prelude::*;

/// combat constants (1.21)
const ATTACK_RANGE: f64 = 3.0;
const ATTACK_COOLDOWN_TICKS: u32 = 10; // 0.5 seconds @ 20 TPS
const SPRINT_CRIT_MULTIPLIER: f64 = 1.5;
const BASE_DAMAGE_IRON_SWORD: f64 = 6.0;
const DIAMOND_ARMOR_REDUCTION: f64 = 0.8; // 80% damage after armor
const MAX_HEALTH: f64 = 20.0;
const MAX_FOOD: f64 = 20.0;
const FOOD_HEAL_THRESHOLD: f64 = 18.0;
const FOOD_HEAL_AMOUNT: f64 = 1.0;
const FOOD_PER_STEAK: f64 = 8.0;
const EAT_TICKS: u32 = 32; // 1.6s to eat

// movement constants (1.21)
const WALK_SPEED: f64 = 0.1; // blocks per tick
const SPRINT_SPEED: f64 = 0.13;
const JUMP_VELOCITY: f64 = 0.42;
const GRAVITY: f64 = 0.08;
const DRAG: f64 = 0.98;
const KNOCKBACK_HORIZONTAL: f64 = 0.4;
const KNOCKBACK_VERTICAL: f64 = 0.36;

#[pyclass]
#[derive(Clone, Debug)]
pub struct Fighter {
    // Position
    #[pyo3(get)]
    pub x: f64,
    #[pyo3(get)]
    pub y: f64,
    #[pyo3(get)]
    pub z: f64,

    // Velocity
    #[pyo3(get)]
    pub vx: f64,
    #[pyo3(get)]
    pub vy: f64,
    #[pyo3(get)]
    pub vz: f64,

    // Rotation (degrees)
    #[pyo3(get)]
    pub yaw: f64,
    #[pyo3(get)]
    pub pitch: f64,

    // State
    #[pyo3(get)]
    pub health: f64,
    #[pyo3(get)]
    pub food: f64,
    #[pyo3(get)]
    pub steaks: u32,

    // Cooldowns
    #[pyo3(get)]
    pub attack_cooldown: u32,
    #[pyo3(get)]
    pub eating_ticks: u32,
    #[pyo3(get)]
    pub jump_cooldown: u32,  // prevent jump spam

    // Flags
    #[pyo3(get)]
    pub on_ground: bool,
    #[pyo3(get)]
    pub sprinting: bool,
    #[pyo3(get)]
    pub eating: bool,

    // Stats for this episode
    #[pyo3(get)]
    pub damage_dealt: f64,
    #[pyo3(get)]
    pub damage_taken: f64,
    #[pyo3(get)]
    pub hits_landed: u32,
    #[pyo3(get)]
    pub hits_taken: u32,
}

impl Default for Fighter {
    fn default() -> Self {
        Self {
            x: 0.0, y: 0.0, z: 0.0,
            vx: 0.0, vy: 0.0, vz: 0.0,
            yaw: 0.0, pitch: 0.0,
            health: MAX_HEALTH,
            food: MAX_FOOD,
            steaks: 64,
            attack_cooldown: 0,
            eating_ticks: 0,
            jump_cooldown: 0,
            on_ground: true,
            sprinting: false,
            eating: false,
            damage_dealt: 0.0,
            damage_taken: 0.0,
            hits_landed: 0,
            hits_taken: 0,
        }
    }
}

#[pymethods]
impl Fighter {
    #[new]
    fn new() -> Self {
        Self::default()
    }

    /// attack cooldown as 0-1 (1 = ready)
    fn cooldown_progress(&self) -> f64 {
        if self.attack_cooldown == 0 {
            1.0
        } else {
            1.0 - (self.attack_cooldown as f64 / ATTACK_COOLDOWN_TICKS as f64)
        }
    }
}

/// Action input for a fighter
#[pyclass]
#[derive(Clone, Debug, Default)]
pub struct FighterAction {
    #[pyo3(get, set)]
    pub forward: bool,
    #[pyo3(get, set)]
    pub backward: bool,
    #[pyo3(get, set)]
    pub left: bool,
    #[pyo3(get, set)]
    pub right: bool,
    #[pyo3(get, set)]
    pub jump: bool,
    #[pyo3(get, set)]
    pub sprint: bool,
    #[pyo3(get, set)]
    pub attack: bool,
    #[pyo3(get, set)]
    pub eat: bool,
    #[pyo3(get, set)]
    pub delta_yaw: f64,   // degrees
    #[pyo3(get, set)]
    pub delta_pitch: f64, // degrees
}

#[pymethods]
impl FighterAction {
    #[new]
    fn new() -> Self {
        Self::default()
    }
}

/// Ultra-fast headless PvP arena
#[pyclass]
pub struct FastArena {
    pub fighter1: Fighter,
    pub fighter2: Fighter,

    #[pyo3(get)]
    pub tick: u32,
    #[pyo3(get)]
    pub done: bool,
    #[pyo3(get)]
    pub winner: i32, // 0=none, 1=fighter1, 2=fighter2, -1=draw

    // Arena bounds
    pub min_x: f64,
    pub max_x: f64,
    pub min_z: f64,
    pub max_z: f64,
    pub floor_y: f64,

    // Config
    pub max_ticks: u32,
}

impl FastArena {
    fn apply_movement(&self, fighter: &mut Fighter, action: &FighterAction) {
        fighter.yaw += action.delta_yaw;
        fighter.pitch = (fighter.pitch + action.delta_pitch).clamp(-90.0, 90.0);

        // Normalize yaw to [-180, 180]
        while fighter.yaw > 180.0 { fighter.yaw -= 360.0; }
        while fighter.yaw < -180.0 { fighter.yaw += 360.0; }

        let yaw_rad = fighter.yaw.to_radians();
        let sin_yaw = yaw_rad.sin();
        let cos_yaw = yaw_rad.cos();

        let mut move_x = 0.0;
        let mut move_z = 0.0;

        if action.forward {
            move_x -= sin_yaw;
            move_z += cos_yaw;
        }
        if action.backward {
            move_x += sin_yaw;
            move_z -= cos_yaw;
        }
        if action.left {
            move_x += cos_yaw;
            move_z += sin_yaw;
        }
        if action.right {
            move_x -= cos_yaw;
            move_z -= sin_yaw;
        }

        // Normalize diagonal movement
        let move_len = (move_x * move_x + move_z * move_z).sqrt();
        if move_len > 0.0 {
            move_x /= move_len;
            move_z /= move_len;
        }

        // Apply speed
        fighter.sprinting = action.sprint && action.forward && fighter.food > 6.0;
        let speed = if fighter.sprinting { SPRINT_SPEED } else { WALK_SPEED };

        if !fighter.eating {
            fighter.vx += move_x * speed;
            fighter.vz += move_z * speed;
        }

        // Jump
        // Jump - requires on_ground, not eating, and cooldown ready
        if action.jump && fighter.on_ground && !fighter.eating && fighter.jump_cooldown == 0 {
            fighter.vy = JUMP_VELOCITY;
            fighter.on_ground = false;
            fighter.jump_cooldown = 10;  // ~0.5 sec cooldown after landing
        }

        // Apply gravity
        if !fighter.on_ground {
            fighter.vy -= GRAVITY;
        }

        // Apply drag
        fighter.vx *= DRAG;
        fighter.vz *= DRAG;

        // Update position
        fighter.x += fighter.vx;
        fighter.y += fighter.vy;
        fighter.z += fighter.vz;

        // Floor collision
        if fighter.y <= self.floor_y {
            fighter.y = self.floor_y;
            fighter.vy = 0.0;
            fighter.on_ground = true;
        }

        // Arena bounds
        fighter.x = fighter.x.clamp(self.min_x, self.max_x);
        fighter.z = fighter.z.clamp(self.min_z, self.max_z);
    }

    fn try_attack(&mut self, attacker_idx: usize) -> bool {
        let (attacker, defender) = if attacker_idx == 0 {
            (&mut self.fighter1, &mut self.fighter2)
        } else {
            (&mut self.fighter2, &mut self.fighter1)
        };

        // Check cooldown
        if attacker.attack_cooldown > 0 || attacker.eating {
            return false;
        }

        // Check range
        let dx = defender.x - attacker.x;
        let dy = defender.y - attacker.y;
        let dz = defender.z - attacker.z;
        let dist = (dx*dx + dy*dy + dz*dz).sqrt();

        if dist > ATTACK_RANGE {
            return false;
        }

        // Check if looking at target
        let to_target_yaw = (-dx).atan2(dz).to_degrees();
        let mut yaw_diff = (attacker.yaw - to_target_yaw).abs();
        if yaw_diff > 180.0 { yaw_diff = 360.0 - yaw_diff; }

        if yaw_diff > 60.0 {
            return false;
        }

        // Hit! Calculate damage
        let mut damage = BASE_DAMAGE_IRON_SWORD;

        // Sprint crit
        if attacker.sprinting && !attacker.on_ground {
            damage *= SPRINT_CRIT_MULTIPLIER;
        }

        // Armor reduction
        damage *= 1.0 - DIAMOND_ARMOR_REDUCTION;

        // Apply damage
        defender.health -= damage;
        defender.damage_taken += damage;
        defender.hits_taken += 1;

        attacker.damage_dealt += damage;
        attacker.hits_landed += 1;
        attacker.attack_cooldown = ATTACK_COOLDOWN_TICKS;

        // Knockback
        let kb_yaw = attacker.yaw.to_radians();
        defender.vx += -kb_yaw.sin() * KNOCKBACK_HORIZONTAL;
        defender.vz += kb_yaw.cos() * KNOCKBACK_HORIZONTAL;
        defender.vy += KNOCKBACK_VERTICAL;
        defender.on_ground = false;

        // Interrupt eating
        defender.eating = false;
        defender.eating_ticks = 0;

        // Stop sprinting after hit
        attacker.sprinting = false;

        true
    }

    fn process_eating(&mut self, fighter: &mut Fighter, wants_eat: bool) {
        if wants_eat && !fighter.eating && fighter.steaks > 0 && fighter.food < MAX_FOOD {
            fighter.eating = true;
            fighter.eating_ticks = EAT_TICKS;
        }

        if fighter.eating {
            fighter.eating_ticks = fighter.eating_ticks.saturating_sub(1);

            if fighter.eating_ticks == 0 {
                // Finished eating
                fighter.eating = false;
                fighter.steaks -= 1;
                fighter.food = (fighter.food + FOOD_PER_STEAK).min(MAX_FOOD);
            }
        }

        // Natural regen when food is high
        if fighter.food >= FOOD_HEAL_THRESHOLD && fighter.health < MAX_HEALTH {
            fighter.health = (fighter.health + FOOD_HEAL_AMOUNT * 0.05).min(MAX_HEALTH);
            fighter.food -= 0.1; // Slow food drain during regen
        }
    }
}

#[pymethods]
impl FastArena {
    #[new]
    #[pyo3(signature = (arena_size=32.0, max_ticks=2400))]
    fn new(arena_size: f64, max_ticks: u32) -> Self {
        let half = arena_size / 2.0;
        Self {
            fighter1: Fighter::default(),
            fighter2: Fighter::default(),
            tick: 0,
            done: false,
            winner: 0,
            min_x: -half,
            max_x: half,
            min_z: -half,
            max_z: half,
            floor_y: 0.0,
            max_ticks,
        }
    }

    /// Reset arena for new episode
    fn reset(&mut self, spawn_distance: f64) {
        // MC yaw: 0=+Z, 90=-X, -90=+X, 180=-Z
        self.fighter1 = Fighter {
            x: -spawn_distance / 2.0,
            z: 0.0,
            yaw: -90.0, // Facing +X (east, toward fighter2)
            ..Fighter::default()
        };
        self.fighter2 = Fighter {
            x: spawn_distance / 2.0,
            z: 0.0,
            yaw: 90.0, // Facing -X (west, toward fighter1)
            ..Fighter::default()
        };
        self.tick = 0;
        self.done = false;
        self.winner = 0;
    }

    /// Step the simulation by one tick
    /// Returns: (reward1, reward2, done)
    fn step(&mut self, action1: &FighterAction, action2: &FighterAction) -> (f64, f64, bool) {
        if self.done {
            return (0.0, 0.0, true);
        }

        let health1_before = self.fighter1.health;
        let health2_before = self.fighter2.health;

        // Track eating state before step
        let was_eating1 = self.fighter1.eating;
        let was_eating2 = self.fighter2.eating;
        let eating_ticks1_before = self.fighter1.eating_ticks;
        let eating_ticks2_before = self.fighter2.eating_ticks;

        // Process cooldowns
        self.fighter1.attack_cooldown = self.fighter1.attack_cooldown.saturating_sub(1);
        self.fighter2.attack_cooldown = self.fighter2.attack_cooldown.saturating_sub(1);
        self.fighter1.jump_cooldown = self.fighter1.jump_cooldown.saturating_sub(1);
        self.fighter2.jump_cooldown = self.fighter2.jump_cooldown.saturating_sub(1);

        // Movement (clone fighters for borrow checker)
        let mut f1 = self.fighter1.clone();
        let mut f2 = self.fighter2.clone();
        self.apply_movement(&mut f1, action1);
        self.apply_movement(&mut f2, action2);
        self.fighter1 = f1;
        self.fighter2 = f2;

        // Attacks - track if we tried but missed
        let tried1 = action1.attack;
        let tried2 = action2.attack;
        let hit1 = if tried1 { self.try_attack(0) } else { false };
        let hit2 = if tried2 { self.try_attack(1) } else { false };
        let whiff1 = tried1 && !hit1;  // Swung but missed
        let whiff2 = tried2 && !hit2;

        // Eating
        let mut f1 = self.fighter1.clone();
        let mut f2 = self.fighter2.clone();
        self.process_eating(&mut f1, action1.eat);
        self.process_eating(&mut f2, action2.eat);
        self.fighter1 = f1;
        self.fighter2 = f2;

        // Track eating completion
        let finished_eating1 = was_eating1 && !self.fighter1.eating && eating_ticks1_before == 1;
        let finished_eating2 = was_eating2 && !self.fighter2.eating && eating_ticks2_before == 1;
        let interrupted_eating1 = was_eating1 && !self.fighter1.eating && eating_ticks1_before > 1;
        let interrupted_eating2 = was_eating2 && !self.fighter2.eating && eating_ticks2_before > 1;

        self.tick += 1;

        // Calculate rewards
        let damage1_dealt = health2_before - self.fighter2.health;
        let damage2_dealt = health1_before - self.fighter1.health;

        let mut reward1 = damage1_dealt * 0.5 - damage2_dealt * 0.3;
        let mut reward2 = damage2_dealt * 0.5 - damage1_dealt * 0.3;

        // Hit bonus
        if hit1 { reward1 += 0.2; }
        if hit2 { reward2 += 0.2; }

        // Whiff penalty
        if whiff1 { reward1 -= 0.05; }
        if whiff2 { reward2 -= 0.05; }

        // No penalty for tactical jumps
        if action1.jump && self.fighter1.jump_cooldown > 0 { reward1 -= 0.03; }
        if action2.jump && self.fighter2.jump_cooldown > 0 { reward2 -= 0.03; }

        // Healing rewards
        let heal1 = (self.fighter1.health - health1_before).max(0.0);
        let heal2 = (self.fighter2.health - health2_before).max(0.0);
        reward1 += heal1 * 0.3;  // Healing is valuable
        reward2 += heal2 * 0.3;

        // Reward for continuing to eat (each tick of progress)
        if self.fighter1.eating && was_eating1 {
            reward1 += 0.02;  // Small reward for each tick of eating
        }
        if self.fighter2.eating && was_eating2 {
            reward2 += 0.02;
        }

        // Bonus for successfully finishing eating
        if finished_eating1 { reward1 += 0.5; }
        if finished_eating2 { reward2 += 0.5; }

        // Penalty for getting eating interrupted
        if interrupted_eating1 { reward1 -= 0.3; }
        if interrupted_eating2 { reward2 -= 0.3; }

        // Approach reward
        let dx = self.fighter2.x - self.fighter1.x;
        let dz = self.fighter2.z - self.fighter1.z;
        let dist = (dx*dx + dz*dz).sqrt();

        // Proximity reward (peaks at attack range ~3 blocks)
        if dist < 10.0 {
            let closeness_reward = (10.0 - dist) * 0.01;  // Max 0.1 per tick when very close
            reward1 += closeness_reward;
            reward2 += closeness_reward;
        }

        // Small time penalty
        reward1 -= 0.001;
        reward2 -= 0.001;

        // Check win conditions
        if self.fighter1.health <= 0.0 {
            self.done = true;
            self.winner = 2;
            reward1 -= 10.0;
            reward2 += 10.0;
            // Speed bonus: up to +5.0 for quick kills
            let speed_bonus = 5.0 * (1.0 - self.tick as f64 / self.max_ticks as f64);
            reward2 += speed_bonus;
        } else if self.fighter2.health <= 0.0 {
            self.done = true;
            self.winner = 1;
            reward1 += 10.0;
            reward2 -= 10.0;
            // Speed bonus: up to +5.0 for quick kills
            let speed_bonus = 5.0 * (1.0 - self.tick as f64 / self.max_ticks as f64);
            reward1 += speed_bonus;
        } else if self.tick >= self.max_ticks {
            self.done = true;
            // Winner by health
            if self.fighter1.health > self.fighter2.health {
                self.winner = 1;
                reward1 += 2.0;
                reward2 -= 2.0;
            } else if self.fighter2.health > self.fighter1.health {
                self.winner = 2;
                reward1 -= 2.0;
                reward2 += 2.0;
            } else {
                self.winner = -1; // Draw
                // Penalize both fighters for not finishing the fight
                reward1 -= 3.0;
                reward2 -= 3.0;
            }
        }

        (reward1, reward2, self.done)
    }

    fn get_obs1(&self) -> Vec<f64> {
        self.get_obs(&self.fighter1, &self.fighter2)
    }

    fn get_obs2(&self) -> Vec<f64> {
        self.get_obs(&self.fighter2, &self.fighter1)
    }

    /// Get observation vector
    fn get_obs(&self, me: &Fighter, enemy: &Fighter) -> Vec<f64> {
        let dx = enemy.x - me.x;
        let dy = enemy.y - me.y;
        let dz = enemy.z - me.z;
        let dist = (dx*dx + dy*dy + dz*dz).sqrt();

        // Calculate enemy yaw relative to looking at us
        let enemy_to_me_yaw = (-(-dx)).atan2(-dz).to_degrees();

        vec![
            // My state (13)
            me.x / 32.0,
            me.y / 32.0,
            me.z / 32.0,
            me.vx,
            me.vy,
            me.vz,
            me.health / MAX_HEALTH,
            me.food / MAX_FOOD,
            me.cooldown_progress(),
            me.yaw / 180.0,
            me.pitch / 90.0,
            if me.on_ground { 1.0 } else { 0.0 },
            if me.sprinting { 1.0 } else { 0.0 },
            // Enemy state (10)
            dx / 32.0,
            dy / 16.0,
            dz / 32.0,
            (dist / 32.0).min(1.0),
            enemy.health / MAX_HEALTH,
            enemy_to_me_yaw / 180.0,
            enemy.vx,
            enemy.vy,
            enemy.vz,
            1.0, // enemy visible (always true in arena)
            // Combat state (4)
            me.damage_dealt / 40.0,
            me.damage_taken / 40.0,
            (me.hits_landed as f64) / 20.0,
            (me.hits_taken as f64) / 20.0,
            // Eating state (4)
            if me.eating { 1.0 } else { 0.0 },
            me.eating_ticks as f64 / EAT_TICKS as f64,  // Progress (1.0 = just started, 0.0 = done)
            if enemy.eating { 1.0 } else { 0.0 },  // Enemy is vulnerable!
            me.steaks as f64 / 64.0,  // Steaks remaining
        ]
    }

    /// Get fighter 1 state
    fn get_fighter1(&self) -> Fighter {
        self.fighter1.clone()
    }

    /// Get fighter 2 state
    fn get_fighter2(&self) -> Fighter {
        self.fighter2.clone()
    }

    /// Run N ticks with given actions (for batched simulation)
    fn step_n(&mut self, n: u32, action1: &FighterAction, action2: &FighterAction) -> (f64, f64, bool) {
        let mut total_r1 = 0.0;
        let mut total_r2 = 0.0;

        for _ in 0..n {
            let (r1, r2, done) = self.step(action1, action2);
            total_r1 += r1;
            total_r2 += r2;
            if done { break; }
        }

        (total_r1, total_r2, self.done)
    }
}

#[pyclass]
pub struct ArenaVec {
    arenas: Vec<FastArena>,
}

#[pymethods]
impl ArenaVec {
    #[new]
    fn new(count: usize, arena_size: f64, max_ticks: u32) -> Self {
        let arenas = (0..count)
            .map(|_| FastArena::new(arena_size, max_ticks))
            .collect();
        Self { arenas }
    }

    fn len(&self) -> usize {
        self.arenas.len()
    }

    /// Reset all arenas
    fn reset_all(&mut self, spawn_distance: f64) {
        for arena in &mut self.arenas {
            arena.reset(spawn_distance);
        }
    }

    /// Reset specific arena
    fn reset(&mut self, idx: usize, spawn_distance: f64) {
        if idx < self.arenas.len() {
            self.arenas[idx].reset(spawn_distance);
        }
    }

    /// Step specific arena
    fn step(&mut self, idx: usize, action1: &FighterAction, action2: &FighterAction) -> (f64, f64, bool) {
        if idx < self.arenas.len() {
            self.arenas[idx].step(action1, action2)
        } else {
            (0.0, 0.0, true)
        }
    }

    /// Get observation from specific arena
    fn get_obs1(&self, idx: usize) -> Vec<f64> {
        if idx < self.arenas.len() {
            self.arenas[idx].get_obs1()
        } else {
            vec![0.0; 27]
        }
    }

    fn get_obs2(&self, idx: usize) -> Vec<f64> {
        if idx < self.arenas.len() {
            self.arenas[idx].get_obs2()
        } else {
            vec![0.0; 27]
        }
    }

    /// Check if arena is done
    fn is_done(&self, idx: usize) -> bool {
        if idx < self.arenas.len() {
            self.arenas[idx].done
        } else {
            true
        }
    }

    /// Get winner of arena
    fn get_winner(&self, idx: usize) -> i32 {
        if idx < self.arenas.len() {
            self.arenas[idx].winner
        } else {
            0
        }
    }
}
