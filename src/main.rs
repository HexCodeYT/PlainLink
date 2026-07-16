use plainlink::{
    RuleSet, WatchOptions, clean_url, read_last_cleaned, watch_clipboard, write_clipboard_text,
};
use std::{env, process, time::Duration};

fn main() {
    if let Err(error) = run() {
        eprintln!("plainlink: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next();

    match command.as_deref() {
        None | Some("-h") | Some("--help") | Some("help") => {
            print_help();
            Ok(())
        }
        Some("-V") | Some("--version") | Some("version") => {
            println!("plainlink {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("clean") => clean_command(args.collect()),
        Some("inspect") => inspect_command(args.collect()),
        Some("restore") => restore_command(),
        Some("watch") => watch_command(args.collect()),
        Some(other) => Err(format!("unknown command `{other}`. Try `plainlink help`.")),
    }
}

fn clean_command(args: Vec<String>) -> Result<(), String> {
    let input = positional_text(args)?;
    let rules = RuleSet::default_rules();
    let result = clean_url(&input, &rules);

    println!("{}", result.cleaned);
    Ok(())
}

fn inspect_command(args: Vec<String>) -> Result<(), String> {
    let input = positional_text(args)?;
    let rules = RuleSet::default_rules();
    let result = clean_url(&input, &rules);

    println!("Original: {}", result.original.trim());
    println!("Cleaned:  {}", result.cleaned);

    if result.removed.is_empty() {
        println!("Removed:  none");
    } else {
        println!("Removed:");
        for removed in result.removed {
            let value = removed.value.unwrap_or_else(|| "<none>".to_string());
            println!("  - {}={} ({})", removed.name, value, removed.reason);
        }
    }

    Ok(())
}

fn watch_command(args: Vec<String>) -> Result<(), String> {
    let options = parse_watch_options(args)?;
    let rules = RuleSet::default_rules();

    println!(
        "PlainLink is watching the macOS clipboard every {}ms. Press Ctrl-C to stop.",
        options.interval.as_millis()
    );

    watch_clipboard(&rules, options).map_err(|error| error.to_string())
}

fn restore_command() -> Result<(), String> {
    let last_cleaned = read_last_cleaned().map_err(|error| {
        format!("could not read last cleaned URL: {error}. Run `plainlink watch` first.")
    })?;

    write_clipboard_text(&last_cleaned.original)
        .map_err(|error| format!("could not restore original URL to clipboard: {error}"))?;

    println!("Restored original URL to clipboard:");
    println!("{}", last_cleaned.original.trim());

    Ok(())
}

fn parse_watch_options(args: Vec<String>) -> Result<WatchOptions, String> {
    let mut options = WatchOptions::default();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--clean-current" => {
                options.clean_current = true;
                index += 1;
            }
            "--interval-ms" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("--interval-ms needs a number".to_string());
                };
                let milliseconds = value
                    .parse::<u64>()
                    .map_err(|_| "--interval-ms needs a positive integer".to_string())?;

                if milliseconds < 100 {
                    return Err("--interval-ms must be at least 100".to_string());
                }

                options.interval = Duration::from_millis(milliseconds);
                index += 2;
            }
            other => return Err(format!("unknown watch option `{other}`")),
        }
    }

    Ok(options)
}

fn positional_text(args: Vec<String>) -> Result<String, String> {
    if args.is_empty() {
        return Err("missing URL".to_string());
    }

    Ok(args.join(" "))
}

fn print_help() {
    println!(
        r#"PlainLink - clean copied links before you share them.

Usage:
  plainlink clean <url>       Print a cleaned URL
  plainlink inspect <url>     Show what PlainLink removed and why
  plainlink restore           Restore the last cleaned URL to the clipboard
  plainlink watch [options]   Watch and clean the macOS clipboard

Watch options:
  --interval-ms <ms>          Polling interval, default 500
  --clean-current             Clean the current clipboard once at startup

Examples:
  plainlink clean 'https://youtu.be/LYa_ReqRlcs?si=VC4qVB_EUC90uwbo'
  plainlink inspect 'https://example.com/?utm_source=newsletter&id=42'
  plainlink watch --interval-ms 500
"#
    );
}
