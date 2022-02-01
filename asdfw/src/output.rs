use std::fmt::Display;

use anyhow::Error;
use textwrap::{wrap, Options};
use yansi::Paint;

pub fn print_out<T: Display>(lines: Vec<T>) {
    for l in lines.iter() {
        println!("{}", l);
    }
}

pub fn output_full_error(err: Error, width: Option<usize>) -> Vec<String> {
    let width = match width {
        Some(n) => n,
        None => textwrap::termwidth() - 4,
    };
    let main_prefix = format!(" {}  ", Paint::red(""));
    let causes_prefix = format!("   {}  ", Paint::red("-"));
    let main_options = Options::new(width)
        .initial_indent(&main_prefix)
        .subsequent_indent("    ");
    let causes_options = Options::new(width)
        .initial_indent(&causes_prefix)
        .subsequent_indent("      ");
    let main_msg = format!("{}", err);

    let mut output: Vec<String> = wrap(&main_msg, main_options)
        .iter()
        .map(|s| s.clone().into_owned())
        .collect();

    let causes = err.chain().skip(1);
    if causes.len() > 0 {
        let mut caused_by = vec!["".to_owned(), " Caused by:".to_owned()];
        output.append(&mut caused_by);
    };
    causes.for_each(|cause| {
        let msg = format!("{}", cause);
        for line in wrap(&msg, &causes_options) {
            output.push(line.into_owned());
        }
    });

    output
}

pub fn success_message(msg: &str) -> Vec<std::borrow::Cow<str>> {
    let prefix = format!(" {}  ", Paint::green(""));
    let options = Options::new(textwrap::termwidth() - 4)
        .initial_indent(&prefix)
        .subsequent_indent("    ");
    wrap(msg, &options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{anyhow, Context, Result};

    #[test]
    fn test_output_full_error_with_nested_error() {
        let expected = [
            " \u{1b}[31m\u{1b}[0m  This is an error description that should span",
            "    over several lines",
            " Caused by:",
            "",
            "   \u{1b}[31m-\u{1b}[0m  The first cause",
            "   \u{1b}[31m-\u{1b}[0m  The most nested cause. Should also span over",
            "      multiple lines hopefully.",
        ];
        let err1: Result<()> = Err(anyhow!(
            "The most nested cause. Should also span over multiple lines hopefully."
        ));
        let err2 = err1.context("The first cause").unwrap_err();
        let err3 = err2.context("This is an error description that should span over several lines");
        let result = output_full_error(err3, Some(50));
        assert_eq!(result, expected)
    }

    #[test]
    fn test_output_full_error_with_simple_error() {
        let expected = [
            " \u{1b}[31m\u{1b}[0m  This is an error description that should span",
            "    over several lines",
        ];
        let err = anyhow!("This is an error description that should span over several lines");
        let result = output_full_error(err, Some(50));
        assert_eq!(result, expected)
    }
}
