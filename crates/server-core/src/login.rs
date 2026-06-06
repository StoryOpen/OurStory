use protocol::types::WorldId;

pub struct Login {
    worlds: Vec<WorldInfo>,
}

pub struct WorldInfo {
    pub id: WorldId,
    pub name: String,
    pub channel_count: i32,
    pub flag: i32,
    pub exp_rate: f64,
    pub meso_rate: f64,
}

impl Login {
    pub fn new() -> Self {
        Self { worlds: Vec::new() }
    }

    pub fn add_world(&mut self, info: WorldInfo) {
        self.worlds.push(info);
    }

    pub fn worlds(&self) -> &[WorldInfo] {
        &self.worlds
    }
}
