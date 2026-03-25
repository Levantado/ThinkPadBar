use std::future::Future;
use std::pin::Pin;
use zbus::Connection;

pub use crate::modules::wifi::{WifiInfo, WifiNetwork};

pub trait NetworkBackend {
    fn get_wifi_info<'a>(
        &'a self,
        conn: &'a Connection,
        adapter_path: &'a str,
        station_path: &'a str,
    ) -> Pin<Box<dyn Future<Output = WifiInfo> + Send + 'a>>;
    fn scan_networks<'a>(
        &'a self,
        conn: &'a Connection,
        station_path: &'a str,
    ) -> Pin<Box<dyn Future<Output = Vec<WifiNetwork>> + Send + 'a>>;
    fn connect_network<'a>(
        &'a self,
        conn: &'a Connection,
        station_path: &'a str,
        ssid: String,
        passphrase: Option<String>,
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>>;
    fn toggle_wifi<'a>(
        &'a self,
        conn: &'a Connection,
        adapter_path: &'a str,
        station_path: &'a str,
        enable: bool,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct IwdBackend;

impl NetworkBackend for IwdBackend {
    fn get_wifi_info<'a>(
        &'a self,
        conn: &'a Connection,
        adapter_path: &'a str,
        station_path: &'a str,
    ) -> Pin<Box<dyn Future<Output = WifiInfo> + Send + 'a>> {
        Box::pin(async move {
            crate::modules::wifi::get_wifi_info(conn, adapter_path, station_path).await
        })
    }

    fn scan_networks<'a>(
        &'a self,
        conn: &'a Connection,
        station_path: &'a str,
    ) -> Pin<Box<dyn Future<Output = Vec<WifiNetwork>> + Send + 'a>> {
        Box::pin(async move { crate::modules::wifi::scan_networks(conn, station_path).await })
    }

    fn connect_network<'a>(
        &'a self,
        conn: &'a Connection,
        station_path: &'a str,
        ssid: String,
        passphrase: Option<String>,
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            crate::modules::wifi::connect_network(conn, station_path, ssid, passphrase).await
        })
    }

    fn toggle_wifi<'a>(
        &'a self,
        conn: &'a Connection,
        adapter_path: &'a str,
        station_path: &'a str,
        enable: bool,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            crate::modules::wifi::toggle_wifi(conn, adapter_path, station_path, enable).await
        })
    }
}

fn active_backend() -> IwdBackend {
    IwdBackend
}

pub async fn get_wifi_info(conn: &Connection, adapter_path: &str, station_path: &str) -> WifiInfo {
    active_backend()
        .get_wifi_info(conn, adapter_path, station_path)
        .await
}

pub async fn scan_networks(conn: &Connection, station_path: &str) -> Vec<WifiNetwork> {
    active_backend().scan_networks(conn, station_path).await
}

pub async fn connect_network(
    conn: &Connection,
    station_path: &str,
    ssid: String,
    passphrase: Option<String>,
) -> bool {
    active_backend()
        .connect_network(conn, station_path, ssid, passphrase)
        .await
}

pub async fn toggle_wifi(conn: &Connection, adapter_path: &str, station_path: &str, enable: bool) {
    active_backend()
        .toggle_wifi(conn, adapter_path, station_path, enable)
        .await
}
