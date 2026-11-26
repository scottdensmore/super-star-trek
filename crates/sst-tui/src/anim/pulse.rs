//! Red alert badge pulse (port of `pkg/tui/anim/pulse.go`).

use sst_tuikit::lipgloss::{self, Style};

/// `RedAlertBadgeStyle`: bright bold crimson on even cycles, dimmed red on odd.
pub fn red_alert_badge_style(cycle: i32) -> Style {
    if cycle % 2 == 0 {
        return lipgloss::new_style()
            .bold(true)
            .foreground(lipgloss::color("#FF1744"));
    }
    lipgloss::new_style().foreground(lipgloss::color("#881B2B"))
}
