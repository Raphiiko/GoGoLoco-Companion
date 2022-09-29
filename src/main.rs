use rosc::OscPacket;
use std::{
    ffi::CString,
    net::{SocketAddrV4, UdpSocket},
    str::FromStr,
    time::Duration,
};

use vigem_client::{Client, XGamepad, Xbox360Wired};
use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowA, SetForegroundWindow};

fn main() {
    // Set up virtual controller
    println!("Setting up virtual controller...");
    let vigem_client = match vigem_client::Client::connect() {
        Ok(client) => client,
        Err(_) => {
            eprintln!(
                "Failed to connect to the ViGEmBus Driver. Please make sure it is installed."
            );
            return;
        }
    };
    let mut controller =
        vigem_client::Xbox360Wired::new(vigem_client, vigem_client::TargetId::XBOX360_WIRED);
    controller.plugin().unwrap();
    controller.wait_ready().unwrap();
    let mut gamepad = vigem_client::XGamepad {
        buttons: vigem_client::XButtons!(UP | RIGHT | LB | A | X),
        ..Default::default()
    };
    std::thread::sleep(Duration::from_secs(3));
    println!("Virtual Controller Set Up");

    // Connect to VRChat's OSC server
    println!("Connecting to VRChat...");
    let addr = match SocketAddrV4::from_str("127.0.0.1:9001") {
        Ok(addr) => addr,
        Err(_) => {
            eprintln!(
                "Failed to connect to VRChat. Please make sure it is running and that OSC is enabled."
            );
            return;
        }
    };
    let sock = UdpSocket::bind(addr).unwrap();
    println!("Connected to VRChat");

    // Listen for OSC messages
    let mut buf = [0u8; rosc::decoder::MTU];
    let mut last_vrc_emote: i32 = 0;
    loop {
        match sock.recv_from(&mut buf) {
            Ok((size, _)) => {
                let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
                handle_osc_packet(packet, &mut last_vrc_emote, &mut gamepad, &mut controller);
            }
            Err(e) => {
                println!("Error receiving OSC packet from VRChat: {}", e);
                break;
            }
        }
    }
}

fn handle_osc_packet(
    packet: OscPacket,
    last_vrc_emote: &mut i32,
    gamepad: &mut XGamepad,
    controller: &mut Xbox360Wired<Client>,
) {
    match packet {
        OscPacket::Message(msg) => {
            if msg.addr == "/avatar/parameters/VRCEmote" {
                let vrc_emote = match msg.args[0] {
                    rosc::OscType::Int(value) => value,
                    _ => -1,
                };
                if *last_vrc_emote == 212 && vrc_emote == 0 {
                    *last_vrc_emote = vrc_emote;
                    println!("Scaling Radial Menu Closed");
                    if focus("VRChat") {
                        osc_toggle_sequence(gamepad, controller);
                    }
                } else {
                    *last_vrc_emote = vrc_emote;
                }
            }
        }
        _ => (),
    }
}

fn osc_toggle_sequence(gamepad: &mut XGamepad, controller: &mut Xbox360Wired<Client>) {
    println!("Running OSC Toggle Sequence...");
    for _ in 0..6 {
        click_angle(gamepad, controller, 0.0);
    }
    click_angle(gamepad, controller, 190.0);
    for _ in 0..2 {
        click_angle(gamepad, controller, 180.0);
    }
    // Reset thumb stick
    gamepad.thumb_rx = 0;
    gamepad.thumb_ry = 0;
    let _ = controller.update(&gamepad);
    println!("OSC Toggle Sequence Complete");
}

fn click_angle(gamepad: &mut XGamepad, controller: &mut Xbox360Wired<Client>, angle: f64) {
    println!("Clicking Menu option at {} degrees", angle);
    // Set thumb stick angle if needed
    let x = (angle.to_radians().sin() * 30000.0) as i16;
    let y = (angle.to_radians().cos() * 30000.0) as i16;
    if x != gamepad.thumb_rx || y != gamepad.thumb_ry {
        gamepad.thumb_rx = x;
        gamepad.thumb_ry = y;
        let _ = controller.update(&gamepad);
        std::thread::sleep(Duration::from_millis(200));
    }
    // Press trigger
    gamepad.right_trigger = 255;
    let _ = controller.update(&gamepad);
    std::thread::sleep(Duration::from_millis(100));
    // Release trigger
    gamepad.right_trigger = 0;
    let _ = controller.update(&gamepad);
    std::thread::sleep(Duration::from_millis(300));
}

fn focus(window_name: &str) -> bool {
    let c_window_name = CString::new(window_name).unwrap();
    unsafe {
        let window_handle = FindWindowA(std::ptr::null_mut(), c_window_name.as_ptr() as *const u8);
        if window_handle > 0 {
            SetForegroundWindow(window_handle);
            return true;
        }
    };
    return false;
}
