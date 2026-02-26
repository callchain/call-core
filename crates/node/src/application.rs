use crate::config::Config;
use consensus::{Consensus, ConsensusParms};
use network::Overlay;
use storage::Database;

pub struct Application {
    pub config: Config,
    pub consensus: Consensus,
    pub overlay: Overlay,
    pub database: Database,
}

impl Application {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let backend = Box::new(storage::RocksDBBackend::new(&config.data_dir));
        let database = Database::new(backend);
        let consensus = Consensus::new(ConsensusParms::default());
        let overlay = Overlay::new();

        Ok(Self {
            config,
            consensus,
            overlay,
            database,
        })
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn shutdown(&mut self) {
    }
}
