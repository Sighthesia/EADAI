use eadai::serial::{SerialDeviceTransport, describe_visible_devices};
use serialport::{SerialPortInfo, SerialPortType, UsbPortInfo};

#[test]
fn linux_scan_filters_builtin_ttys_but_keeps_usb_metadata() {
    let devices = describe_visible_devices(vec![
        SerialPortInfo {
            port_name: "/dev/ttyS0".to_string(),
            port_type: SerialPortType::PciPort,
        },
        SerialPortInfo {
            port_name: "/dev/ttyUSB0".to_string(),
            port_type: SerialPortType::UsbPort(UsbPortInfo {
                vid: 0x10c4,
                pid: 0xea60,
                serial_number: Some("0001".to_string()),
                manufacturer: Some("Silicon Labs".to_string()),
                product: Some("CP2102 USB to UART Bridge Controller".to_string()),
            }),
        },
    ]);

    if cfg!(target_os = "linux") {
        assert_eq!(devices.len(), 1);
    } else {
        assert_eq!(devices.len(), 2);
    }

    let usb_device = devices
        .iter()
        .find(|device| device.port_name == "/dev/ttyUSB0")
        .expect("usb device remains visible");

    assert_eq!(
        usb_device.display_name,
        "CP2102 USB to UART Bridge Controller"
    );
    assert_eq!(usb_device.port_type, SerialDeviceTransport::Usb);
    assert_eq!(usb_device.manufacturer.as_deref(), Some("Silicon Labs"));
    assert_eq!(
        usb_device.product.as_deref(),
        Some("CP2102 USB to UART Bridge Controller")
    );
    assert_eq!(usb_device.serial_number.as_deref(), Some("0001"));
    assert_eq!(usb_device.vid, Some(0x10c4));
    assert_eq!(usb_device.pid, Some(0xea60));
}

#[test]
fn usb_devices_sort_before_unknown_ports() {
    let devices = describe_visible_devices(vec![
        SerialPortInfo {
            port_name: "/dev/ttyACM0".to_string(),
            port_type: SerialPortType::Unknown,
        },
        SerialPortInfo {
            port_name: "/dev/ttyUSB0".to_string(),
            port_type: SerialPortType::UsbPort(UsbPortInfo {
                vid: 0x1a86,
                pid: 0x7523,
                serial_number: None,
                manufacturer: Some("QinHeng Electronics".to_string()),
                product: Some("USB-SERIAL CH340".to_string()),
            }),
        },
    ]);

    assert_eq!(devices[0].port_name, "/dev/ttyUSB0");
    assert_eq!(devices[1].port_name, "/dev/ttyACM0");
}
