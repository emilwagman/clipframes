//! cf-read-rule: the rule that lets Claude Code read a captures folder, and the change the
//! Settings switch makes, against a settings file of your choosing. For checking the rule
//! with a real Claude Code session without touching your own settings.
//!
//!   cf-read-rule ~/Clipframes                       print the rule
//!   cf-read-rule ~/Clipframes on  /tmp/x/settings.json   add it to that file
//!   cf-read-rule ~/Clipframes off /tmp/x/settings.json   take it away again

use clipframes_lib::claude;
use std::{env, path::Path, process::exit};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let home = env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default();
    let rules = args.first().map(|folder| claude::rules(Path::new(folder), Path::new(&home))).unwrap_or_default();
    if rules.is_empty() {
        eprintln!("Give a captures folder as a full path.");
        exit(2);
    }
    match (args.get(1).map(String::as_str), args.get(2)) {
        (None, _) => println!("{}", rules.join("\n")),
        (Some(turn @ ("on" | "off")), Some(file)) => println!("{}\n{:?}", rules.join("\n"), claude::set(Path::new(file), &rules, turn == "on")),
        _ => {
            eprintln!("cf-read-rule <folder> [on|off <settings file>]");
            exit(2);
        }
    }
}
