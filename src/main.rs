use log::LevelFilter;
use simplelog::{ColorChoice, TermLogger, TerminalMode};

fn init_logging() {
    TermLogger::init(LevelFilter::Debug, Default::default(),TerminalMode::Mixed, ColorChoice::Auto).unwrap();
}

fn main() {
    init_logging();
    
    println!("Hello, world!");
}
