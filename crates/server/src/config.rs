pub struct Config {
    pub login_port: u16,
    pub worlds: Vec<WorldConfig>,
    pub db_url: String,
}

pub struct WorldConfig {
    pub id: i32,
    pub name: String,
    pub channels: Vec<ChannelConfig>,
    #[allow(dead_code)]
    pub exp_rate: f64,
    #[allow(dead_code)]
    pub meso_rate: f64,
}

pub struct ChannelConfig {
    pub id: i32,
    pub port: u16,
}

#[allow(dead_code)]
pub struct MapConfig {
    pub map_id: i32,
    pub channel_port: u16,
}

impl Config {
    pub fn load(_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            login_port: 8484,
            worlds: vec![WorldConfig {
                id: 0,
                name: "Scania".into(),
                channels: vec![
                    ChannelConfig {
                        id: 1,
                        port: 7575,
                    },
                    ChannelConfig {
                        id: 2,
                        port: 7576,
                    },
                    ChannelConfig {
                        id: 3,
                        port: 7577,
                    },
                ],
                exp_rate: 1.0,
                meso_rate: 1.0,
            }],
            db_url: "postgres://localhost/ourstory".into(),
        })
    }
}
