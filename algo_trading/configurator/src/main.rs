//! Configurator for the platform.  Initializes config files for the modules and sets up
//! the initial environment for platform runtime.

extern crate cursive;
extern crate serde_json;
extern crate termion;

use std::process::{Command, Stdio};
use std::path::Path;
use std::rc::Rc;
use std::str;
use cursive::Cursive;
use cursive::views::{Dialog, TextView, EditView, ListView, BoxedView, LinearLayout};
use cursive::view::SizeConstraint;
use cursive::direction::Orientation;
use cursive::align::VAlign;
use cursive::traits::*;
use cursive::CursiveExt;
use cursive::view::{Nameable, Resizable};

mod theme;
use theme::THEME;
mod misc;
use misc::*;
mod directory;
use directory::*;
mod schema;
use crate::schema::*;

const MIN15: SizeConstraint = SizeConstraint::AtLeast(10);
const FREE: SizeConstraint = SizeConstraint::Free;

fn main() {
    // Check if this is the first run of the configurator
    let path = Path::new("settings.json");
    let mut s = Cursive::new();
    s.set_theme(THEME.clone());
    if !path.exists() {
        first_time(&mut s);
    } else {
        let settings = Settings::read_json("settings.json");
        show_directory(&mut s, settings, true);
    }
}

/// Called after exiting the directory.
fn directory_exit(s: &mut Cursive, settings: Settings) {
    write_settings(settings.clone());
    let content = indoc(
        "Settings files have been regenerated.  However, the platform must be rebuilt (`make`) \
        in order for any changes to be reflected.
        Edit `settings.json` in the `configurator` directory and run `make config` again to change settings.
        Delete `settings.json` and re-run configurator to start from scratch."
    );
    s.add_layer(Dialog::text(content)
        .button("Ok", move |s| {
            s.quit();
        })
    );
}

/// Returns the content of the `EditView` with the given ID.
fn get_by_id(id: &str, s: &mut Cursive) -> Option<Rc<String>> {
    match s.call_on_name::<EditView, _, _>(id, |view| view.get_content()) {
        Some(content) => Some(content),
        None => None
    }
}

/// Displays welcome and walks the user through first time configuration of the platform.
fn first_time(siv: &mut Cursive) {
    // Main Key:Value settings for the application
    let settings = Settings::new();
    siv.add_layer(
        Dialog::around(TextView::new(
            &indoc(
                "Welcome to the TickGrinder Algorithmic Trading Platform!
                This tool will set up the environment for the trading platform.  It will walk you through the process of \
                installing all prerequisite software and initializing all necessary configuration settings for the platform's \
                modules."
            )
        )).title("Welcome")
            .button("Continue", {
                let settings = settings.clone();
                move |s| {
                    if !is_installed("node") {
                        s.add_layer(Dialog::text(indoc(
                            "NodeJS is required in order to run the platform's Management+Monitoring (MM) Web GUI.
                            Please install NodeJS and add the `node` binary to the system path before installing the platform."
                        )).button("Ok", |s| s.quit() ));
                    }
                    settings.set("node_binary_path", &which("node"));
                    boost_config(s, settings.clone());
                }
            })
    );
    // Start the event loop
    siv.run();
}

/// Checks if we think libboost is installed and lets the user know.
fn boost_config(s: &mut Cursive, settings: Settings) {
    s.pop_layer();
    // TODO: Fix this so that it detects the library on all platforms
    let content = if /*libboost_detected()*/ true {
        &indoc(
            "From what I can see, libboost is installed on this system.  Boost is required for this platform's C++ \
            FFI components.
            If it's true that you have it installed (`sudo apt-get install libboost-all-dev`) then \
            you can proceed with the installation.  If not, please install it before continuing."
        )
    } else {
        &indoc(
            "I was unable to detect the boot library on your computer.  It's possible that it's installed and that I \
            simply can't see it.
            However, if you haven't already installed libboost (`sudo apt-get install \
            libboost-all-dev`), please install it before continuing."
        )
    };
    let dialog = Dialog::around(TextView::new(content))
        .button("Proceed", {
            let settings = settings.clone();
            move |s| {
                redis_config(s, settings.clone());
            }
        }).button("Exit", move |s| s.quit());
    s.add_layer(dialog);
}

/// First stage of Redis configuration; asks if you want to do a Remote or Local installation.
fn redis_config(s: &mut Cursive, settings: Settings) {
    let settings_clone = settings.clone();
    let mut message_text =
        String::from(
            &indoc(
                "The first thing that needs to be configured is Redis.  Redis PubSub is used as a messaging service \
                that allows for communication between the platform's different modules.  It is recommended that you \
                use a local instance of Redis since it's easy to create a security vulnerability if it is not properly \
                configured.\n\n"
            )
        );
    let installed = is_installed("redis-server");
    if installed {
        message_text +=
            &indoc(
                "I detected that you currently have Redis installed.  To use this local Redis install for the platform, \
                select the \"Local\" option below."
            );
    } else {
        message_text +=
            &indoc(
                "I was unable to detect a local Redis installation.  If Redis is currently installed and you \
                want to use a local installation, please add the `redis-server` executable to the system PATH and \
                restart the configuration process."
            );
    }
    s.pop_layer();
    s.add_layer(Dialog::text(message_text)
        .title("Information")
        .button("Local", {
            let settings = settings.clone();
            move |s| redis_local(s, is_installed("redis-server"), settings.clone())
        })
        .button("Remote", {
            let settings = settings_clone.clone();
            move |s| redis_remote(s, settings.clone())
        })
        .button("Exit", |s| s.quit() )
    )
}

fn redis_local(s: &mut Cursive, installed: bool, settings: Settings) {
    if !installed {
        s.add_layer(Dialog::text(
            &indoc(
                "You must install redis in order to use it locally.  Install Redis, add the `redis-server` \
                binary to to the system PATH, and select local again."
            )
        ).dismiss_button("Ok"))
    } else {
        s.pop_layer();
        let mut port_box = EditView::new();
        port_box.set_content("6379");
        port_box.set_on_submit({
            let settings = settings.clone();
            move |s, port| {
                let settings = settings.clone();
                settings.set("redis_host", &format!(
                    "redis://localhost:{}/",
                    port
                ));
                settings.set("redis_server_binary_path", &which("redis-server"));
                postgres_config(s, settings.clone())
            }
        });
        s.add_layer(Dialog::around(port_box)
            .title("Redis Port")
        );
    }
}

fn redis_remote(s: &mut Cursive, settings: Settings) {
    s.pop_layer();
    s.add_layer(Dialog::new()
        .content(ListView::new()
            .child("Redis Host", EditView::new().with_name("redis_host").fixed_width(40))
            .child("Redis Port", EditView::new().content("6379").with_name("redis_port").fixed_width(40))
        ).title("Remote Redis Server Settings")
        .button("Ok", {
            let settings = settings.clone();
            move |s| {
                let settings = settings.clone();
                settings.set("redis_host", &format!(
                    "redis://{}:{}/",
                    &*get_by_id("redis_host", s).unwrap_or(Rc::new(String::new())),
                    &*get_by_id("redis_port", s).unwrap_or(Rc::new(String::from("6379")))
                ));
                postgres_config(s, settings)
            }
        })
    );
}

fn postgres_config(s: &mut Cursive, settings: Settings) {
    let settings_clone = settings.clone();
    let installed = is_installed("psql");
    let mut message_text =
        String::from(
            &indoc("The platform also relies on PostgreSQL to store historical Tick data, persistant platform \
                    data, and other information for the platform.\n\n"
            )
        );
    if !installed {
        message_text += &indoc(
            "I was unable do detect an active PostgreSQL installation on this host.  In order to use this \
            host for the platform, you must first install PostgreSQL and add the `psql` binary to the system \
            path.  Once you've installed it, select \"Local\" again. "
        );
    } else {
        message_text += &indoc(
            "I detected that you have PostgreSQL installed locally.  To configure the platform to use the \
            local PostgreSQL installation, select the \"Local\" option below."
        );
    }
    s.pop_layer();
    s.add_layer(Dialog::text(message_text)
        .title("PostgreSQL Configuration")
        .button("Local", {
            let settings = settings.clone();
            move |s| postgres_local(s, is_installed("psql"), settings.clone())
        })
        .button("Remote", {
            let settings = settings_clone.clone();
            move |s| postgres_remote(s, settings.clone())
        })
        .button("Exit", |s| s.quit() )
    );
}

fn postgres_remote(s: &mut Cursive, settings: Settings) {
    s.pop_layer();
    s.add_layer(Dialog::new()
        .content(ListView::new()
            .child("Postgres Host", EditView::new().with_name("postgres_host").fixed_width(40))
            .child("Postgres User", EditView::new().with_name("postgres_user").fixed_width(40))
            .child("Postgres Password", EditView::new().secret().with_name("postgres_password").fixed_width(40))
            .child("Postgres Port", EditView::new().content("5432").with_name("postgres_port").fixed_width(40))
            .child("Postgres Database", EditView::new().with_name("postgres_db").fixed_width(40))
        ).title("Remote PostgreSQL Server Settings")
            .button("Ok", {
                let settings = settings.clone();
                move |s| {
                    save_settings(s, settings.clone(), POSTGRES_IDS);
                    set_data_dir(s, settings.clone())
                }
            })
    )
}

fn postgres_local(s: &mut Cursive, installed: bool, settings: Settings) {
    settings.set("postgres_host", "localhost");
    if !installed {
        s.add_layer(Dialog::text(
            &indoc(
                "You must install PostgreSQL in order to use it locally.  Install PostgreSQL, add the `psql` \
                binary to to the system PATH, and select local again."
            )
        ).dismiss_button("Ok"))
    } else {
        s.pop_layer();
        s.add_layer(Dialog::new()
            .content(ListView::new()
                .child("Postgres User", EditView::new().with_name("postgres_user").fixed_width(40))
                .child("Postgres Password", EditView::new().secret().with_name("postgres_password").fixed_width(40))
                .child("Postgres Port", EditView::new().content("5432").with_name("postgres_port").fixed_width(40))
                .child("Postgres Database", EditView::new().with_name("postgres_db").fixed_width(40))
            ).title("Local PostgreSQL Server Settings").button("Ok", {
                let settings = settings.clone();
                move |s| {
                    settings.set("postgres_host", "localhost");
                    save_settings(s, settings.clone(), POSTGRES_IDS);
                    set_data_dir(s, settings.clone())
                }
            })
        )
    }
}

/// Ask the user for a place to store historical data and do some basic sanity checks on the supplied path.
fn set_data_dir(s: &mut Cursive, settings: Settings) {
    s.pop_layer();
    let dialog = Dialog::new().content(LinearLayout::new(Orientation::Vertical)
        .child(TextView::new(
            &indoc(
                "The data directory holds all persistant storage for the platform including historical price data, \
                database dumps, and platform configuration settings.  The entry below should be the absolute path of a \
                folder that exists and is writable by the user that the platform will be run as.\n\n"
            )
        ))
        .child(ListView::new()
            .child("Data Directory", EditView::new().content("/var/tickgrinder_data/").with_name("data_directory").fixed_width(40))
        )
    ).title("Data Directory").button("Ok", {
        let settings = settings.clone();
        move |s| {
            let settings = settings.clone();
            let dir = get_by_id("data_directory", s);
            match check_data_dir(&*(dir.clone()).unwrap_or(Rc::new(String::from("/var/tickgrinder_data/")))) {
                Ok(()) => {
                    settings.set("data_dir", &*dir.unwrap_or(Rc::new(String::from("/var/tickgrinder_data/"))));
                    fxcm_config(s, settings.clone());
                },
                Err(err) => {
                    error_popup(s, err)
                },
            };
        }
    });
    s.add_layer(dialog)
}

/// Runs `which [command]` and returns true if the binary is located.
fn is_installed(binary: &str) -> bool {
    // Special case for macOS common installations
    if cfg!(target_os = "macos") {
        // On macOS, check common installation locations as well
        match binary {
            "redis-server" => {
                // Check all common Redis installation locations on macOS
                Path::new("/usr/local/bin/redis-server").exists() || 
                Path::new("/opt/homebrew/bin/redis-server").exists() ||
                Path::new("/usr/local/Cellar/redis").exists() ||
                Path::new("/opt/homebrew/Cellar/redis").exists() ||
                !which(binary).is_empty()
            },
            "psql" => {
                // Check all common PostgreSQL installation locations on macOS
                Path::new("/usr/local/bin/psql").exists() ||
                Path::new("/opt/homebrew/bin/psql").exists() || 
                Path::new("/Applications/Postgres.app/Contents/Versions/latest/bin/psql").exists() ||
                Path::new("/usr/local/Cellar/postgresql").exists() ||
                Path::new("/opt/homebrew/Cellar/postgresql").exists() ||
                !which(binary).is_empty()
            },
            _ => !which(binary).is_empty()
        }
    } else {
        !which(binary).is_empty()
    }
}

fn which(binary: &str) -> String {
    // Use `which` command to find the binary, but handle errors gracefully
    let result = Command::new("which")
        .arg(binary)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped()) // Capture stderr too to avoid error messages
        .spawn();
    
    match result {
        Ok(child) => {
            match child.wait_with_output() {
                Ok(output) => {
                    if output.status.success() {
                        String::from(String::from_utf8_lossy(&output.stdout).trim())
                    } else {
                        String::new()
                    }
                },
                Err(_) => String::new()
            }
        },
        Err(_) => String::new()
    }
}

/// Creates an error popup with the supplied message and a button to dismiss it.
fn error_popup(s: &mut Cursive, err_str: &str) {
    s.add_layer(Dialog::text(err_str).dismiss_button("Ok"));
}

/// Writes the entered settings to a JSON file.  Also generates Rust and JavaScript config files
/// that can be copied into the project src directories.
fn write_settings(settings: Settings) {
    settings.write_json("settings.json");
    settings.write_rust("conf.rs");
    settings.write_js("conf.js");
}

/// Gets credentials for use in the FXCM broker shim.
fn fxcm_config(s: &mut Cursive, settings: Settings) {
    s.pop_layer();
    let lv = ListView::new()
        .child("Demo Account Username", EditView::new()
            .content("D000000000000")
            .with_name("fxcm_username").fixed_width(40)
        ).child("Demo Account Password", EditView::new()
            .content("1234")
            .with_name("fxcm_password").fixed_width(40)
        );
    let text = TextView::new(
        &indoc(
            "The platform has a shim to the native C++ FXCM ForexConnect API which allows for historical data \
            downloading and live price streaming of real FXCM data.  In order to use this broker, you must make a \
            FXCM Demo account.  You can do this for free in a couple seconds (no personal details necessary) here:
            https://www.fxcm.com/forex-trading-demo/
            Once you have credentials, you can enter them here.  ONLY USE DEMO CREDENTIALS, NOT REAL ACCOUNT \
            CREDENTIALS; the API isn't yet ready for live trading.
            If you don't want to use the FXCM API, you can just leave these values at their defaults and select \
            continue to proceed with the rest of the configuration process.\n\n"
        )
    );
    s.add_layer(Dialog::around(LinearLayout::new(Orientation::Vertical)
        .child(text)
        .child(lv)
    ).button("Ok", {
        let settings = settings.clone();
        move |s| {
            let settings = settings.clone();
            settings.set("fxcm_username", &*get_by_id("fxcm_username", s).unwrap_or(Rc::new(String::from("D000000000000"))));
            settings.set("fxcm_password", &*get_by_id("fxcm_password", s).unwrap_or(Rc::new(String::from("1234"))));
            settings.set("fxcm_url", "http://www.fxcorporate.com/Hosts.jsp");
            // let fxcm_username = String::from(settings.get(String::from("fxcm_username")).unwrap());
            // s.add_layer(Dialog::around(TextView::new(fxcm_username)));
            initial_config_done(s, settings.clone());
        }
    }));
}

/// Displays a message about how to use the directory and saves all settings to file.
fn initial_config_done(s: &mut Cursive, settings: Settings) {
    s.pop_layer();
    write_settings(settings.clone());
    s.add_layer(Dialog::text(
        &indoc(
            "The trading platform has been successfully configured.  Run `run.sh` and visit `localhost:8002` in \
            your web browser to start using the platform.
            You will now be presented with the configuration directory containing all of the platform's settings.  \
            You can reach that menu at any time by running `make configure` in the project's root directory.  If you \
            want to reset all the settings and start the configuration process from scratch, just delete the \
            `settings.json` file in the `configurator` directory and run `make config` again from the project root."
        )
    ).button("Ok", {
        let settings = settings.clone();
        move |s| {
            show_directory(s, settings.clone(), false);
        }
    }))
}

/// Attempts to read the values of all fields with the supplied IDs from the Cursive object and write them
/// into the Settings object.  Ignores them if such an ID doesn't exist.
fn save_settings(s: &mut Cursive, settings: Settings, ids: &[&str]) {
    for id in ids {
        let id: &str = id;
        let val = get_by_id(id, s);
        if val.is_some() {
            settings.set(id, &*val.unwrap() );
        }
    }
}

/// Returns Ok if the user's selected data directory is good to use and an Err with a reason why not otherwise.
fn check_data_dir(dir: &str) -> Result<(), &'static str> {
    let path = Path::new(dir);
    if !path.exists() {
        return Err(
            "The path you specified does not exist.  Please make sure that you supplied a directory that the \
            platform's user has full read and write access to."
        );
    }
    // TODO: Check that the directory has the correct permissions, maybe auto-create directory if it doesn't exist.
    Ok(())
}

/// Returns True if we think libboost is installed
fn libboost_detected() -> bool {
    // ldconfig -p | grep libboost
    let child = Command::new("ldconfig")
        .arg("-p")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Unable to spawn `ldconfig -p`");
    let output = child.wait_with_output()
        .expect("Unable to get output from `which redis_server`");
    let output_string = String::from(str::from_utf8(output.stdout.as_slice())
        .expect("Unable to convert result buffer into String"));
    output_string.find("libboost").is_some()
}

// Simple indoc function to replace the removed indoc! macro
fn indoc(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut result = String::new();
    
    for line in lines {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            result.push_str(trimmed);
            result.push('\n');
        }
    }
    
    result
}

#[test]
fn redis_installed_test() {
    is_installed("redis-server");
}
