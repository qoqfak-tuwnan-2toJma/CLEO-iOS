pub use loader::get_game_path;
pub use res::*;

mod loader;
mod res;
mod stream;

pub fn init() {
    loader::init();
    stream::init();
    res::init();
}
