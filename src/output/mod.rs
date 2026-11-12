pub mod diagnostic;
mod status;
pub mod terminal;

pub use status::{clear_progress, configure, is_quiet, progress, warning};

pub use diagnostic::{
    print_explanation, print_explanation_deterministic, print_explanation_footer,
    print_explanation_language,
};
pub use terminal::{print_config, print_scan, print_search, print_search_language};
