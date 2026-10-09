// Changed by Hookwars: new file (09).
//! The instructions, by area: config, passports and badges, proof levels, the record, the policy
//! wallet and bonds; [`common`] holds the CPI and account helpers.

pub mod admin;
pub mod bonds;
pub mod common;
pub mod passport;
pub mod policy;
pub mod proof;
pub mod record;

pub use admin::*;
pub use bonds::*;
pub use passport::*;
pub use policy::*;
pub use proof::*;
pub use record::*;
