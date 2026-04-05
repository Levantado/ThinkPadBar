use std::{
    collections::HashMap,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};

use iced::futures::{SinkExt, StreamExt};
use tracing::{info, warn};
use zbus::{
    fdo::ManagedObjects,
    message::{Message, Type},
    proxy,
    zvariant::{OwnedObjectPath, OwnedValue},
    Connection, MatchRule, MessageStream,
};

const BLUEZ_SERVICE: &str = "org.bluez";
const ROOT_PATH: &str = "/";
const ADAPTER_INTERFACE: &str = "org.bluez.Adapter1";
const DEVICE_INTERFACE: &str = "org.bluez.Device1";
const BATTERY_INTERFACE: &str = "org.bluez.Battery1";
const BLUEZ_MODEL_DETAIL: &str = "ObjectManager + Adapter1/Device1";
const DBUS_PROPERTIES_INTERFACE: &str = "org.freedesktop.DBus.Properties";
const DBUS_OBJECT_MANAGER_INTERFACE: &str = "org.freedesktop.DBus.ObjectManager";
const PROPERTIES_CHANGED_MEMBER: &str = "PropertiesChanged";
const INTERFACES_ADDED_MEMBER: &str = "InterfacesAdded";
const INTERFACES_REMOVED_MEMBER: &str = "InterfacesRemoved";
const BLUEZ_PATH_NAMESPACE: &str = "/org/bluez";
const BLUETOOTH_EVENT_RETRY_DELAY: Duration = Duration::from_secs(2);

#[proxy(
    interface = "org.bluez.Adapter1",
    default_service = "org.bluez",
    gen_blocking = true
)]
trait Adapter {
    #[zbus(property)]
    fn powered(&self) -> zbus::Result<bool>;

    #[zbus(property, name = "Powered")]
    fn set_powered(&self, value: bool) -> zbus::Result<()>;

    fn start_discovery(&self) -> zbus::Result<()>;
    fn stop_discovery(&self) -> zbus::Result<()>;
    fn remove_device(&self, device: OwnedObjectPath) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.bluez.Device1",
    default_service = "org.bluez",
    gen_blocking = true
)]
trait Device {
    fn connect(&self) -> zbus::Result<()>;
    fn disconnect(&self) -> zbus::Result<()>;
    fn pair(&self) -> zbus::Result<()>;

    #[zbus(property, name = "Trusted")]
    fn set_trusted(&self, value: bool) -> zbus::Result<()>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BluezBackendDiagnostics {
    pub unavailable_reason: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Clone, Default)]
pub struct BluezBluetoothBackend {
    async_connection: Arc<tokio::sync::Mutex<Option<Arc<Connection>>>>,
    diagnostics: Arc<Mutex<BluezBackendDiagnostics>>,
}

impl std::fmt::Debug for BluezBluetoothBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BluezBluetoothBackend")
            .finish_non_exhaustive()
    }
}

impl super::BluetoothBackend for BluezBluetoothBackend {
    fn backend_name(&self) -> &'static str {
        "bluez-dbus"
    }

    fn capability_mode(&self) -> crate::services::capabilities::CapabilityMode {
        let diagnostics = self.diagnostics.lock().unwrap();
        if diagnostics.unavailable_reason.is_some() {
            crate::services::capabilities::CapabilityMode::Unavailable
        } else {
            crate::services::capabilities::CapabilityMode::Native
        }
    }

    fn diagnostics_summary(&self) -> Option<String> {
        let diagnostics = self.diagnostics.lock().unwrap().clone();
        if let Some(reason) = diagnostics.unavailable_reason {
            return Some(reason);
        }
        if let Some(last_error) = diagnostics.last_error {
            return Some(format!("last error: {last_error}"));
        }
        Some(BLUEZ_MODEL_DETAIL.to_string())
    }

    fn enabled(&self) -> super::BackendFuture<'_, bool> {
        let backend = self.clone();
        Box::pin(async move {
            backend
                .async_adapter_state()
                .await
                .map(|(_path, powered)| powered)
                .unwrap_or(false)
        })
    }

    fn device_summary(
        &self,
    ) -> super::BackendFuture<'_, crate::services::controls::BluetoothDeviceSummary> {
        let backend = self.clone();
        Box::pin(async move {
            match backend.async_managed_objects().await {
                Ok(objects) => {
                    backend.clear_last_error();
                    device_summary_from_objects(&objects)
                }
                Err(error) => {
                    backend.record_error(format!("managed objects query failed: {error}"));
                    crate::services::controls::BluetoothDeviceSummary::default()
                }
            }
        })
    }

    fn toggle(&self, enable: bool) -> super::BackendFuture<'_, bool> {
        let backend = self.clone();
        Box::pin(async move {
            let Some(adapter_path) = backend.async_adapter_path().await.ok() else {
                return false;
            };
            let Ok(connection) = backend.async_connection().await else {
                return false;
            };

            info!("Attempting to toggle bluetooth power via BlueZ: {}", enable);
            let result = match AdapterProxy::builder(connection.as_ref()).path(adapter_path.as_str())
            {
                Ok(builder) => match builder.build().await {
                    Ok(proxy) => proxy.set_powered(enable).await,
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            };

            match result {
                Ok(()) => {
                    backend.clear_last_error();
                    true
                }
                Err(error) => {
                    backend.record_error(format!("set bluetooth power failed: {error}"));
                    false
                }
            }
        })
    }

    fn scan_devices(&self) -> super::BackendFuture<'_, bool> {
        Box::pin(async move {
            let Ok(connection) = self.async_connection().await else {
                return false;
            };
            let Ok(adapter_path) = self.async_adapter_path().await else {
                return false;
            };

            let proxy = match AdapterProxy::builder(connection.as_ref()).path(adapter_path.as_str())
            {
                Ok(builder) => match builder.build().await {
                    Ok(proxy) => proxy,
                    Err(error) => {
                        self.record_error(format!("adapter proxy build failed: {error}"));
                        return false;
                    }
                },
                Err(error) => {
                    self.record_error(format!("adapter path resolve failed: {error}"));
                    return false;
                }
            };

            match proxy.start_discovery().await {
                Ok(()) => {
                    self.clear_last_error();
                    true
                }
                Err(error) => {
                    self.record_error(format!("start discovery failed: {error}"));
                    false
                }
            }
        })
    }

    fn stop_scan_devices(&self) -> super::BackendFuture<'_, bool> {
        Box::pin(async move {
            let Ok(connection) = self.async_connection().await else {
                return false;
            };
            let Ok(adapter_path) = self.async_adapter_path().await else {
                return false;
            };

            let proxy = match AdapterProxy::builder(connection.as_ref()).path(adapter_path.as_str())
            {
                Ok(builder) => match builder.build().await {
                    Ok(proxy) => proxy,
                    Err(error) => {
                        self.record_error(format!("adapter proxy build failed: {error}"));
                        return false;
                    }
                },
                Err(error) => {
                    self.record_error(format!("adapter path resolve failed: {error}"));
                    return false;
                }
            };

            match proxy.stop_discovery().await {
                Ok(()) => {
                    self.clear_last_error();
                    true
                }
                Err(error) => {
                    self.record_error(format!("stop discovery failed: {error}"));
                    false
                }
            }
        })
    }

    fn connect_device(&self, address: String) -> super::BackendFuture<'_, bool> {
        Box::pin(async move {
            self.execute_device_operation(address, DeviceOperation::Connect)
                .await
        })
    }

    fn disconnect_device(&self, address: String) -> super::BackendFuture<'_, bool> {
        Box::pin(async move {
            self.execute_device_operation(address, DeviceOperation::Disconnect)
                .await
        })
    }

    fn pair_device(&self, address: String) -> super::BackendFuture<'_, bool> {
        Box::pin(async move {
            self.execute_device_operation(address, DeviceOperation::Pair)
                .await
        })
    }

    fn trust_device(&self, address: String) -> super::BackendFuture<'_, bool> {
        Box::pin(async move {
            self.execute_device_operation(address, DeviceOperation::Trust)
                .await
        })
    }

    fn remove_device(&self, address: String) -> super::BackendFuture<'_, bool> {
        Box::pin(async move {
            let Ok(connection) = self.async_connection().await else {
                return false;
            };
            let Ok(device_path) = self.async_device_path(&address).await else {
                return false;
            };
            let Some(adapter_path) = parent_adapter_path(&device_path) else {
                self.record_error(format!(
                    "remove device failed: adapter path missing for {}",
                    device_path.as_str()
                ));
                return false;
            };

            let proxy = match AdapterProxy::builder(connection.as_ref()).path(adapter_path.as_str())
            {
                Ok(builder) => match builder.build().await {
                    Ok(proxy) => proxy,
                    Err(error) => {
                        self.record_error(format!("adapter proxy build failed: {error}"));
                        return false;
                    }
                },
                Err(error) => {
                    self.record_error(format!("adapter path resolve failed: {error}"));
                    return false;
                }
            };

            match proxy.remove_device(device_path.clone()).await {
                Ok(()) => {
                    self.clear_last_error();
                    true
                }
                Err(error) => {
                    self.record_error(format!("remove device failed for {address}: {error}"));
                    false
                }
            }
        })
    }

    fn subscription(&self) -> iced::Subscription<crate::services::controls::ControlsEvent> {
        struct BluetoothListener;

        let backend = self.clone();
        iced::Subscription::run_with_id(
            std::any::TypeId::of::<BluetoothListener>(),
            iced::stream::channel(1, move |mut output| async move {
                loop {
                    let connection = match Connection::system().await {
                        Ok(connection) => connection,
                        Err(error) => {
                            backend.record_error(format!(
                                "BlueZ event listener failed to connect to system bus: {error}"
                            ));
                            warn!(
                                "BlueZ event listener failed to connect to system bus: {}",
                                error
                            );
                            tokio::time::sleep(BLUETOOTH_EVENT_RETRY_DELAY).await;
                            continue;
                        }
                    };

                    let stream = match MessageStream::for_match_rule(
                        match bluez_signal_match_rule() {
                            Ok(rule) => rule,
                            Err(error) => {
                                backend.record_error(format!(
                                    "BlueZ event listener failed to build match rule: {error}"
                                ));
                                warn!("BlueZ event listener failed to build match rule: {}", error);
                                tokio::time::sleep(BLUETOOTH_EVENT_RETRY_DELAY).await;
                                continue;
                            }
                        },
                        &connection,
                        Some(16),
                    )
                    .await
                    {
                        Ok(stream) => stream,
                        Err(error) => {
                            backend.record_error(format!(
                                "BlueZ event listener failed to subscribe: {error}"
                            ));
                            warn!("BlueZ event listener failed to subscribe: {}", error);
                            tokio::time::sleep(BLUETOOTH_EVENT_RETRY_DELAY).await;
                            continue;
                        }
                    };

                    let mut stream = stream;
                    backend.clear_last_error();

                    while let Some(message) = stream.next().await {
                        match message {
                            Ok(message) => {
                                if !is_relevant_bluetooth_signal(&message) {
                                    continue;
                                }

                                backend.clear_last_error();
                                if output
                                    .send(crate::services::controls::ControlsEvent::Bluetooth)
                                    .await
                                    .is_err()
                                {
                                    return;
                                }
                            }
                            Err(error) => {
                                backend.record_error(format!(
                                    "BlueZ event listener stream error: {error}"
                                ));
                                warn!("BlueZ event listener stream error: {}", error);
                                break;
                            }
                        }
                    }

                    tokio::time::sleep(BLUETOOTH_EVENT_RETRY_DELAY).await;
                }
            }),
        )
    }

    fn open_overskride(&self) -> bool {
        let direct = Command::new("overskride")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok();
        if direct {
            return true;
        }

        Command::new("flatpak")
            .args(["run", "io.github.kaii_lb.Overskride"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .is_ok()
    }
}

impl BluezBluetoothBackend {
    async fn async_connection(&self) -> zbus::Result<Arc<Connection>> {
        let mut guard = self.async_connection.lock().await;
        if let Some(connection) = guard.as_ref() {
            return Ok(Arc::clone(connection));
        }

        let connection = Arc::new(Connection::system().await?);
        *guard = Some(Arc::clone(&connection));
        Ok(connection)
    }

    async fn async_managed_objects(&self) -> zbus::Result<ManagedObjects> {
        let connection = self.async_connection().await?;
        let proxy = zbus::fdo::ObjectManagerProxy::builder(connection.as_ref())
            .destination(BLUEZ_SERVICE)?
            .path(ROOT_PATH)?
            .build()
            .await?;
        let managed = proxy.get_managed_objects().await?;
        if adapter_state_from_objects(&managed).is_some() {
            self.clear_unavailable_reason();
        } else {
            self.record_unavailable("org.bluez adapter unavailable");
        }
        Ok(managed)
    }

    async fn async_adapter_state(&self) -> Option<(OwnedObjectPath, bool)> {
        match self.async_managed_objects().await {
            Ok(objects) => {
                let state = adapter_state_from_objects(&objects);
                if state.is_none() {
                    self.record_unavailable("org.bluez adapter unavailable");
                }
                state
            }
            Err(error) => {
                self.record_error(format!("BlueZ unavailable: {error}"));
                None
            }
        }
    }

    async fn async_adapter_path(&self) -> zbus::Result<OwnedObjectPath> {
        let managed = self.async_managed_objects().await?;
        adapter_state_from_objects(&managed)
            .map(|(path, _powered)| path)
            .ok_or_else(|| missing_target_error("org.bluez adapter unavailable"))
    }

    async fn async_device_path(&self, address: &str) -> zbus::Result<OwnedObjectPath> {
        let managed = self.async_managed_objects().await?;
        device_path_from_objects(&managed, address)
            .ok_or_else(|| missing_target_error(format!("bluetooth device not found: {address}")))
    }

    async fn execute_device_operation(&self, address: String, operation: DeviceOperation) -> bool {
        let Ok(connection) = self.async_connection().await else {
            return false;
        };
        let Ok(device_path) = self.async_device_path(&address).await else {
            return false;
        };

        let proxy = match DeviceProxy::builder(connection.as_ref()).path(device_path.as_str()) {
            Ok(builder) => match builder.build().await {
                Ok(proxy) => proxy,
                Err(error) => {
                    self.record_error(format!("device proxy build failed for {address}: {error}"));
                    return false;
                }
            },
            Err(error) => {
                self.record_error(format!("device path resolve failed for {address}: {error}"));
                return false;
            }
        };

        let result = match operation {
            DeviceOperation::Connect => proxy.connect().await,
            DeviceOperation::Disconnect => proxy.disconnect().await,
            DeviceOperation::Pair => proxy.pair().await,
            DeviceOperation::Trust => proxy.set_trusted(true).await,
        };

        match result {
            Ok(()) => {
                self.clear_last_error();
                true
            }
            Err(error) => {
                self.record_error(format!("bluetooth action failed for {address}: {error}"));
                false
            }
        }
    }

    fn record_error(&self, message: impl Into<String>) {
        self.diagnostics.lock().unwrap().last_error = Some(message.into());
    }

    fn clear_last_error(&self) {
        self.diagnostics.lock().unwrap().last_error = None;
    }

    fn record_unavailable(&self, reason: impl Into<String>) {
        self.diagnostics.lock().unwrap().unavailable_reason = Some(reason.into());
    }

    fn clear_unavailable_reason(&self) {
        self.diagnostics.lock().unwrap().unavailable_reason = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BluezDeviceRecord {
    path: OwnedObjectPath,
    address: String,
    name: String,
    connected: bool,
    paired: bool,
    trusted: bool,
    battery_percent: Option<u8>,
    audio_profiles: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeviceOperation {
    Connect,
    Disconnect,
    Pair,
    Trust,
}

fn missing_target_error(message: impl Into<String>) -> zbus::Error {
    zbus::Error::Failure(message.into())
}

fn bluez_signal_match_rule() -> zbus::Result<MatchRule<'static>> {
    Ok(MatchRule::builder()
        .msg_type(Type::Signal)
        .sender(BLUEZ_SERVICE)?
        .path_namespace(BLUEZ_PATH_NAMESPACE)?
        .build())
}

fn is_relevant_bluetooth_signal(message: &Message) -> bool {
    if message.message_type() != Type::Signal {
        return false;
    }

    let header = message.header();
    is_relevant_bluetooth_signal_parts(
        header.path().map(|path| path.as_str()),
        header.interface().map(|interface| interface.as_str()),
        header.member().map(|member| member.as_str()),
    )
}

fn is_relevant_bluetooth_signal_parts(
    path: Option<&str>,
    interface: Option<&str>,
    member: Option<&str>,
) -> bool {
    let Some(path) = path else {
        return false;
    };
    if !path.starts_with(BLUEZ_PATH_NAMESPACE) {
        return false;
    }

    matches!(
        (interface, member),
        (
            Some(DBUS_PROPERTIES_INTERFACE),
            Some(PROPERTIES_CHANGED_MEMBER)
        ) | (
            Some(DBUS_OBJECT_MANAGER_INTERFACE),
            Some(INTERFACES_ADDED_MEMBER)
        ) | (
            Some(DBUS_OBJECT_MANAGER_INTERFACE),
            Some(INTERFACES_REMOVED_MEMBER)
        )
    )
}

fn adapter_state_from_objects(objects: &ManagedObjects) -> Option<(OwnedObjectPath, bool)> {
    objects
        .iter()
        .filter_map(|(path, interfaces)| {
            interface_props(interfaces, ADAPTER_INTERFACE).map(|props| {
                (
                    path.clone(),
                    property_bool(props, "Powered").unwrap_or(false),
                )
            })
        })
        .min_by(|(left_path, _), (right_path, _)| left_path.as_str().cmp(right_path.as_str()))
}

fn device_summary_from_objects(
    objects: &ManagedObjects,
) -> crate::services::controls::BluetoothDeviceSummary {
    let mut device_records = objects
        .iter()
        .filter_map(|(path, interfaces)| parse_device_record(path, interfaces))
        .collect::<Vec<_>>();
    device_records.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then(left.address.cmp(&right.address))
    });

    let connected_devices = device_records
        .iter()
        .filter(|device| device.connected)
        .map(|device| device.name.clone())
        .collect::<Vec<_>>();

    let device_details = device_records
        .into_iter()
        .map(
            |device| crate::services::controls::BluetoothConnectedDevice {
                address: device.address,
                name: device.name,
                connected: device.connected,
                paired: device.paired,
                trusted: device.trusted,
                battery_percent: device.battery_percent,
                audio_profiles: device.audio_profiles,
            },
        )
        .collect::<Vec<_>>();

    crate::services::controls::BluetoothDeviceSummary {
        connected_devices,
        device_details,
    }
}

fn device_path_from_objects(objects: &ManagedObjects, address: &str) -> Option<OwnedObjectPath> {
    let normalized = address.to_ascii_lowercase();
    objects.iter().find_map(|(path, interfaces)| {
        let device = parse_device_record(path, interfaces)?;
        (device.address.to_ascii_lowercase() == normalized).then_some(device.path)
    })
}

fn parse_device_record(
    path: &OwnedObjectPath,
    interfaces: &HashMap<zbus::names::OwnedInterfaceName, HashMap<String, OwnedValue>>,
) -> Option<BluezDeviceRecord> {
    let device_props = interface_props(interfaces, DEVICE_INTERFACE)?;
    let address = property_string(device_props, "Address")?;
    let name = property_string(device_props, "Alias")
        .filter(|value| !value.trim().is_empty())
        .or_else(|| property_string(device_props, "Name").filter(|value| !value.trim().is_empty()))
        .unwrap_or_else(|| address.clone());
    let battery_percent = interface_props(interfaces, BATTERY_INTERFACE)
        .and_then(|props| property_u8(props, "Percentage"));

    Some(BluezDeviceRecord {
        path: path.clone(),
        address,
        name,
        connected: property_bool(device_props, "Connected").unwrap_or(false),
        paired: property_bool(device_props, "Paired").unwrap_or(false),
        trusted: property_bool(device_props, "Trusted").unwrap_or(false),
        battery_percent,
        audio_profiles: property_string_vec(device_props, "UUIDs")
            .map(normalize_audio_profile_uuids)
            .unwrap_or_default(),
    })
}

fn interface_props<'a>(
    interfaces: &'a HashMap<zbus::names::OwnedInterfaceName, HashMap<String, OwnedValue>>,
    interface_name: &str,
) -> Option<&'a HashMap<String, OwnedValue>> {
    interfaces.iter().find_map(|(name, props)| {
        if name.as_str() == interface_name {
            Some(props)
        } else {
            None
        }
    })
}

fn property_bool(properties: &HashMap<String, OwnedValue>, key: &str) -> Option<bool> {
    properties
        .get(key)
        .and_then(|value| bool::try_from(value).ok())
}

fn property_u8(properties: &HashMap<String, OwnedValue>, key: &str) -> Option<u8> {
    properties
        .get(key)
        .and_then(|value| u8::try_from(value).ok())
}

fn property_string(properties: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    let value = properties.get(key)?;
    let owned = value.try_clone().ok()?;
    String::try_from(owned).ok()
}

fn property_string_vec(properties: &HashMap<String, OwnedValue>, key: &str) -> Option<Vec<String>> {
    let value = properties.get(key)?;
    let owned = value.try_clone().ok()?;
    Vec::<String>::try_from(owned).ok()
}

fn normalize_audio_profile_uuids(uuids: Vec<String>) -> Vec<String> {
    let mut profiles = Vec::new();
    for uuid in uuids {
        let Some(profile) = normalize_audio_profile_uuid(&uuid) else {
            continue;
        };
        if !profiles.iter().any(|existing| existing == profile) {
            profiles.push(profile.to_string());
        }
    }
    profiles
}

fn normalize_audio_profile_uuid(uuid: &str) -> Option<&'static str> {
    let lower = uuid.to_ascii_lowercase();
    if lower.starts_with("0000110b") {
        Some("A2DP")
    } else if lower.starts_with("0000111e") {
        Some("HFP")
    } else if lower.starts_with("00001108") {
        Some("HSP")
    } else if lower.starts_with("0000110c") {
        Some("AVRCP Target")
    } else if lower.starts_with("0000110e") {
        Some("AVRCP")
    } else {
        None
    }
}

fn parent_adapter_path(device_path: &OwnedObjectPath) -> Option<OwnedObjectPath> {
    let (parent, _) = device_path.as_str().rsplit_once('/')?;
    OwnedObjectPath::try_from(parent.to_string()).ok()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        adapter_state_from_objects, device_summary_from_objects,
        is_relevant_bluetooth_signal_parts, normalize_audio_profile_uuid, parent_adapter_path,
        parse_device_record, DBUS_OBJECT_MANAGER_INTERFACE, DBUS_PROPERTIES_INTERFACE,
        INTERFACES_ADDED_MEMBER, INTERFACES_REMOVED_MEMBER, PROPERTIES_CHANGED_MEMBER,
    };
    use zbus::{
        fdo::ManagedObjects,
        names::OwnedInterfaceName,
        zvariant::{OwnedObjectPath, OwnedValue, Value},
    };

    #[test]
    fn normalize_audio_profile_uuid_maps_known_profiles() {
        assert_eq!(
            normalize_audio_profile_uuid("0000110b-0000-1000-8000-00805f9b34fb"),
            Some("A2DP")
        );
        assert_eq!(
            normalize_audio_profile_uuid("0000111e-0000-1000-8000-00805f9b34fb"),
            Some("HFP")
        );
        assert_eq!(normalize_audio_profile_uuid("deadbeef"), None);
    }

    #[test]
    fn parse_device_record_prefers_alias_and_extracts_battery_and_profiles() {
        let path = object_path("/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF");
        let mut interfaces = HashMap::new();
        interfaces.insert(
            iface_name("org.bluez.Device1"),
            HashMap::from([
                ("Address".to_string(), val_str("AA:BB:CC:DD:EE:FF")),
                ("Alias".to_string(), val_str("WH-1000XM5")),
                ("Name".to_string(), val_str("Fallback Name")),
                ("Connected".to_string(), OwnedValue::from(true)),
                ("Paired".to_string(), OwnedValue::from(true)),
                ("Trusted".to_string(), OwnedValue::from(true)),
                (
                    "UUIDs".to_string(),
                    OwnedValue::try_from(Value::from(vec![
                        "0000110b-0000-1000-8000-00805f9b34fb".to_string(),
                        "0000111e-0000-1000-8000-00805f9b34fb".to_string(),
                        "0000110e-0000-1000-8000-00805f9b34fb".to_string(),
                    ]))
                    .unwrap(),
                ),
            ]),
        );
        interfaces.insert(
            iface_name("org.bluez.Battery1"),
            HashMap::from([("Percentage".to_string(), OwnedValue::from(90_u8))]),
        );

        let device = parse_device_record(&path, &interfaces).unwrap();
        assert_eq!(device.name, "WH-1000XM5");
        assert!(device.connected);
        assert!(device.paired);
        assert!(device.trusted);
        assert_eq!(device.battery_percent, Some(90));
        assert_eq!(
            device.audio_profiles,
            vec!["A2DP".to_string(), "HFP".to_string(), "AVRCP".to_string()]
        );
    }

    #[test]
    fn device_summary_filters_connected_devices_and_keeps_sorted_details() {
        let objects = ManagedObjects::from([
            (
                object_path("/org/bluez/hci0"),
                HashMap::from([(
                    iface_name("org.bluez.Adapter1"),
                    HashMap::from([("Powered".to_string(), OwnedValue::from(true))]),
                )]),
            ),
            (
                object_path("/org/bluez/hci0/dev_11_22_33_44_55_66"),
                HashMap::from([(
                    iface_name("org.bluez.Device1"),
                    HashMap::from([
                        ("Address".to_string(), val_str("11:22:33:44:55:66")),
                        ("Alias".to_string(), val_str("MX Master 3S")),
                        ("Connected".to_string(), OwnedValue::from(false)),
                        ("Paired".to_string(), OwnedValue::from(true)),
                        ("Trusted".to_string(), OwnedValue::from(true)),
                    ]),
                )]),
            ),
            (
                object_path("/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF"),
                HashMap::from([(
                    iface_name("org.bluez.Device1"),
                    HashMap::from([
                        ("Address".to_string(), val_str("AA:BB:CC:DD:EE:FF")),
                        ("Alias".to_string(), val_str("WH-1000XM5")),
                        ("Connected".to_string(), OwnedValue::from(true)),
                        ("Paired".to_string(), OwnedValue::from(true)),
                        ("Trusted".to_string(), OwnedValue::from(true)),
                    ]),
                )]),
            ),
        ]);

        let summary = device_summary_from_objects(&objects);
        assert_eq!(summary.connected_devices, vec!["WH-1000XM5".to_string()]);
        assert_eq!(summary.device_details.len(), 2);
        assert_eq!(summary.device_details[0].name, "MX Master 3S");
        assert_eq!(summary.device_details[1].name, "WH-1000XM5");
    }

    #[test]
    fn adapter_state_extracts_first_available_adapter_power() {
        let objects = ManagedObjects::from([(
            object_path("/org/bluez/hci0"),
            HashMap::from([(
                iface_name("org.bluez.Adapter1"),
                HashMap::from([("Powered".to_string(), OwnedValue::from(true))]),
            )]),
        )]);

        let (path, powered) = adapter_state_from_objects(&objects).unwrap();
        assert_eq!(path.as_str(), "/org/bluez/hci0");
        assert!(powered);
    }

    #[test]
    fn parent_adapter_path_trims_device_segment() {
        let parent = parent_adapter_path(&object_path("/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF"))
            .expect("parent adapter path");
        assert_eq!(parent.as_str(), "/org/bluez/hci0");
    }

    #[test]
    fn signal_filter_accepts_only_relevant_bluez_runtime_signals() {
        assert!(is_relevant_bluetooth_signal_parts(
            Some("/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF"),
            Some(DBUS_PROPERTIES_INTERFACE),
            Some(PROPERTIES_CHANGED_MEMBER)
        ));
        assert!(is_relevant_bluetooth_signal_parts(
            Some("/org/bluez/hci0"),
            Some(DBUS_OBJECT_MANAGER_INTERFACE),
            Some(INTERFACES_ADDED_MEMBER)
        ));
        assert!(is_relevant_bluetooth_signal_parts(
            Some("/org/bluez/hci0"),
            Some(DBUS_OBJECT_MANAGER_INTERFACE),
            Some(INTERFACES_REMOVED_MEMBER)
        ));

        assert!(!is_relevant_bluetooth_signal_parts(
            Some("/org/bluez/hci0"),
            Some("org.freedesktop.DBus"),
            Some("NameOwnerChanged")
        ));
        assert!(!is_relevant_bluetooth_signal_parts(
            Some("/org/freedesktop/UPower"),
            Some(DBUS_PROPERTIES_INTERFACE),
            Some(PROPERTIES_CHANGED_MEMBER)
        ));
        assert!(!is_relevant_bluetooth_signal_parts(
            None,
            Some(DBUS_PROPERTIES_INTERFACE),
            Some(PROPERTIES_CHANGED_MEMBER)
        ));
    }

    fn object_path(path: &str) -> OwnedObjectPath {
        OwnedObjectPath::try_from(path.to_string()).unwrap()
    }

    fn iface_name(name: &str) -> OwnedInterfaceName {
        OwnedInterfaceName::try_from(name.to_string()).unwrap()
    }

    fn val_str(value: &str) -> OwnedValue {
        OwnedValue::try_from(Value::from(value.to_string())).unwrap()
    }
}
