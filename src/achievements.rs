//! Frogger achievement definitions.
//!
//! Registered once in `init()`. Home/score achievements unlock from
//! `fill_home`, round achievements from `clear_round` (see `gameplay/flow`).

use engine_core::prelude::*;

/// IDs — kept as `&'static str` so the compiler catches typos at call sites.
pub(crate) const FIRST_HOME: &str = "first_home";
pub(crate) const ROUND_CLEAR: &str = "round_clear";
pub(crate) const HOMES_25: &str = "homes_25";

pub(crate) const DEATHLESS_ROUND: &str = "deathless_round";
pub(crate) const SPEEDY: &str = "speedy";
pub(crate) const SCORE_5K: &str = "score_5k";

pub(crate) const INSICULOUS_CLEAR: &str = "insiculous_clear";
pub(crate) const COOP_ROUND: &str = "coop_round";

/// Grouped display order for the achievements page. First tuple element is
/// the section header, second is the list of ids to render under it.
pub(crate) const DISPLAY_SECTIONS: &[(&str, &[&str])] = &[
    ("Crossings", &[FIRST_HOME, ROUND_CLEAR, HOMES_25]),
    ("Skill", &[DEATHLESS_ROUND, SPEEDY, SCORE_5K]),
    ("Chaos & Co-op", &[INSICULOUS_CLEAR, COOP_ROUND]),
];

/// Register every Frogger achievement. Call once from `Game::init`.
pub(crate) fn register_all(mgr: &mut AchievementManager) {
    mgr.register(Achievement::new(FIRST_HOME,
        "Pond Pioneer",
        "Guide a frog into its first home."));
    mgr.register(Achievement::new(ROUND_CLEAR,
        "Full House",
        "Fill all five homes in one round."));
    mgr.register(Achievement::new(HOMES_25,
        "Homecoming Hero",
        "Fill 25 homes in one session."));

    mgr.register(Achievement::new(DEATHLESS_ROUND,
        "Untouched Crossing",
        "Clear a round without losing a single life."));
    mgr.register(Achievement::new(SPEEDY,
        "Swift Swimmer",
        "Reach home with 15 or more seconds to spare."));
    mgr.register(Achievement::new(SCORE_5K,
        "High Hopper",
        "Score 5,000 points."));

    mgr.register(Achievement::new(INSICULOUS_CLEAR,
        "Insiculous Crossing",
        "Clear a round in Insiculous mode."));
    mgr.register(Achievement::new(COOP_ROUND,
        "Tag Team",
        "In co-op, both players fill a home in the same round."));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_all_adds_eight() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr);
        assert_eq!(mgr.total(), 8);
    }

    #[test]
    fn display_sections_cover_every_registered_achievement() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr);

        let shown: std::collections::HashSet<&str> = DISPLAY_SECTIONS
            .iter()
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect();

        for ach in mgr.all() {
            assert!(
                shown.contains(ach.id.as_str()),
                "{} registered but not in DISPLAY_SECTIONS",
                ach.id
            );
        }
        assert_eq!(shown.len(), mgr.total(), "DISPLAY_SECTIONS has duplicates or extras");
    }

    #[test]
    fn every_id_is_registered() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr);
        for id in [
            FIRST_HOME, ROUND_CLEAR, HOMES_25,
            DEATHLESS_ROUND, SPEEDY, SCORE_5K,
            INSICULOUS_CLEAR, COOP_ROUND,
        ] {
            assert!(mgr.get(id).is_some(), "{} not registered", id);
        }
    }
}
