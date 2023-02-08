use log::*;
use colored::*;

pub struct ClientLogger {
    peer_id: Option<String>,
    kam_level: Level,
    other_level: Level,
}

fn colored_level(level: Level) -> ColoredString {
    match level {
        Level::Error => level.to_string().red().bold(),
        Level::Warn => level.to_string().yellow(),
        Level::Info => level.to_string().green(),
        Level::Debug => level.to_string().blue(),
        Level::Trace => level.to_string().magenta(),
    }
}

impl ClientLogger {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            peer_id: None,
            kam_level: Level::Trace,
            other_level: Level::Info,
        })
    }

    pub fn with_peer_id(&mut self, peer_id: libp2p::PeerId) {
        self.peer_id = Some(format!("{peer_id} "));
    }

    pub fn with_level(&mut self, level: Level) {
        self.kam_level = level;
    }

    pub fn activate(self: Box<Self>) {
        log::set_logger(Box::leak(self)).unwrap();
        log::set_max_level(LevelFilter::Trace);
    }
}

impl Log for ClientLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        if metadata.target().contains("kamilata") {
            metadata.level() <= self.kam_level
        } else {
            metadata.level() <= self.other_level
        }
    }

    fn log(&self, record: &Record) {
        if Log::enabled(self, record.metadata()) {
            let mut args = record.args().to_string();
            let mut target = record.target().to_string();
            if let Some(peer_id) = &self.peer_id {
                if args.starts_with("12D3KooW") {
                    if !args.starts_with(peer_id) {
                        return;
                    }
                    args = args[peer_id.len()..].replace(peer_id, "12D3KooW.. ");
                }
            }
            if target.contains("kamilata") {
                target = target.replace("kamilata::", "kam::");
            }
            println!(
                "[{} {target}] {args}", colored_level(record.level()),
            );
        }
    }

    fn flush(&self) {}
}
