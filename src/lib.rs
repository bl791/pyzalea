use pyo3::prelude::*;
use std::sync::Arc;
use tokio::runtime::Runtime;

mod bot;
mod state;
mod arena;

pub use bot::PyBot;
pub use state::PyGameState;
pub use arena::{FastArena, ArenaVec, Fighter, FighterAction};

lazy_static::lazy_static! {
    pub static ref RUNTIME: Arc<Runtime> = Arc::new(
        Runtime::new().expect("Failed to create tokio runtime")
    );
}

#[pyfunction]
#[pyo3(signature = (host, port=25565, username="Bot"))]
fn connect(host: &str, port: u16, username: &str) -> PyResult<PyBot> {
    PyBot::connect(host, port, username)
}

#[pyfunction]
#[pyo3(signature = (host, port=25565, usernames=vec!["Bot1".to_string(), "Bot2".to_string()]))]
fn connect_swarm(host: &str, port: u16, usernames: Vec<String>) -> PyResult<Vec<PyBot>> {
    usernames
        .iter()
        .map(|name| PyBot::connect(host, port, name))
        .collect()
}

/// Python module
#[pymodule]
fn pyzalea(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // fsor connecting to remote servers
    m.add_function(wrap_pyfunction!(connect, m)?)?;
    m.add_function(wrap_pyfunction!(connect_swarm, m)?)?;
    m.add_class::<PyBot>()?;
    m.add_class::<PyGameState>()?;
    m.add_class::<state::PyEntity>()?;

    // headless arena / simulation
    m.add_class::<FastArena>()?;
    m.add_class::<ArenaVec>()?;
    m.add_class::<Fighter>()?;
    m.add_class::<FighterAction>()?;

    Ok(())
}
