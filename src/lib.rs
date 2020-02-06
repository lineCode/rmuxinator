use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::process::Command;

extern crate toml;

fn build_pane_args(session_name: &String, window_index: &usize) -> Vec<String> {
    vec![
        String::from("splitw"),
        String::from("-t"),
        format!("{}:{}", session_name, window_index.to_string()),
    ]
}

fn create_pane(create_pane_args: Vec<String>) {
    let _create_window_command_output = Command::new("tmux")
        .args(&create_pane_args)
        .output()
        .expect("Unable to run create pane command.");
}

fn build_window_layout_args(
    session_name: &String,
    window_index: &usize,
    layout: &Layout,
) -> Vec<String> {
    vec![
        String::from("select-layout"),
        String::from("-t"),
        format!("{}:{}", session_name, window_index.to_string()),
        String::from(layout.to_string()),
    ]
}

fn set_window_layout(
    session_name: &String,
    window_index: &usize,
    layout: &Layout,
) {
    let set_window_layout_args =
        build_window_layout_args(session_name, window_index, layout);
    let _set_window_layout_output = Command::new("tmux")
        .args(&set_window_layout_args)
        .output()
        .expect("Unable to set window layout.");
}

fn build_create_window_args(
    session_name: &String,
    window_index: usize,
    window_name: &String,
    start_directory: &Option<String>,
) -> Vec<String> {
    let mut create_window_args = vec![
        String::from("new-window"),
        String::from("-t"),
        format!("{}:{}", session_name, window_index.to_string()),
        String::from("-n"),
        String::from(window_name),
    ];

    if let Some(start_directory_) = start_directory {
        create_window_args.push(String::from("-c"));
        create_window_args.push(String::from(start_directory_));
    }

    create_window_args
}

fn create_window(create_window_args: Vec<String>) {
    let _create_window_command_output = Command::new("tmux")
        .args(&create_window_args)
        .output()
        .expect("Unable to run create window command.");
}

fn build_session_args(
    session_name: &String,
    window_name: &String,
) -> Vec<String> {
    // TODO: conditionally append start_directory
    // Pass first window name to new-session, otherwise a default window gets
    // created that would need to be killed at a later point. I tried doing
    // this, but saw unexpected behavior -- most likely because the indexes get
    // shuffled.
    vec![
        String::from("new-session"),
        String::from("-d"),
        String::from("-s"),
        String::from(session_name),
        String::from("-n"),
        String::from(window_name),
    ]
}

fn create_session(create_session_args: Vec<String>) {
    Command::new("tmux")
        .args(&create_session_args)
        .output()
        .expect("Unable to create session.");
}

fn build_pane_command_args(
    session_name: &String,
    window_index: &usize,
    pane_index: &usize,
    command: &String,
) -> Vec<String> {
    vec![
        String::from("send-keys"),
        String::from("-t"),
        format!("{}:{}.{}", session_name, window_index, pane_index),
        String::from(command),
        String::from("Enter"),
    ]
}

fn run_pane_command(pane_command_args: &Vec<String>) {
    Command::new("tmux")
        .args(pane_command_args)
        .output()
        .expect("Unable to run pane command.");
}

fn build_attach_args(session_name: &String) -> Vec<String> {
    vec![
        String::from("-u"),
        String::from("attach-session"),
        String::from("-t"),
        String::from(session_name),
    ]
}

fn build_default_start_directory(config: &Config) -> StartDirectory {
    // Compute start_directory for first window using:
    // config.start_directory || window.start_directory

    let mut start_directory = None;
    if let Some(start_directory_) = config.start_directory.clone() {
        start_directory = Some(start_directory_);
    }
    if config.windows.len() > 0 {
        if let Some(start_directory_) = &config.windows[0].start_directory {
            start_directory = Some(String::from(start_directory_));
        }
    }
    start_directory
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let session_name = &config.name;

    // TODO: use conditionally
    let _start_directory = build_default_start_directory(&config);

    let create_session_args =
        build_session_args(session_name, &config.windows[0].name);
    create_session(create_session_args);

    for (window_index, window) in config.windows.iter().enumerate() {
        // The first window is created by create_session because tmux always
        // creates a window when creating a session.
        // The alternative would be to create all of the project windows and
        // then kill the first/default one, but I saw unexpected behavior
        // (first window's commands not being run) when attempting that -- I
        // think it's because the indexes get shuffled.
        // The alternative approach would be preferable, so maybe it's worth
        // revisiting.
        if window_index != 0 {
            let create_window_args = build_create_window_args(
                session_name,
                window_index,
                &window.name,
                &window.start_directory,
            );
            create_window(create_window_args);
        }

        for (pane_index, pane) in window.panes.iter().enumerate() {
            // Pane 0 is created by default by the containing window
            if pane_index > 0 {
                let pane_args = build_pane_args(session_name, &window_index);
                create_pane(pane_args);
            }

            // Conditionally set start_directory for pane.
            // Unfortunately, this can't be done cleanly using create_pane
            // because pane 0 is created implicitly.
            if let Some(start_directory) = &pane.start_directory {
                let command = format!("cd {}", &start_directory);
                let pane_command_args = build_pane_command_args(
                    session_name,
                    &window_index,
                    &pane_index,
                    &command,
                );
                run_pane_command(&pane_command_args);
            }

            for (_, command) in pane.commands.iter().enumerate() {
                let pane_command_args = build_pane_command_args(
                    session_name,
                    &window_index,
                    &pane_index,
                    command,
                );
                run_pane_command(&pane_command_args);
            }
        }

        match &window.layout {
            Some(layout) => {
                set_window_layout(session_name, &window_index, layout)
            }
            None => (),
        }
    }

    // TODO: Move this into helper. First attempt resulted in error caused by
    // return type. I think I either need to return the command and then spawn
    // or return the result of calling spawn.
    let attach_args = build_attach_args(&session_name);
    let _attach_output =
        Command::new("tmux").args(&attach_args).spawn()?.wait();

    Ok(())
}

#[derive(Debug)]
pub struct CliArgs {
    pub command: String,
    pub project_name: String,
}

impl CliArgs {
    pub fn new(args: &[String]) -> Result<CliArgs, &'static str> {
        println!("CliArgs: {:#?}", args);

        Ok(CliArgs {
            command: args[1].clone(),
            project_name: args[2].clone(),
        })
    }
}

#[derive(Debug, Deserialize)]
pub enum Layout {
    #[serde(rename = "even-horizontal")]
    EvenHorizontal,
    #[serde(rename = "even-vertical")]
    EvenVertical,
    #[serde(rename = "main-horizontal")]
    MainHorizontal,
    #[serde(rename = "main-vertical")]
    MainVertical,
    #[serde(rename = "tiled")]
    Tiled,
}

impl Layout {
    fn to_string(&self) -> String {
        match self {
            Self::EvenHorizontal => String::from("even-horizontal"),
            Self::EvenVertical => String::from("even-vertical"),
            Self::MainHorizontal => String::from("main-horizontal"),
            Self::MainVertical => String::from("main-vertical"),
            Self::Tiled => String::from("tiled"),
        }
    }
}

type StartDirectory = Option<String>;

#[derive(Debug, Deserialize)]
pub struct Pane {
    pub commands: Vec<String>,
    pub start_directory: StartDirectory,
}

#[derive(Debug, Deserialize)]
pub struct Window {
    pub layout: Option<Layout>,
    pub name: String,
    pub panes: Vec<Pane>,
    pub start_directory: StartDirectory,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub name: String,
    pub start_directory: StartDirectory,
    pub windows: Vec<Window>,
}

impl Config {
    pub fn new(cli_args: CliArgs) -> Result<Config, &'static str> {
        let mut f = match File::open(cli_args.project_name) {
            Ok(x) => x,
            Err(_) => return Err("Unable to open config file."),
        };
        let mut contents = String::new();

        match f.read_to_string(&mut contents) {
            Ok(_) => (),
            Err(_) => return Err("Unable to read config file."),
        }

        let decoded: Config = toml::from_str(&contents).unwrap();

        println!("decoded: {:#?}", decoded);

        Ok(decoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_builds_session_args() {
        let session_name = String::from("a session");
        let window_name = String::from("a window");
        let expected = vec![
            String::from("new-session"),
            String::from("-d"),
            String::from("-s"),
            String::from(&session_name),
            String::from("-n"),
            String::from(&window_name),
        ];
        let actual = build_session_args(&session_name, &window_name);
        assert_eq!(expected, actual);
    }

    #[test]
    fn it_builds_window_layout_args() {
        let session_name = String::from("foo");
        let window_index = 2;
        let layout = Layout::Tiled;
        let expected = vec![
            String::from("select-layout"),
            String::from("-t"),
            format!("{}:{}", &session_name, &window_index),
            layout.to_string(),
        ];
        let actual =
            build_window_layout_args(&session_name, &window_index, &layout);
        assert_eq!(expected, actual);
    }

    #[test]
    fn it_builds_window_args_without_a_start_directory() {
        let session_name = String::from("a session");
        let window_name = String::from("a window");
        let window_index = 42;
        let start_directory = None;
        let expected = vec![
            String::from("new-window"),
            String::from("-t"),
            format!("{}:{}", &session_name, &window_index),
            String::from("-n"),
            String::from(&window_name),
        ];
        let actual = build_create_window_args(
            &session_name,
            window_index,
            &window_name,
            &start_directory,
        );
        assert_eq!(expected, actual);
    }

    #[test]
    fn it_builds_window_args_with_a_start_directory() {
        let session_name = String::from("a session");
        let window_name = String::from("a window");
        let window_index = 42;
        let start_directory = Some(String::from("/tmp/neat"));

        let expected = vec![
            String::from("new-window"),
            String::from("-t"),
            format!("{}:{}", &session_name, &window_index),
            String::from("-n"),
            String::from(&window_name),
            String::from("-c"),
            String::from("/tmp/neat"),
        ];
        let actual = build_create_window_args(
            &session_name,
            window_index,
            &window_name,
            &start_directory,
        );
        assert_eq!(expected, actual);
    }

    #[test]
    fn it_builds_attach_args() {
        let session_name = String::from("a session");
        let expected = vec![
            String::from("-u"),
            String::from("attach-session"),
            String::from("-t"),
            String::from(&session_name),
        ];
        let actual = build_attach_args(&session_name);
        assert_eq!(expected, actual);
    }

    #[test]
    fn it_converts_layout_to_string() {
        let layout = Layout::Tiled;
        let expected = layout.to_string();
        let actual = String::from("tiled");
        assert_eq!(expected, actual);
    }

    #[test]
    fn it_uses_the_project_start_directory_when_present() {
        let config = Config {
            name: String::from("foo"),
            start_directory: Some(String::from("/foo/bar")),
            windows: Vec::new(),
        };
        let expected = Some(String::from("/foo/bar"));
        let actual = build_default_start_directory(&config);
        assert_eq!(expected, actual);
    }

    #[test]
    fn it_uses_the_first_windows_start_directory_when_window_start_directory_not_present(
    ) {
        let config = Config {
            name: String::from("foo"),
            start_directory: None,
            windows: vec![Window {
                layout: None,
                name: String::from("a window"),
                panes: Vec::new(),
                start_directory: Some(String::from("/bar/baz")),
            }],
        };
        let expected = Some(String::from("/bar/baz"));
        let actual = build_default_start_directory(&config);
        assert_eq!(expected, actual);
    }
}
