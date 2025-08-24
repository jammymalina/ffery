use indicatif::{ProgressBar, ProgressStyle};

pub fn get_progress_bar(count: u64) -> ProgressBar {
    let progress_bar = ProgressBar::new(count);

    progress_bar.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )
        .unwrap(),
    );

    progress_bar
}
