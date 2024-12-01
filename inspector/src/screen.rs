mod explorer;
pub use explorer::*;

mod loader;
pub use loader::*;

mod game;
pub use game::*;

pub enum Screen {
    Loader(Loader),
    Explorer(ActionPath),
    Game(MusArenaUi),
}
