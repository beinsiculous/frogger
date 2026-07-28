//! Frogger achievement definitions.
//!
//! Registered once in `init()` with names/descriptions from the locale
//! tables, and re-registered on locale switches (id-keyed insert — unlock
//! state survives). Home/score achievements unlock from `fill_home`, round
//! achievements from `clear_round` (see `gameplay/flow`).

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

/// Every achievement id, in registration order.
pub(crate) const ALL_IDS: [&str; 8] = [
    FIRST_HOME, ROUND_CLEAR, HOMES_25,
    DEATHLESS_ROUND, SPEEDY, SCORE_5K,
    INSICULOUS_CLEAR, COOP_ROUND,
];

/// Grouped display order for the achievements page. First tuple element is
/// the section-header locale key, second is the list of ids under it.
pub(crate) const DISPLAY_SECTIONS: &[(&str, &[&str])] = &[
    ("ach.section.crossings", &[FIRST_HOME, ROUND_CLEAR, HOMES_25]),
    ("ach.section.skill", &[DEATHLESS_ROUND, SPEEDY, SCORE_5K]),
    ("ach.section.chaos_coop", &[INSICULOUS_CLEAR, COOP_ROUND]),
];

/// Register every Frogger achievement with names/descriptions from the
/// locale tables (`ach.<id>.name` / `ach.<id>.desc`). Called from
/// `Game::init` AND again after a locale switch — `register` is an id-keyed
/// insert, so re-registering refreshes the display strings without touching
/// unlock state.
pub(crate) fn register_all(mgr: &mut AchievementManager, strings: &Strings) {
    for id in ALL_IDS {
        let name_key = format!("ach.{id}.name");
        let desc_key = format!("ach.{id}.desc");
        mgr.register(Achievement::new(
            id,
            strings.tr(&name_key).to_string(),
            strings.tr(&desc_key).to_string(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The game's real locale tables, loaded from assets/locales.
    fn real_strings() -> Strings {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/locales");
        Strings::load_dir(&dir)
    }

    #[test]
    fn register_all_adds_eight() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr, &Strings::empty());
        assert_eq!(mgr.total(), 8);
    }

    #[test]
    fn register_all_uses_locale_names_and_rereg_keeps_unlocks() {
        let mut strings = real_strings();
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr, &strings);
        assert_eq!(mgr.get(FIRST_HOME).unwrap().name, "Pond Pioneer");

        mgr.unlock(FIRST_HOME);
        assert!(mgr.is_unlocked(FIRST_HOME));

        // Switch locale, re-register: names refresh, unlock state survives.
        strings.set_locale("pirate");
        register_all(&mut mgr, &strings);
        assert_eq!(mgr.get(FIRST_HOME).unwrap().name, "First Safe Harbor");
        assert!(mgr.is_unlocked(FIRST_HOME), "re-registering must not reset unlocks");
    }

    #[test]
    fn locale_files_have_matching_keys() {
        let strings = real_strings();
        let en = strings.locale_keys("en").expect("en.ron loads");
        let pirate = strings.locale_keys("pirate").expect("pirate.ron loads");
        assert!(!en.is_empty(), "en locale must define keys");
        assert_eq!(en, pirate, "en.ron and pirate.ron must define the same key set");
    }

    #[test]
    fn every_achievement_id_has_name_and_desc_keys() {
        let strings = real_strings();
        let en = strings.locale_keys("en").expect("en.ron loads");
        for id in ALL_IDS {
            let name_key = format!("ach.{id}.name");
            let desc_key = format!("ach.{id}.desc");
            assert!(en.contains(&name_key.as_str()), "{name_key} missing from en.ron");
            assert!(en.contains(&desc_key.as_str()), "{desc_key} missing from en.ron");
        }
    }

    #[test]
    fn display_sections_cover_every_registered_achievement() {
        let mut mgr = AchievementManager::in_memory();
        register_all(&mut mgr, &Strings::empty());

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
    fn section_headers_have_locale_keys() {
        let strings = real_strings();
        let en = strings.locale_keys("en").expect("en.ron loads");
        for (section_key, _) in DISPLAY_SECTIONS {
            assert!(en.contains(section_key), "{section_key} missing from en.ron");
        }
    }
}
