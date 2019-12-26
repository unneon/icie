use crate::tutorial::Skill;

pub mod manage;
pub mod render;

/// If the test view contains any failed tests, it will scroll the view so that the failure are
/// visible. This will try to scroll so that the first failing test is as high on the screen as
/// possible.
#[evscode::config]
static SCROLL_TO_FIRST_FAILED: evscode::Config<bool> = true;

const SKILL_ACTIONS: Skill = Skill::new("skill.actions", 4);
