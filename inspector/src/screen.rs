mod explorer;
pub use explorer::*;

mod loader;
pub use loader::*;

pub enum Screen {
    Loader(Loader),
    Explorer(ActionPath),
}
