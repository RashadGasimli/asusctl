use eframe::NativeOptions;
use rog_aura::layouts::KeyLayout;
use rog_control_center::{
    config::Config, error::Result, get_ipc_file, notify::start_notifications, on_tmp_dir_exists,
    page_states::PageDataStates, print_versions, startup_error::AppErrorShow, RogApp,
    RogDbusClientBlocking, SHOWING_GUI, SHOW_GUI,
};
use rog_platform::supported::SupportedFunctions;
use tokio::runtime::Runtime;

use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
};

#[cfg(not(feature = "mocking"))]
const DATA_DIR: &str = "/usr/share/rog-gui/";
#[cfg(feature = "mocking")]
const DATA_DIR: &str = env!("CARGO_MANIFEST_DIR");
const BOARD_NAME: &str = "/sys/class/dmi/id/board_name";

fn main() -> Result<()> {
    print_versions();

    // start tokio
    let rt = Runtime::new().expect("Unable to create Runtime");
    // Enter the runtime so that `tokio::spawn` is available immediately.
    let _enter = rt.enter();

    let native_options = eframe::NativeOptions {
        vsync: true,
        decorated: true,
        transparent: false,
        min_window_size: Some(egui::vec2(840.0, 600.0)),
        max_window_size: Some(egui::vec2(840.0, 600.0)),
        run_and_return: true,
        ..Default::default()
    };

    let (dbus, _) = RogDbusClientBlocking::new()
        .map_err(|e| {
            eframe::run_native(
                "ROG Control Center",
                native_options.clone(),
                Box::new(move |_| Box::new(AppErrorShow::new(e.to_string()))),
            );
        })
        .unwrap();

    // Startup
    let mut config = Config::load()?;
    let mut start_closed = config.startup_in_background;

    if config.startup_in_background {
        config.run_in_background = true;
        config.save()?;
    }

    // Find and load a matching layout for laptop
    let mut file = OpenOptions::new()
        .read(true)
        .open(PathBuf::from(BOARD_NAME))
        .map_err(|e| {
            println!("{BOARD_NAME}, {e}");
            e
        })?;
    let mut board_name = String::new();
    file.read_to_string(&mut board_name)?;

    #[cfg(feature = "mocking")]
    {
        board_name = "gl504".to_string();
        path.pop();
        path.push("rog-aura");
        path.push("data");
    }

    let layout = KeyLayout::find_layout(board_name.as_str(), PathBuf::from(DATA_DIR))
        .map_err(|e| {
            println!("{BOARD_NAME}, {e}");
        })
        .unwrap_or_else(|_| KeyLayout::ga401_layout());

    // tmp-dir must live to the end of program life
    let _tmp_dir = match tempfile::Builder::new()
        .prefix("rog-gui")
        .rand_bytes(0)
        .tempdir()
    {
        Ok(tmp) => tmp,
        Err(_) => on_tmp_dir_exists().unwrap(),
    };

    let supported = match dbus.proxies().supported().supported_functions() {
        Ok(s) => s,
        Err(e) => {
            eframe::run_native(
                "ROG Control Center",
                native_options,
                Box::new(move |_| Box::new(AppErrorShow::new(e.to_string()))),
            );
            SupportedFunctions::default()
        }
    };
    let states = setup_page_state_and_notifs(
        &supported,
        layout.clone(),
        &config,
        native_options.clone(),
        &dbus,
    )
    .unwrap();

    loop {
        if !start_closed {
            start_app(supported, states.clone(), native_options.clone(), &dbus)?;
        }

        let config = Config::load().unwrap();
        if !config.run_in_background {
            break;
        }

        if config.run_in_background {
            let mut buf = [0u8; 4];
            // blocks until it is read, typically the read will happen after a second
            // process writes to the IPC (so there is data to actually read)
            if get_ipc_file().unwrap().read(&mut buf).is_ok() && buf[0] == SHOW_GUI {
                start_closed = false;
                continue;
            }
        }
    }

    // loop {
    //     // This is just a blocker to idle and ensure the reator reacts
    //     sleep(Duration::from_millis(1000)).await;
    // }
    Ok(())
}

fn setup_page_state_and_notifs(
    supported: &SupportedFunctions,
    keyboard_layout: KeyLayout,
    config: &Config,
    native_options: NativeOptions,
    dbus: &RogDbusClientBlocking,
) -> Result<PageDataStates> {
    // Cheap method to alert to notifications rather than spinning a thread for each
    // This is quite different when done in a retained mode app
    let charge_notified = Arc::new(AtomicBool::new(false));
    let bios_notified = Arc::new(AtomicBool::new(false));
    let aura_notified = Arc::new(AtomicBool::new(false));
    let anime_notified = Arc::new(AtomicBool::new(false));
    let profiles_notified = Arc::new(AtomicBool::new(false));
    let fans_notified = Arc::new(AtomicBool::new(false));
    let notifs_enabled = Arc::new(AtomicBool::new(config.enable_notifications));

    start_notifications(
        charge_notified.clone(),
        bios_notified.clone(),
        aura_notified.clone(),
        anime_notified.clone(),
        profiles_notified.clone(),
        fans_notified.clone(),
        notifs_enabled.clone(),
    )?;

    PageDataStates::new(
        keyboard_layout,
        notifs_enabled.clone(),
        charge_notified.clone(),
        bios_notified.clone(),
        aura_notified.clone(),
        anime_notified.clone(),
        profiles_notified.clone(),
        fans_notified.clone(),
        &supported,
        &dbus,
    )
}

fn start_app(
    supported: SupportedFunctions,
    states: PageDataStates,
    native_options: NativeOptions,
    dbus: &RogDbusClientBlocking,
) -> Result<()> {
    let mut ipc_file = get_ipc_file().unwrap();
    ipc_file.write_all(&[SHOWING_GUI]).unwrap();
    eframe::run_native(
        "ROG Control Center",
        native_options,
        Box::new(move |cc| {
            Box::new(RogApp::new(Config::load().unwrap(), states, supported, dbus, cc).unwrap())
        }),
    );
    Ok(())
}
