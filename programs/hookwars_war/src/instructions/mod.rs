// Changed by Hookwars: pass 4b: boss, coalition, rivalry.
//! The instructions, by area: the config and timelock, setup and funding, attack and defense,
//! treaties, bounties, loot, quests and seasons.

pub mod admin;
pub mod attack;
pub mod boss;
pub mod bounty;
pub mod coalition;
pub mod loot;
pub mod quests;
pub mod rivalry;
pub mod seasons;
pub mod setup;
pub mod treaty;

pub use admin::*;
pub use attack::*;
pub use boss::*;
pub use bounty::*;
pub use coalition::*;
pub use loot::*;
pub use quests::*;
pub use rivalry::*;
pub use seasons::*;
pub use setup::*;
pub use treaty::*;
