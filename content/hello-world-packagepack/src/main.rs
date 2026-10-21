fn main() {
    terminal_engine::run(|app| {
        vapor_example_wheel_salvage::install_composition(app);
    });
}
