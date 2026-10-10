// Changed by Hookwars: pass 4b: boss, coalition and rivalry errors.
//! Errors (05 section 13, plus the ones the implementation needs).

use anchor_lang::prelude::*;

#[error_code]
pub enum WarError {
    #[msg("the war state of this mint already exists")]
    AlreadyInitialized,
    #[msg("the mint has no War slot")]
    MissingWarSlot,
    #[msg("the War slot is empty, or the orders turn this action off")]
    NoWarOrders,
    #[msg("the item is not the War orders the mint's War slot names")]
    WrongWarOrders,
    #[msg("an account the step needs was not passed")]
    MissingAccount,
    #[msg("nothing to do")]
    NothingToDo,
    #[msg("the chest cannot pay this")]
    ChestInsufficient,
    #[msg("a token cannot besiege itself")]
    SelfSiege,
    #[msg("the rival's kit has holder rewards on: a war chest cannot hold it")]
    SiegeTargetHasRewards,
    #[msg("this token's kit has holder rewards on: its chest cannot hold it")]
    OwnTokenHasRewards,
    #[msg("the siege is not due")]
    SiegeNotDue,
    #[msg("the captured table is full")]
    CapturedTableFull,
    #[msg("the counter-strike is not due")]
    CounterNotDue,
    #[msg("the observations do not cover the window")]
    NoObservations,
    #[msg("the War orders do not allow razing")]
    RazeDisabled,
    #[msg("this window's raze allowance is used")]
    RazeLimit,
    #[msg("the treaty does not return captured holdings")]
    PeaceReturnsOff,
    #[msg("the treaty is not equipped by both tokens")]
    NoTreaty,
    #[msg("the touch did not change the holding as asked")]
    TouchMismatch,
    #[msg("the randomness is not ready")]
    RandomnessNotReady,
    #[msg("the randomness account is not this roll's")]
    WrongRandomness,
    #[msg("the roll has not expired")]
    RollNotExpired,
    #[msg("the quest was already claimed this period")]
    QuestAlreadyClaimed,
    #[msg("the quest's condition is not met")]
    QuestConditionUnmet,
    #[msg("no season is open")]
    SeasonNotOpen,
    #[msg("the season has not ended")]
    SeasonNotEnded,
    #[msg("the challenge window is closed")]
    ChallengeClosed,
    #[msg("the score is not higher than the leader's")]
    NotHigherScore,
    #[msg("the season is not finalized")]
    SeasonNotFinalized,
    #[msg("the timelock has not passed")]
    TimelockNotPassed,
    #[msg("only the admin may do this")]
    NotAdmin,
    #[msg("arithmetic overflow")]
    MathOverflow,
    #[msg("only the program's upgrade authority may create the config")]
    NotUpgradeAuthority,
    #[msg("invalid parameters")]
    InvalidParams,
    #[msg("a foreign account does not decode as the spec's layout")]
    WrongForeignAccount,
    #[msg("the pool gives no quote")]
    NoQuote,
    #[msg("the launch is not this mint's")]
    WrongLaunch,
    #[msg("the slot is not this program's Raid slot")]
    WrongRaidSlot,
    #[msg("nothing is pending")]
    NothingPending,
    #[msg("the season is already open or finalized")]
    SeasonClosed,
    #[msg("the season's loot table is missing or not yet in force")]
    LootTableNotReady,
    #[msg("a loot entry is outside its template")]
    InvalidLootEntry,
    #[msg("the treaty template is not configured")]
    NoTreatyTemplate,
    #[msg("the account passed is not the one expected")]
    WrongAccount,
    // Pass 4b (10 sections 8, 11.1, 11.3).
    #[msg("the token does not equip that item")]
    ItemNotEquipped,
    #[msg("the item is not of the template this step reads")]
    WrongItem,
    #[msg("the boss pool is sealed or not yet sealable")]
    BossPoolState,
    #[msg("this source has no share or has claimed it")]
    NoBossShare,
    #[msg("the coalition's members are invalid")]
    InvalidCoalition,
    #[msg("the coalition has ended or is dissolved")]
    CoalitionClosed,
    #[msg("the coalition has not ended yet, or still holds captured tokens")]
    CoalitionNotDone,
    #[msg("the contribution is above the item's cap or inside its interval")]
    ContributionLimit,
    #[msg("the rivalry is invalid, not live, or already open")]
    RivalryState,
}
