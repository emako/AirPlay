#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod cue;
mod lrc;
mod meta;
mod net;
mod player;
mod playlist;
mod skin;
mod textutil;
mod track;
mod ui;

fn main() {
    let code = std::panic::catch_unwind(|| ui::run()).unwrap_or(1);
    std::process::exit(code);
}
