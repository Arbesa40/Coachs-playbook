// Desktop entry point, only used to try the companion UI on a PC.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    cpb_companion_lib::run()
}
