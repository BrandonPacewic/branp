use std::{env, fs};

use crate::doctor::{
    command_error_detail, finding, DoctorCheck, DoctorContext, Finding, Fix, Severity,
};

const LINKS_MARKER: &str = "bp doctor: hyperlinks";

pub struct LinksCheck;

impl DoctorCheck for LinksCheck {
    fn name(&self) -> &'static str {
        "links"
    }

    fn summary(&self) -> &'static str {
        "Checks OSC 8 terminal hyperlinks and tmux click handling."
    }

    fn run(&self, ctx: &DoctorContext) -> Vec<Finding> {
        let mut findings = Vec::new();
        findings.push(terminal_finding());

        if crate::utils::tmux::is_inside_tmux() {
            findings.extend(tmux_findings(ctx));
        } else {
            findings.push(finding(
                "links.no-tmux",
                Severity::Ok,
                "Not running inside tmux.",
                vec![
                    "Terminal hyperlink handling is delegated directly to the terminal emulator."
                        .to_string(),
                ],
                None,
            ));
        }

        findings
    }
}

fn terminal_finding() -> Finding {
    let terminal = if env::var_os("ALACRITTY_SOCKET").is_some()
        || env::var("__CFBundleIdentifier").is_ok_and(|value| value == "org.alacritty")
    {
        "Alacritty".to_string()
    } else if let Ok(program) = env::var("TERM_PROGRAM") {
        program
    } else {
        env::var("TERM").unwrap_or_else(|_| "unknown terminal".to_string())
    };

    finding(
        "links.terminal",
        Severity::Info,
        format!("Detected terminal: {terminal}."),
        vec![
            "OSC 8 links are emitted by bp as standard terminal hyperlink escape sequences."
                .to_string(),
        ],
        None,
    )
}

fn tmux_findings(ctx: &DoctorContext) -> Vec<Finding> {
    let mut findings = Vec::new();
    findings.push(tmux_config_finding(ctx));

    match crate::utils::tmux::version() {
        Ok(version) => {
            let severity = if version.supports_hyperlinks() {
                Severity::Ok
            } else {
                Severity::Error
            };
            findings.push(finding(
                "links.tmux-version",
                severity,
                format!("Detected {}.", version.raw()),
                if matches!(severity, Severity::Ok) {
                    vec!["tmux 3.4 or newer supports OSC 8 hyperlinks.".to_string()]
                } else {
                    vec![
                        "tmux must be 3.4 or newer for native OSC 8 hyperlink support.".to_string(),
                    ]
                },
                if matches!(severity, Severity::Ok) {
                    None
                } else {
                    Some(Fix::Manual {
                        description: "Upgrade tmux.".to_string(),
                        instructions: vec![
                            "Install tmux 3.4 or newer, then start a fresh tmux server."
                                .to_string(),
                        ],
                    })
                },
            ));
        }
        Err(error) => findings.push(finding(
            "links.tmux-version",
            Severity::Error,
            "Could not inspect tmux version.",
            command_error_detail(error),
            None,
        )),
    }

    let client_features = crate::utils::tmux::client_features().unwrap_or_default();
    if client_features.has("hyperlinks") {
        findings.push(finding(
            "links.tmux-client-features",
            Severity::Ok,
            "tmux client advertises hyperlink support.",
            vec![format!("client_termfeatures: {}", client_features.raw())],
            None,
        ));
    } else {
        findings.push(finding(
            "links.tmux-client-features",
            Severity::Warning,
            "tmux client does not advertise hyperlink support.",
            vec![
                format!("client_termfeatures: {}", client_features.raw()),
                "A new attach may be required after changing terminal-features.".to_string(),
            ],
            Some(tmux_config_fix(ctx)),
        ));
    }

    let terminal_features = crate::utils::tmux::terminal_features().unwrap_or_default();
    if terminal_features.has("hyperlinks") {
        findings.push(finding(
            "links.tmux-terminal-features",
            Severity::Ok,
            "tmux terminal-features includes hyperlinks.",
            vec![],
            None,
        ));
    } else {
        findings.push(finding(
            "links.tmux-terminal-features",
            Severity::Warning,
            "tmux terminal-features does not include hyperlinks.",
            vec!["tmux may strip OSC 8 hyperlink metadata without this feature.".to_string()],
            Some(tmux_config_fix(ctx)),
        ));
    }

    match crate::utils::tmux::mouse_mode() {
        Ok(true) => {
            let binding = crate::utils::tmux::mouse_down1_pane_binding().unwrap_or_default();
            if binding.opens_mouse_hyperlink() {
                findings.push(finding(
                    "links.tmux-mouse",
                    Severity::Ok,
                    "tmux mouse binding opens hyperlinks.",
                    vec![],
                    None,
                ));
            } else {
                findings.push(finding(
                    "links.tmux-mouse",
                    Severity::Warning,
                    "tmux mouse mode captures clicks before the terminal can open links.",
                    vec!["A MouseDown1Pane binding can open #{mouse_hyperlink} and keep normal tmux click behavior elsewhere.".to_string()],
                    Some(tmux_config_fix(ctx)),
                ));
            }
        }
        Ok(false) => findings.push(finding(
            "links.tmux-mouse",
            Severity::Ok,
            "tmux mouse mode is off.",
            vec!["The terminal emulator can receive normal link clicks directly.".to_string()],
            None,
        )),
        Err(error) => findings.push(finding(
            "links.tmux-mouse",
            Severity::Warning,
            "Could not inspect tmux mouse mode.",
            command_error_detail(error),
            None,
        )),
    }

    findings
}

fn tmux_config_finding(ctx: &DoctorContext) -> Finding {
    let path = ctx.home.join(".tmux.conf");
    let config = match fs::read_to_string(&path) {
        Ok(config) => crate::utils::tmux::TmuxConfig::parse(&config),
        Err(error) => {
            return finding(
                "links.tmux-config",
                Severity::Warning,
                format!("Could not read {}.", path.display()),
                vec![
                    error.to_string(),
                    "Live tmux checks may still pass, but new tmux servers may not be configured."
                        .to_string(),
                ],
                Some(tmux_config_fix(ctx)),
            )
        }
    };

    let has_features = config.has_hyperlink_terminal_features();
    let has_binding = config.has_mouse_hyperlink_binding();
    if has_features && has_binding {
        finding(
            "links.tmux-config",
            Severity::Ok,
            "~/.tmux.conf persists hyperlink support and click handling.",
            vec![
                "This checks active, non-commented config lines; live tmux state is reported separately.".to_string(),
            ],
            None,
        )
    } else {
        let mut details = Vec::new();
        if !has_features {
            details.push("Missing active terminal-features hyperlink configuration.".to_string());
        }
        if !has_binding {
            details.push("Missing active MouseDown1Pane #{mouse_hyperlink} binding.".to_string());
        }
        details.push(
            "Live tmux checks can still pass until tmux is re-sourced or restarted.".to_string(),
        );

        finding(
            "links.tmux-config",
            Severity::Warning,
            "~/.tmux.conf does not persist the full hyperlink fix.",
            details,
            Some(tmux_config_fix(ctx)),
        )
    }
}

fn tmux_config_fix(ctx: &DoctorContext) -> Fix {
    Fix::MarkerBlock {
        path: ctx.home.join(".tmux.conf"),
        marker: LINKS_MARKER,
        description: "Add tmux OSC 8 hyperlink support and click handling.".to_string(),
        lines: crate::utils::tmux::hyperlink_config_lines(),
    }
}
