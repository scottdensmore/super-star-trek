//! sst-tuikit: a faithful Rust re-implementation of the Charmbracelet
//! subset used by Super Star Trek's TUI, so that the `pkg/tui` port can be
//! translated nearly line by line and produce byte-identical views.
//!
//! | Go package | module |
//! |---|---|
//! | `github.com/charmbracelet/lipgloss` v1.1.0 | [`lipgloss`] |
//! | `github.com/muesli/termenv` v0.16.0 | [`termenv`] |
//! | `github.com/charmbracelet/x/ansi` v0.11.9 | [`ansi`] |
//! | `github.com/charmbracelet/x/cellbuf` (Wrap) | [`cellbuf`] |
//! | `github.com/charmbracelet/x/term` | [`term`] |
//! | `github.com/charmbracelet/bubbletea` v1.3.10 | [`tea`] |
//! | `github.com/charmbracelet/bubbles` v1.0.0 | [`bubbles`] |
//! | `github.com/sahilm/fuzzy` v0.1.1 | [`fuzzy`] |
//! | `github.com/lucasb-eyer/go-colorful` (subset) | [`colorful`] |
//! | width libraries (displaywidth, uniseg, runewidth) | [`unicode`] |

pub mod ansi;
pub mod bubbles;
pub mod cellbuf;
pub mod colorful;
pub mod fuzzy;
pub mod gomath;
pub mod gosort;
pub mod lipgloss;
pub mod tea;
pub mod term;
pub mod termenv;
pub mod unicode;
