//! One module per `src/commands/*.ts` file, same names. Wave 1 stubs every
//! command's body with `Err(UserError::new("not yet ported"))`; wave 2
//! replaces bodies in place without touching this file or `main.rs`'s
//! dispatch signatures (`fn run(argv: Vec<String>) -> Result<(), UserError>`).

pub mod adapt;
pub mod advise;
pub mod bootstrap;
pub mod distill;
pub mod doctor;
pub mod feedback;
pub mod init;
pub mod log;
pub mod pack;
pub mod reflect;
pub mod show;
