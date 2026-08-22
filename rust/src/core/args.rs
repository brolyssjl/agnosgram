//! Port of `src/core/args.ts`: shared CLI argument parsing. Every command
//! routes through `parse_cli_args` instead of a raw arg loop. It fixes Node
//! `parseArgs`'s "ambiguous option value" guard, which raw-crashes on any
//! dash-leading option value (`--budget -1`, `--learned "--x"`), and turns
//! every remaining parse failure into a clean `UserError` instead of a stack
//! trace - the same contract `src/core/args.ts` gives the TS commands.
//!
//! There is no `node:util.parseArgs` in std Rust, so this module also
//! contains a hand-rolled strict parser that reproduces the subset of its
//! behavior the frozen CLI surface actually exercises (long options,
//! `--opt=value`, short aliases, boolean vs string types, `allowPositionals`,
//! the `--` terminator). Error text is the first line of the corresponding
//! Node `parseArgs` error, verified empirically against Node 20/22 (see
//! `docs/rust-port.md`).

use std::collections::{HashMap, HashSet};

use super::output::UserError;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OptType {
    Boolean,
    Str,
}

/// One option's shape: type, optional short alias, optional default.
#[derive(Clone, Debug)]
pub struct OptionDef {
    pub kind: OptType,
    pub short: Option<char>,
    pub bool_default: Option<bool>,
    pub str_default: Option<String>,
}

impl OptionDef {
    /// `{ type: "boolean", default: <default> }` - every command in the
    /// frozen surface declares boolean options with a `false` default.
    pub fn boolean(default: bool) -> Self {
        OptionDef {
            kind: OptType::Boolean,
            short: None,
            bool_default: Some(default),
            str_default: None,
        }
    }

    /// `{ type: "string" }`.
    pub fn string() -> Self {
        OptionDef {
            kind: OptType::Str,
            short: None,
            bool_default: None,
            str_default: None,
        }
    }

    #[cfg(test)]
    pub fn with_short(mut self, c: char) -> Self {
        self.short = Some(c);
        self
    }
}

/// `parseArgs`'s `{ args, options, allowPositionals }` config.
pub struct ArgsConfig {
    pub options: Vec<(String, OptionDef)>,
    pub allow_positionals: bool,
}

impl ArgsConfig {
    pub fn new(allow_positionals: bool) -> Self {
        ArgsConfig {
            options: Vec::new(),
            allow_positionals,
        }
    }

    pub fn option(mut self, name: impl Into<String>, def: OptionDef) -> Self {
        self.options.push((name.into(), def));
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ArgValue {
    Bool(bool),
    Str(String),
}

/// The result of `parse_cli_args`: resolved option values plus positionals.
#[derive(Debug, Default)]
pub struct ParsedArgs {
    values: HashMap<String, ArgValue>,
    pub positionals: Vec<String>,
}

impl ParsedArgs {
    pub fn bool(&self, name: &str) -> bool {
        matches!(self.values.get(name), Some(ArgValue::Bool(true)))
    }

    pub fn str(&self, name: &str) -> Option<&str> {
        match self.values.get(name) {
            Some(ArgValue::Str(s)) => Some(s.as_str()),
            _ => None,
        }
    }
}

fn known_flags(options: &[(String, OptionDef)]) -> HashSet<String> {
    let mut flags = HashSet::new();
    for (name, spec) in options {
        flags.insert(format!("--{name}"));
        if let Some(c) = spec.short {
            flags.insert(format!("-{c}"));
        }
    }
    flags
}

fn find_by_name<'a>(
    options: &'a [(String, OptionDef)],
    name: &str,
) -> Option<&'a (String, OptionDef)> {
    options.iter().find(|(n, _)| n == name)
}

fn find_by_short(options: &[(String, OptionDef)], c: char) -> Option<&(String, OptionDef)> {
    options.iter().find(|(_, s)| s.short == Some(c))
}

fn find_by_name_or_short<'a>(
    options: &'a [(String, OptionDef)],
    name: &str,
) -> Option<&'a OptionDef> {
    options
        .iter()
        .find(|(n, s)| n == name || s.short.map(|c| c.to_string()).as_deref() == Some(name))
        .map(|(_, s)| s)
}

/// Node's `parseArgs` throws `ERR_PARSE_ARGS_INVALID_OPTION_VALUE` whenever a
/// string option's value starts with a dash. Rewriting `--opt VALUE` to
/// `--opt=VALUE` up front sidesteps the guard, except when the next token is
/// itself a recognized flag (a genuinely missing value) - that case is left
/// alone so it still surfaces as an error below. Ported 1:1 from
/// `disambiguate` in `src/core/args.ts`.
fn disambiguate(args: &[String], options: &[(String, OptionDef)]) -> Vec<String> {
    let flags = known_flags(options);
    let mut out: Vec<String> = Vec::with_capacity(args.len());
    let mut i = 0usize;
    while i < args.len() {
        let tok = &args[i];
        if tok == "--" {
            out.extend(args[i..].iter().cloned());
            break;
        }
        if tok.contains('=') || !tok.starts_with('-') {
            out.push(tok.clone());
            i += 1;
            continue;
        }
        let name: Option<&str> = if let Some(rest) = tok.strip_prefix("--") {
            Some(rest)
        } else if tok.chars().count() == 2 {
            Some(&tok[1..])
        } else {
            None
        };
        let spec = name.and_then(|n| find_by_name_or_short(options, n));
        let next = args.get(i + 1);
        let is_string = matches!(spec.map(|s| s.kind), Some(OptType::Str));
        if is_string {
            if let Some(next) = next {
                if next != "--"
                    && next.starts_with('-')
                    && next != "-"
                    && !flags.contains(next.as_str())
                {
                    let joined = if tok.starts_with("--") {
                        format!("{tok}={next}")
                    } else {
                        format!("{tok}{next}")
                    };
                    out.push(joined);
                    i += 2;
                    continue;
                }
            }
        }
        out.push(tok.clone());
        i += 1;
    }
    out
}

fn display_form(name: &str, spec: &OptionDef) -> String {
    match spec.short {
        Some(c) => format!("-{c}, --{name}"),
        None => format!("--{name}"),
    }
}

enum NextOutcome {
    Consume(String),
    Missing,
    Ambiguous,
}

/// By construction this is only called on tokens that already went through
/// `disambiguate`: any dash-leading next token that survived unmerged must be
/// either the `--` terminator or a genuinely known flag - both cases Node
/// refuses to silently swallow as a value.
fn resolve_next(tokens: &[String], idx: usize) -> NextOutcome {
    match tokens.get(idx) {
        None => NextOutcome::Missing,
        Some(next) if next == "-" => NextOutcome::Consume(next.clone()),
        Some(next) if next.starts_with('-') => NextOutcome::Ambiguous,
        Some(next) => NextOutcome::Consume(next.clone()),
    }
}

fn strict_parse(
    tokens: &[String],
    options: &[(String, OptionDef)],
    allow_positionals: bool,
) -> Result<ParsedArgs, UserError> {
    let mut values: HashMap<String, ArgValue> = HashMap::new();
    let mut positionals: Vec<String> = Vec::new();
    let mut i = 0usize;

    while i < tokens.len() {
        let tok = &tokens[i];

        if tok == "--" {
            if allow_positionals {
                positionals.extend(tokens[i + 1..].iter().cloned());
                break;
            }
            if let Some(first) = tokens.get(i + 1) {
                return Err(UserError::new(format!(
                    "Unexpected argument '{first}'. This command does not take positional arguments"
                )));
            }
            i += 1;
            continue;
        }

        if tok == "-" {
            if allow_positionals {
                positionals.push(tok.clone());
                i += 1;
                continue;
            }
            return Err(UserError::new(
                "Unexpected argument '-'. This command does not take positional arguments"
                    .to_string(),
            ));
        }

        if let Some(rest) = tok.strip_prefix("--") {
            let (name, attached) = match rest.split_once('=') {
                Some((n, v)) => (n.to_string(), Some(v.to_string())),
                None => (rest.to_string(), None),
            };
            let Some((opt_name, spec)) = find_by_name(options, &name) else {
                return Err(UserError::new(format!("Unknown option '--{name}'")));
            };
            match spec.kind {
                OptType::Boolean => {
                    if attached.is_some() {
                        return Err(UserError::new(format!(
                            "Option '{}' does not take an argument",
                            display_form(opt_name, spec)
                        )));
                    }
                    values.insert(opt_name.clone(), ArgValue::Bool(true));
                    i += 1;
                }
                OptType::Str => {
                    if let Some(v) = attached {
                        values.insert(opt_name.clone(), ArgValue::Str(v));
                        i += 1;
                    } else {
                        match resolve_next(tokens, i + 1) {
                            NextOutcome::Consume(v) => {
                                values.insert(opt_name.clone(), ArgValue::Str(v));
                                i += 2;
                            }
                            NextOutcome::Missing => {
                                return Err(UserError::new(format!(
                                    "Option '{} <value>' argument missing",
                                    display_form(opt_name, spec)
                                )));
                            }
                            NextOutcome::Ambiguous => {
                                return Err(UserError::new(format!(
                                    "Option '--{name}' argument is ambiguous."
                                )));
                            }
                        }
                    }
                }
            }
            continue;
        }

        if let Some(rest) = tok.strip_prefix('-') {
            let chars: Vec<char> = rest.chars().collect();
            let mut idx = 0usize;
            let mut consumed_extra = false;
            while idx < chars.len() {
                let c = chars[idx];
                let Some((opt_name, spec)) = find_by_short(options, c) else {
                    if idx == 0 && c.is_ascii_digit() && allow_positionals {
                        return Err(UserError::new(format!(
                            "Unknown option '{tok}'. To specify a positional argument starting with a '-', \
                             place it at the end of the command after '--', as in '-- \"{tok}\"'"
                        )));
                    }
                    return Err(UserError::new(format!("Unknown option '-{c}'")));
                };
                match spec.kind {
                    OptType::Boolean => {
                        values.insert(opt_name.clone(), ArgValue::Bool(true));
                        idx += 1;
                    }
                    OptType::Str => {
                        let remainder: String = chars[idx + 1..].iter().collect();
                        if !remainder.is_empty() {
                            values.insert(opt_name.clone(), ArgValue::Str(remainder));
                        } else {
                            match resolve_next(tokens, i + 1) {
                                NextOutcome::Consume(v) => {
                                    values.insert(opt_name.clone(), ArgValue::Str(v));
                                    consumed_extra = true;
                                }
                                NextOutcome::Missing => {
                                    return Err(UserError::new(format!(
                                        "Option '{} <value>' argument missing",
                                        display_form(opt_name, spec)
                                    )));
                                }
                                NextOutcome::Ambiguous => {
                                    return Err(UserError::new(format!(
                                        "Option '-{c}' argument is ambiguous."
                                    )));
                                }
                            }
                        }
                        idx = chars.len();
                    }
                }
            }
            i += if consumed_extra { 2 } else { 1 };
            continue;
        }

        if allow_positionals {
            positionals.push(tok.clone());
            i += 1;
        } else {
            return Err(UserError::new(format!(
                "Unexpected argument '{tok}'. This command does not take positional arguments"
            )));
        }
    }

    for (name, spec) in options {
        if values.contains_key(name) {
            continue;
        }
        match spec.kind {
            OptType::Boolean => {
                if let Some(d) = spec.bool_default {
                    values.insert(name.clone(), ArgValue::Bool(d));
                }
            }
            OptType::Str => {
                if let Some(d) = &spec.str_default {
                    values.insert(name.clone(), ArgValue::Str(d.clone()));
                }
            }
        }
    }

    Ok(ParsedArgs {
        values,
        positionals,
    })
}

/// Port of `parseCliArgs`: disambiguate, then strict-parse, turning every
/// parse failure into a `UserError` carrying the first line of the message.
pub fn parse_cli_args(args: &[String], config: &ArgsConfig) -> Result<ParsedArgs, UserError> {
    let rewritten = disambiguate(args, &config.options);
    strict_parse(&rewritten, &config.options, config.allow_positionals)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn string_opt_config(allow_positionals: bool) -> ArgsConfig {
        ArgsConfig::new(allow_positionals).option("budget", OptionDef::string())
    }

    #[test]
    fn dash_leading_negative_number_is_accepted_as_string_value() {
        let cfg = string_opt_config(false);
        let parsed = parse_cli_args(&args(&["--budget", "-1"]), &cfg).unwrap();
        assert_eq!(parsed.str("budget"), Some("-1"));
    }

    #[test]
    fn literal_value_starting_with_dashdash_is_accepted_verbatim() {
        let cfg = ArgsConfig::new(false).option("learned", OptionDef::string());
        let parsed = parse_cli_args(&args(&["--learned", "--x"]), &cfg).unwrap();
        assert_eq!(parsed.str("learned"), Some("--x"));
    }

    #[test]
    fn already_disambiguated_eq_form_still_works() {
        let cfg = string_opt_config(false);
        let parsed = parse_cli_args(&args(&["--budget=-1"]), &cfg).unwrap();
        assert_eq!(parsed.str("budget"), Some("-1"));
    }

    #[test]
    fn genuinely_missing_value_raises_clean_user_error() {
        let cfg = ArgsConfig::new(false)
            .option("scope", OptionDef::string())
            .option("budget", OptionDef::string());
        let err = parse_cli_args(&args(&["--scope", "--budget", "500"]), &cfg).unwrap_err();
        assert!(!err.0.contains("at Object"));
        assert!(!err.0.contains("node:internal"));
    }

    #[test]
    fn unknown_option_raises_clean_user_error() {
        let cfg = string_opt_config(false);
        let err = parse_cli_args(&args(&["--nope", "x"]), &cfg).unwrap_err();
        assert!(err.0.contains("Unknown option"));
    }

    #[test]
    fn unexpected_positional_raises_clean_user_error() {
        let cfg = string_opt_config(false);
        assert!(parse_cli_args(&args(&["foo"]), &cfg).is_err());
    }

    #[test]
    fn boolean_option_given_eq_value_raises_clean_user_error() {
        let cfg = ArgsConfig::new(false).option("json", OptionDef::boolean(false));
        assert!(parse_cli_args(&args(&["--json=1"]), &cfg).is_err());
    }

    #[test]
    fn missing_value_at_end_of_argv_raises_clean_user_error() {
        let cfg = string_opt_config(false);
        assert!(parse_cli_args(&args(&["--budget"]), &cfg).is_err());
    }

    #[test]
    fn short_option_aliases_still_disambiguate_dash_leading_values() {
        let cfg = ArgsConfig::new(false).option("budget", OptionDef::string().with_short('b'));
        let parsed = parse_cli_args(&args(&["-b", "-1"]), &cfg).unwrap();
        assert_eq!(parsed.str("budget"), Some("-1"));
    }

    #[test]
    fn positionals_pass_through_untouched() {
        let cfg = ArgsConfig::new(true);
        let parsed = parse_cli_args(&args(&["claude", "agents"]), &cfg).unwrap();
        assert_eq!(
            parsed.positionals,
            vec!["claude".to_string(), "agents".to_string()]
        );
    }

    #[test]
    fn bare_dashdash_terminator_stops_rewriting() {
        let cfg = ArgsConfig::new(true).option("type", OptionDef::string());
        let parsed = parse_cli_args(&args(&["--", "--type", "-x"]), &cfg).unwrap();
        assert_eq!(parsed.str("type"), None);
        assert_eq!(
            parsed.positionals,
            vec!["--type".to_string(), "-x".to_string()]
        );
    }

    #[test]
    fn dashdash_is_never_swallowed_as_a_string_options_value() {
        let cfg = ArgsConfig::new(true).option("type", OptionDef::string());
        let err = parse_cli_args(&args(&["--type", "--", "pitfall"]), &cfg).unwrap_err();
        assert!(!err.0.contains("at Object"));
        assert!(!err.0.contains("node:internal"));
    }

    // Additional coverage beyond the ported TS suite, for the error text the
    // plan documents as verified against Node.
    #[test]
    fn error_text_matches_node_parseargs_first_lines() {
        let cfg = ArgsConfig::new(false).option("budget", OptionDef::string());
        assert_eq!(
            parse_cli_args(&args(&["--budget"]), &cfg).unwrap_err().0,
            "Option '--budget <value>' argument missing"
        );
        let cfg2 = ArgsConfig::new(false).option("json", OptionDef::boolean(false));
        assert_eq!(
            parse_cli_args(&args(&["--json=1"]), &cfg2).unwrap_err().0,
            "Option '--json' does not take an argument"
        );
        assert_eq!(
            parse_cli_args(&args(&["foo"]), &cfg2).unwrap_err().0,
            "Unexpected argument 'foo'. This command does not take positional arguments"
        );
        assert_eq!(
            parse_cli_args(&args(&["--nope"]), &cfg2).unwrap_err().0,
            "Unknown option '--nope'"
        );
        assert_eq!(
            parse_cli_args(&args(&["-z"]), &cfg2).unwrap_err().0,
            "Unknown option '-z'"
        );
    }

    #[test]
    fn boolean_defaults_apply_when_not_given() {
        let cfg = ArgsConfig::new(false).option("json", OptionDef::boolean(false));
        let parsed = parse_cli_args(&args(&[]), &cfg).unwrap();
        assert!(!parsed.bool("json"));
        let parsed2 = parse_cli_args(&args(&["--json"]), &cfg).unwrap();
        assert!(parsed2.bool("json"));
    }

    #[test]
    fn short_bundling_of_boolean_flags() {
        let cfg = ArgsConfig::new(false)
            .option("all", OptionDef::boolean(false).with_short('a'))
            .option("refresh", OptionDef::boolean(false).with_short('r'));
        let parsed = parse_cli_args(&args(&["-ar"]), &cfg).unwrap();
        assert!(parsed.bool("all"));
        assert!(parsed.bool("refresh"));
    }
}
