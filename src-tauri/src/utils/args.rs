pub fn is_network_mode_flag(arg: &str) -> bool {
    arg == "--network-mode" || arg.starts_with("--network-mode=")
}

fn parse_network_mode_flag(arg: &str) -> Option<bool> {
    if arg == "--network-mode" {
        return Some(true);
    }

    match arg.strip_prefix("--network-mode=")? {
        "1" | "t" | "T" | "TRUE" | "true" | "True" => Some(true),
        "0" | "f" | "F" | "FALSE" | "false" | "False" => Some(false),
        _ => None,
    }
}

pub fn remove_network_mode_flags(args: Vec<String>) -> (Vec<String>, Option<bool>) {
    let mut filtered = Vec::with_capacity(args.len());
    let mut network_mode = None;
    let mut options_ended = false;

    for arg in args {
        if options_ended {
            filtered.push(arg);
        } else if arg == "--" {
            options_ended = true;
            filtered.push(arg);
        } else if is_network_mode_flag(&arg) {
            if let Some(value) = parse_network_mode_flag(&arg) {
                network_mode = Some(value);
            }
        } else {
            filtered.push(arg);
        }
    }

    (filtered, network_mode)
}

fn quote_arg(arg: &str) -> String {
    if arg
        .chars()
        .all(|ch| !ch.is_whitespace() && !matches!(ch, '\\' | '\'' | '"'))
    {
        arg.to_string()
    } else {
        format!("\"{}\"", arg.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

pub fn remove_network_mode_flags_from_groups(groups: Vec<String>) -> (Vec<String>, Option<bool>) {
    let mut filtered = Vec::with_capacity(groups.len());
    let mut network_mode = None;
    let mut options_ended = false;

    for group in groups {
        if options_ended {
            filtered.push(group);
            continue;
        }

        let args = split_args(&group);
        let arg_count = args.len();
        let (remaining, group_network_mode) = remove_network_mode_flags(args);
        let removed_network_mode = remaining.len() != arg_count;
        if let Some(value) = group_network_mode {
            network_mode = Some(value);
        }
        if remaining.iter().any(|arg| arg == "--") {
            options_ended = true;
        }

        if removed_network_mode {
            filtered.extend(remaining.into_iter().map(|arg| quote_arg(&arg)));
        } else {
            filtered.push(group);
        }
    }

    (filtered, network_mode)
}

pub fn split_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut in_quotes = false;
    let mut quote_char = '"';
    let mut escape_next = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if escape_next {
            current_arg.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '\\' => {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '"' || next_ch == '\'' || next_ch == '\\' {
                        escape_next = true;
                    } else {
                        current_arg.push(ch);
                    }
                } else {
                    current_arg.push(ch);
                }
            }
            '"' | '\'' if !in_quotes => {
                in_quotes = true;
                quote_char = ch;
            }
            ch if in_quotes && ch == quote_char => {
                in_quotes = false;
            }
            ' ' | '\t' if !in_quotes => {
                if !current_arg.is_empty() {
                    args.push(current_arg.clone());
                    current_arg.clear();
                }
                while let Some(&next_ch) = chars.peek() {
                    if next_ch == ' ' || next_ch == '\t' {
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
            _ => {
                current_arg.push(ch);
            }
        }
    }

    if !current_arg.is_empty() {
        args.push(current_arg);
    }

    args
}

pub fn split_args_vec(args: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    for arg in args {
        result.extend(split_args(&arg));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{remove_network_mode_flags, remove_network_mode_flags_from_groups, split_args_vec};

    #[test]
    fn removes_only_managed_network_mode_flags() {
        let args = split_args_vec(vec![
            "--timeout=5m --network-mode --network-mode=false".into(),
            "--network-mode-extra".into(),
            "--header=--network-mode".into(),
        ]);

        let (filtered, network_mode) = remove_network_mode_flags(args);

        assert_eq!(
            filtered,
            vec![
                "--timeout=5m",
                "--network-mode-extra",
                "--header=--network-mode"
            ]
        );
        assert_eq!(network_mode, Some(false));
    }

    #[test]
    fn preserves_managed_flag_after_option_terminator() {
        let args = vec![
            "--network-mode".into(),
            "--".into(),
            "--network-mode=false".into(),
        ];

        let (filtered, network_mode) = remove_network_mode_flags(args);

        assert_eq!(filtered, vec!["--", "--network-mode=false"]);
        assert_eq!(network_mode, Some(true));
    }

    #[test]
    fn preserves_groups_after_option_terminator() {
        let groups = vec!["--network-mode --".into(), "--network-mode=false".into()];

        let (filtered, network_mode) = remove_network_mode_flags_from_groups(groups);

        assert_eq!(filtered, vec!["--", "--network-mode=false"]);
        assert_eq!(network_mode, Some(true));
    }

    #[test]
    fn cleans_grouped_flags_without_losing_other_arguments() {
        let groups = vec![
            "--timeout=5m --network-mode --header 'value with spaces'".into(),
            "--read-only".into(),
        ];

        let (filtered, network_mode) = remove_network_mode_flags_from_groups(groups);

        assert_eq!(
            filtered,
            vec![
                "--timeout=5m",
                "--header",
                "\"value with spaces\"",
                "--read-only"
            ]
        );
        assert_eq!(network_mode, Some(true));
        assert_eq!(
            split_args_vec(filtered),
            vec![
                "--timeout=5m",
                "--header",
                "value with spaces",
                "--read-only"
            ]
        );
    }

    #[test]
    fn removes_invalid_managed_value_without_enabling_it() {
        let args = vec!["--network-mode=invalid".into(), "--read-only".into()];

        let (filtered, network_mode) = remove_network_mode_flags(args);

        assert_eq!(filtered, vec!["--read-only"]);
        assert_eq!(network_mode, None);
    }
}
