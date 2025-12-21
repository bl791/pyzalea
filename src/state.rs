use pyo3::prelude::*;

/// entity in the game (player/mob/etc.)
#[pyclass]
#[derive(Clone)]
pub struct PyEntity {
    #[pyo3(get)]
    pub id: u32,
    #[pyo3(get)]
    pub entity_type: String,
    #[pyo3(get)]
    pub x: f64,
    #[pyo3(get)]
    pub y: f64,
    #[pyo3(get)]
    pub z: f64,
    #[pyo3(get)]
    pub yaw: f32,
    #[pyo3(get)]
    pub pitch: f32,
    #[pyo3(get)]
    pub velocity_x: f64,
    #[pyo3(get)]
    pub velocity_y: f64,
    #[pyo3(get)]
    pub velocity_z: f64,
    #[pyo3(get)]
    pub health: f32,
    #[pyo3(get)]
    pub is_on_ground: bool,
}

#[pymethods]
impl PyEntity {
    fn __repr__(&self) -> String {
        format!(
            "Entity(id={}, type={}, pos=({:.1}, {:.1}, {:.1}), health={})",
            self.id, self.entity_type, self.x, self.y, self.z, self.health
        )
    }

    fn position(&self) -> (f64, f64, f64) {
        (self.x, self.y, self.z)
    }

    fn velocity(&self) -> (f64, f64, f64) {
        (self.velocity_x, self.velocity_y, self.velocity_z)
    }

    fn distance_to(&self, other: &PyEntity) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    fn horizontal_distance_to(&self, other: &PyEntity) -> f64 {
        let dx = self.x - other.x;
        let dz = self.z - other.z;
        (dx * dx + dz * dz).sqrt()
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyGameState {
    // Player state
    #[pyo3(get)]
    pub x: f64,
    #[pyo3(get)]
    pub y: f64,
    #[pyo3(get)]
    pub z: f64,
    #[pyo3(get)]
    pub yaw: f32,
    #[pyo3(get)]
    pub pitch: f32,
    #[pyo3(get)]
    pub velocity_x: f64,
    #[pyo3(get)]
    pub velocity_y: f64,
    #[pyo3(get)]
    pub velocity_z: f64,
    #[pyo3(get)]
    pub health: f32,
    #[pyo3(get)]
    pub food: u32,
    #[pyo3(get)]
    pub saturation: f32,
    #[pyo3(get)]
    pub is_on_ground: bool,
    #[pyo3(get)]
    pub is_sprinting: bool,
    #[pyo3(get)]
    pub is_sneaking: bool,
    #[pyo3(get)]
    pub is_dead: bool,

    // Combat state
    #[pyo3(get)]
    pub attack_cooldown: f32,  // 0.0 to 1.0, 1.0 = ready
    #[pyo3(get)]
    pub selected_slot: u8,

    // Nearby entities
    #[pyo3(get)]
    pub entities: Vec<PyEntity>,

    // Game tick
    #[pyo3(get)]
    pub tick: u64,
}

#[pymethods]
impl PyGameState {
    fn __repr__(&self) -> String {
        format!(
            "GameState(pos=({:.1}, {:.1}, {:.1}), health={}, entities={})",
            self.x, self.y, self.z, self.health, self.entities.len()
        )
    }

    fn position(&self) -> (f64, f64, f64) {
        (self.x, self.y, self.z)
    }

    fn velocity(&self) -> (f64, f64, f64) {
        (self.velocity_x, self.velocity_y, self.velocity_z)
    }

    #[pyo3(signature = (entity_type=None, max_distance=None))]
    fn nearest_entity(&self, entity_type: Option<&str>, max_distance: Option<f64>) -> Option<PyEntity> {
        let mut nearest: Option<(f64, &PyEntity)> = None;

        for entity in &self.entities {
            // type filter
            if let Some(t) = entity_type {
                if entity.entity_type != t {
                    continue;
                }
            }

            let dx = self.x - entity.x;
            let dy = self.y - entity.y;
            let dz = self.z - entity.z;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();

            // max distance filter
            if let Some(max_d) = max_distance {
                if dist > max_d {
                    continue;
                }
            }

            match nearest {
                None => nearest = Some((dist, entity)),
                Some((d, _)) if dist < d => nearest = Some((dist, entity)),
                _ => {}
            }
        }

        nearest.map(|(_, e)| e.clone())
    }

    #[pyo3(signature = (max_distance=32.0))]
    fn nearby_players(&self, max_distance: f64) -> Vec<PyEntity> {
        self.entities
            .iter()
            .filter(|e| {
                if e.entity_type != "player" {
                    return false;
                }
                let dx = self.x - e.x;
                let dy = self.y - e.y;
                let dz = self.z - e.z;
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                dist <= max_distance
            })
            .cloned()
            .collect()
    }

    fn to_vector(&self) -> Vec<f32> {
        let mut v = vec![
            self.x as f32,
            self.y as f32,
            self.z as f32,
            self.yaw,
            self.pitch,
            self.velocity_x as f32,
            self.velocity_y as f32,
            self.velocity_z as f32,
            self.health,
            self.food as f32,
            self.is_on_ground as u8 as f32,
            self.is_sprinting as u8 as f32,
            self.is_sneaking as u8 as f32,
            self.attack_cooldown,
        ];

        // add nearest player info
        if let Some(enemy) = self.nearest_entity(Some("player"), None) {
            v.extend_from_slice(&[
                enemy.x as f32,
                enemy.y as f32,
                enemy.z as f32,
                enemy.yaw,
                enemy.pitch,
                enemy.velocity_x as f32,
                enemy.velocity_y as f32,
                enemy.velocity_z as f32,
                enemy.health,
                // relative position
                (enemy.x - self.x) as f32,
                (enemy.y - self.y) as f32,
                (enemy.z - self.z) as f32,
            ]);
        } else {
            // pad with zeros if no enemy
            v.extend_from_slice(&[0.0; 12]);
        }

        v
    }
}

impl Default for PyGameState {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            velocity_z: 0.0,
            health: 20.0,
            food: 20,
            saturation: 5.0,
            is_on_ground: true,
            is_sprinting: false,
            is_sneaking: false,
            is_dead: false,
            attack_cooldown: 1.0,
            selected_slot: 0,
            entities: vec![],
            tick: 0,
        }
    }
}
