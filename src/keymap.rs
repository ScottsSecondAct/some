use std::collections::HashMap;
use crossterm::event::{KeyCode, KeyModifiers};
use crate::config::KeysConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Quit,
    ScrollDown,
    ScrollUp,
    HalfPageDown,
    HalfPageUp,
    FullPageDown,
    FullPageUp,
    GotoTop,
    GotoBottom,
    PrevBuffer,
    NextBuffer,
    SearchForward,
    SearchBackward,
    NextMatch,
    PrevMatch,
    ToggleNumbers,
    ToggleWrap,
    FollowMode,
    EnterCommand,
    Filter,
    Visual,
    SetMark,
    JumpMark,
    ScrollRight,
    ScrollLeft,
}

pub struct KeyMap {
    /// Config-driven primary bindings (user-overridable)
    primary: HashMap<(KeyCode, KeyModifiers), Action>,
    /// Hardcoded secondary aliases (arrows, PageUp/Down, Enter) — never overridden
    secondary: HashMap<(KeyCode, KeyModifiers), Action>,
}

impl KeyMap {
    pub fn build(keys: &KeysConfig) -> Self {
        let mut km = KeyMap {
            primary: Self::defaults(),
            secondary: Self::aliases(),
        };
        km.apply_overrides(keys);
        km
    }

    pub fn get(&self, key: &crossterm::event::KeyEvent) -> Option<Action> {
        let k = (key.code, key.modifiers);
        self.primary.get(&k).or_else(|| self.secondary.get(&k)).copied()
    }

    fn defaults() -> HashMap<(KeyCode, KeyModifiers), Action> {
        use Action::*;
        let mut m = HashMap::new();
        m.insert((KeyCode::Char('q'), KeyModifiers::NONE), Quit);
        m.insert((KeyCode::Char('j'), KeyModifiers::NONE), ScrollDown);
        m.insert((KeyCode::Char('k'), KeyModifiers::NONE), ScrollUp);
        m.insert((KeyCode::Char('d'), KeyModifiers::CONTROL), HalfPageDown);
        m.insert((KeyCode::Char('d'), KeyModifiers::NONE), HalfPageDown);
        m.insert((KeyCode::Char('u'), KeyModifiers::CONTROL), HalfPageUp);
        m.insert((KeyCode::Char('u'), KeyModifiers::NONE), HalfPageUp);
        m.insert((KeyCode::Char(' '), KeyModifiers::NONE), FullPageDown);
        m.insert((KeyCode::Char('b'), KeyModifiers::NONE), FullPageUp);
        m.insert((KeyCode::Char('g'), KeyModifiers::NONE), GotoTop);
        m.insert((KeyCode::Char('G'), KeyModifiers::NONE), GotoBottom);
        m.insert((KeyCode::Char('G'), KeyModifiers::SHIFT), GotoBottom);
        m.insert((KeyCode::Char('['), KeyModifiers::NONE), PrevBuffer);
        m.insert((KeyCode::Char(']'), KeyModifiers::NONE), NextBuffer);
        m.insert((KeyCode::Char('/'), KeyModifiers::NONE), SearchForward);
        m.insert((KeyCode::Char('?'), KeyModifiers::NONE), SearchBackward);
        m.insert((KeyCode::Char('?'), KeyModifiers::SHIFT), SearchBackward);
        m.insert((KeyCode::Char('n'), KeyModifiers::NONE), NextMatch);
        m.insert((KeyCode::Char('N'), KeyModifiers::NONE), PrevMatch);
        m.insert((KeyCode::Char('N'), KeyModifiers::SHIFT), PrevMatch);
        m.insert((KeyCode::Char('l'), KeyModifiers::NONE), ToggleNumbers);
        m.insert((KeyCode::Char('w'), KeyModifiers::NONE), ToggleWrap);
        m.insert((KeyCode::Char('F'), KeyModifiers::NONE), FollowMode);
        m.insert((KeyCode::Char('F'), KeyModifiers::SHIFT), FollowMode);
        m.insert((KeyCode::Char(':'), KeyModifiers::NONE), EnterCommand);
        m.insert((KeyCode::Char(':'), KeyModifiers::SHIFT), EnterCommand);
        m.insert((KeyCode::Char('&'), KeyModifiers::NONE), Filter);
        m.insert((KeyCode::Char('&'), KeyModifiers::SHIFT), Filter);
        m.insert((KeyCode::Char('v'), KeyModifiers::NONE), Visual);
        m.insert((KeyCode::Char('m'), KeyModifiers::NONE), SetMark);
        m.insert((KeyCode::Char('\''), KeyModifiers::NONE), JumpMark);
        m.insert((KeyCode::Right, KeyModifiers::NONE), ScrollRight);
        m.insert((KeyCode::Left, KeyModifiers::NONE), ScrollLeft);
        m
    }

    fn aliases() -> HashMap<(KeyCode, KeyModifiers), Action> {
        use Action::*;
        let mut m = HashMap::new();
        // Arrow keys / page keys / Enter always work regardless of config
        m.insert((KeyCode::Down,     KeyModifiers::NONE), ScrollDown);
        m.insert((KeyCode::Enter,    KeyModifiers::NONE), ScrollDown);
        m.insert((KeyCode::Up,       KeyModifiers::NONE), ScrollUp);
        m.insert((KeyCode::PageDown, KeyModifiers::NONE), FullPageDown);
        m.insert((KeyCode::PageUp,   KeyModifiers::NONE), FullPageUp);
        m.insert((KeyCode::Home,     KeyModifiers::NONE), GotoTop);
        m.insert((KeyCode::End,      KeyModifiers::NONE), GotoBottom);
        // Ctrl+C always quits
        m.insert((KeyCode::Char('c'), KeyModifiers::CONTROL), Quit);
        m
    }

    fn apply_overrides(&mut self, keys: &KeysConfig) {
        let overrides: &[(Option<&String>, Action)] = &[
            (keys.quit.as_ref(), Action::Quit),
            (keys.scroll_down.as_ref(), Action::ScrollDown),
            (keys.scroll_up.as_ref(), Action::ScrollUp),
            (keys.half_page_down.as_ref(), Action::HalfPageDown),
            (keys.half_page_up.as_ref(), Action::HalfPageUp),
            (keys.full_page_down.as_ref(), Action::FullPageDown),
            (keys.full_page_up.as_ref(), Action::FullPageUp),
            (keys.goto_top.as_ref(), Action::GotoTop),
            (keys.goto_bottom.as_ref(), Action::GotoBottom),
            (keys.prev_buffer.as_ref(), Action::PrevBuffer),
            (keys.next_buffer.as_ref(), Action::NextBuffer),
            (keys.search_forward.as_ref(), Action::SearchForward),
            (keys.search_backward.as_ref(), Action::SearchBackward),
            (keys.next_match.as_ref(), Action::NextMatch),
            (keys.prev_match.as_ref(), Action::PrevMatch),
            (keys.toggle_numbers.as_ref(), Action::ToggleNumbers),
            (keys.toggle_wrap.as_ref(), Action::ToggleWrap),
            (keys.follow_mode.as_ref(), Action::FollowMode),
            (keys.enter_command.as_ref(), Action::EnterCommand),
            (keys.filter.as_ref(), Action::Filter),
            (keys.visual.as_ref(), Action::Visual),
            (keys.set_mark.as_ref(), Action::SetMark),
            (keys.jump_mark.as_ref(), Action::JumpMark),
            (keys.scroll_right.as_ref(), Action::ScrollRight),
            (keys.scroll_left.as_ref(), Action::ScrollLeft),
        ];

        for (maybe_spec, action) in overrides {
            if let Some(spec) = maybe_spec {
                if let Some(key) = parse_key_spec(spec) {
                    // Remove any existing primary binding for this action
                    self.primary.retain(|_, v| *v != *action);
                    self.primary.insert(key, *action);
                }
            }
        }
    }
}

pub fn parse_key_spec(s: &str) -> Option<(KeyCode, KeyModifiers)> {
    // Handle ctrl+ prefix
    let lower = s.to_lowercase();
    if let Some(rest) = lower.strip_prefix("ctrl+") {
        let c = rest.chars().next()?;
        return Some((KeyCode::Char(c), KeyModifiers::CONTROL));
    }

    // Named keys (case-insensitive)
    match lower.as_str() {
        "space"           => return Some((KeyCode::Char(' '), KeyModifiers::NONE)),
        "enter" | "return" => return Some((KeyCode::Enter, KeyModifiers::NONE)),
        "tab"             => return Some((KeyCode::Tab, KeyModifiers::NONE)),
        "pagedown" | "pgdn" => return Some((KeyCode::PageDown, KeyModifiers::NONE)),
        "pageup" | "pgup"   => return Some((KeyCode::PageUp, KeyModifiers::NONE)),
        "home"            => return Some((KeyCode::Home, KeyModifiers::NONE)),
        "end"             => return Some((KeyCode::End, KeyModifiers::NONE)),
        "up"              => return Some((KeyCode::Up, KeyModifiers::NONE)),
        "down"            => return Some((KeyCode::Down, KeyModifiers::NONE)),
        "left"            => return Some((KeyCode::Left, KeyModifiers::NONE)),
        "right"           => return Some((KeyCode::Right, KeyModifiers::NONE)),
        "backspace"       => return Some((KeyCode::Backspace, KeyModifiers::NONE)),
        "delete" | "del"  => return Some((KeyCode::Delete, KeyModifiers::NONE)),
        "escape" | "esc"  => return Some((KeyCode::Esc, KeyModifiers::NONE)),
        _ => {}
    }

    // Single character: use as-is (preserve case from original string)
    let c = s.chars().next()?;
    Some((KeyCode::Char(c), KeyModifiers::NONE))
}
