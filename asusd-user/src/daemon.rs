use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use asusd_user::config::*;
use asusd_user::ctrl_anime::{CtrlAnime, CtrlAnimeInner};
use config_traits::{StdConfig, StdConfigLoad};
use rog_anime::usb::get_anime_type;
use rog_aura::aura_detection::LaptopLedData;
use rog_aura::layouts::KeyLayout;
use rog_dbus::{RogDbusClientBlocking, DBUS_NAME};
use smol::Executor;
use zbus::Connection;

#[cfg(not(feature = "local_data"))]
const DATA_DIR: &str = "/usr/share/rog-gui/";
#[cfg(feature = "local_data")]
const DATA_DIR: &str = env!("CARGO_MANIFEST_DIR");
const BOARD_NAME: &str = "/sys/class/dmi/id/board_name";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut logger = env_logger::Builder::new();
    logger
        .parse_default_env()
        .target(env_logger::Target::Stdout)
        .format(|buf, record| writeln!(buf, "{}: {}", record.level(), record.args()))
        .init();

    println!("  user daemon v{}", asusd_user::VERSION);
    println!("    rog-anime v{}", rog_anime::VERSION);
    println!("     rog-dbus v{}", rog_dbus::VERSION);
    println!("rog-platform v{}", rog_platform::VERSION);

    let (client, _) = RogDbusClientBlocking::new()?;
    let supported = client
        .proxies()
        .platform()
        .supported_interfaces()
        .unwrap_or_default()
        .contains(&"Anime".to_string());
    let config = ConfigBase::new().load();
    let executor = Executor::new();

    let early_return = Arc::new(AtomicBool::new(false));
    // Set up the anime data and run loop/thread
    if supported {
        if let Some(cfg) = config.active_anime {
            let anime_type = get_anime_type()?;
            let anime_config = ConfigAnime::new().set_name(cfg).load();
            let anime = anime_config.create(anime_type)?;
            let anime_config = Arc::new(Mutex::new(anime_config));

            executor
                .spawn(async move {
                    // Create server
                    let mut connection = Connection::session().await.unwrap();
                    connection.request_name(DBUS_NAME).await.unwrap();

                    // Inner behind mutex required for thread safety
                    let inner = Arc::new(Mutex::new(
                        CtrlAnimeInner::new(anime, client, early_return.clone()).unwrap(),
                    ));
                    // Need new client object for dbus control part
                    let (client, _) = RogDbusClientBlocking::new().unwrap();
                    let anime_control =
                        CtrlAnime::new(anime_config, inner.clone(), client, early_return).unwrap();
                    anime_control.add_to_server(&mut connection).await;
                    loop {
                        if let Ok(inner) = inner.clone().try_lock() {
                            inner.run().ok();
                        }
                    }
                })
                .detach();
        }
    }

    // if supported.keyboard_led.per_key_led_mode {
    if let Some(cfg) = config.active_aura {
        let mut aura_config = ConfigAura::new().set_name(cfg).load();
        // let baord_name = std::fs::read_to_string(BOARD_NAME)?;

        let led_support = LaptopLedData::get_data();

        let layout = KeyLayout::find_layout(led_support, PathBuf::from(DATA_DIR))
            .map_err(|e| {
                println!("{BOARD_NAME}, {e}");
            })
            .unwrap_or_else(|_| KeyLayout::default_layout());

        executor
            .spawn(async move {
                // Create server
                let (client, _) = RogDbusClientBlocking::new().unwrap();
                // let connection = Connection::session().await.unwrap();
                // connection.request_name(DBUS_NAME).await.unwrap();

                loop {
                    aura_config.aura.next_state(&layout);
                    let packets = aura_config.aura.create_packets();

                    client
                        .proxies()
                        .aura()
                        .direct_addressing_raw(packets)
                        .unwrap();
                    std::thread::sleep(std::time::Duration::from_millis(33));
                }
            })
            .detach();
    }
    // }

    loop {
        smol::block_on(executor.tick());
    }
}
