// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
// The ultimate strictness: catches things like missing documentation or overflow risks
#![warn(clippy::restriction)] 

fn main() {
  abyss_watcher::run();
}
