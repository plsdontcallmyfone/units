//! The instructions, by area: the config and timelock, setup and funding, attack and defense,
//! treaties, bounties, loot, quests and seasons.

pub mod admin;
pub mod attack;
pub mod bounty;
pub mod loot;
pub mod quests;
pub mod seasons;
pub mod setup;
pub mod treaty;

pub use admin::*;
pub use attack::*;
pub use bounty::*;
pub use loot::*;
pub use quests::*;
pub use seasons::*;
pub use setup::*;
pub use treaty::*;
