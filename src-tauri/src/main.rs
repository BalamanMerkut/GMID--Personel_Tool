// Sürüm derlemesinde Windows'ta arkada siyah bir konsol penceresi açılmasın.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    gmid_tool_lib::run()
}
