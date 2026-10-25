#[derive(Clone, Copy)]
pub enum LaunchMode {
    Normal,
    Background,
    Show,
}

impl LaunchMode {
    pub fn activates_existing(self) -> bool {
        !matches!(self, Self::Background)
    }

    pub fn initially_hidden(self, start_minimized: bool, has_error: bool) -> bool {
        !has_error
            && match self {
                Self::Normal => start_minimized,
                Self::Background => true,
                Self::Show => false,
            }
    }
}

#[cfg(test)]
mod tests {
    use super::LaunchMode::*;

    #[test]
    fn explicit_show_overrides_tray_preference_and_background_ignores_it() {
        for minimized in [false, true] {
            assert!(!Show.initially_hidden(minimized, false));
            assert!(Background.initially_hidden(minimized, false));
            assert_eq!(Normal.initially_hidden(minimized, false), minimized);
        }
    }

    #[test]
    fn configuration_errors_remain_visible_in_every_launch_mode() {
        for mode in [Normal, Background, Show] {
            assert!(!mode.initially_hidden(true, true));
        }
    }
}
