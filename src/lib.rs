extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

mod schedule;
mod appointment;

pub use schedule::*;
pub use appointment::*;
