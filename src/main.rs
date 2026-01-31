// Main entry point - использует общий код из lib.rs
use roggy;

fn main() {
    let mut app = roggy::build_base_app();
    app.run();
}
