use nalgebra::{UnitQuaternion, Vector3};
use rosc::{OscPacket, OscType};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::{Arc, RwLock};
use std::thread;

#[derive(Clone, Debug, Default)]
pub struct BoneTransform {
    pub position: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
}

#[derive(Clone, Debug, Default)]
pub struct VmcState {
    pub bones: HashMap<String, BoneTransform>,
    pub blend_shapes: HashMap<String, f32>,
    pub root_transform: BoneTransform,
}

pub struct VmcReceiver {
    pub state: Arc<RwLock<VmcState>>,
    is_running: Arc<RwLock<bool>>,
}

impl Default for VmcReceiver {
    fn default() -> Self {
        Self::new()
    }
}

impl VmcReceiver {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(VmcState::default())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub fn start(&self, addr: &str) -> anyhow::Result<()> {
        let mut is_running = self.is_running.write().unwrap();
        if *is_running {
            return Ok(());
        }
        *is_running = true;

        let state = Arc::clone(&self.state);
        let is_running_thread = Arc::clone(&self.is_running);
        let addr = addr.to_string();

        thread::spawn(move || {
            let socket = match UdpSocket::bind(&addr) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("VMC Receiver failed to bind to {}: {}", addr, e);
                    let mut running = is_running_thread.write().unwrap();
                    *running = false;
                    return;
                }
            };
            socket.set_read_timeout(Some(std::time::Duration::from_millis(100))).unwrap();

            let mut buf = [0u8; 2048];

            while *is_running_thread.read().unwrap() {
                match socket.recv_from(&mut buf) {
                    Ok((size, _)) => {
                        if let Ok((_, packet)) = rosc::decoder::decode_udp(&buf[..size]) {
                            Self::handle_packet(packet, &state);
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                        // Timeout reached, loop around to check is_running_thread again
                    }
                    Err(e) => {
                        eprintln!("VMC Receiver socket error: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    pub fn stop(&self) {
        let mut is_running = self.is_running.write().unwrap();
        *is_running = false;
    }

    fn handle_packet(packet: OscPacket, state: &Arc<RwLock<VmcState>>) {
        match packet {
            OscPacket::Message(msg) => {
                let mut state = state.write().unwrap();
                Self::parse_message(msg, &mut state);
            }
            OscPacket::Bundle(bundle) => {
                let mut state = state.write().unwrap();
                for packet in bundle.content {
                    if let OscPacket::Message(msg) = packet {
                        Self::parse_message(msg, &mut state);
                    }
                }
            }
        }
    }

    fn parse_message(msg: rosc::OscMessage, state: &mut VmcState) {
        match msg.addr.as_str() {
            "/VMC/Ext/Bone/Pos" => {
                if let [OscType::String(name), OscType::Float(px), OscType::Float(py), OscType::Float(pz), OscType::Float(rx), OscType::Float(ry), OscType::Float(rz), OscType::Float(rw)] =
                    &msg.args[..]
                {
                    state.bones.insert(
                        name.clone(),
                        BoneTransform {
                            position: Vector3::new(*px, *py, *pz),
                            rotation: UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(
                                *rw, *rx, *ry, *rz,
                            )),
                        },
                    );
                }
            }
            "/VMC/Ext/Blend/Val" => {
                if let [OscType::String(name), OscType::Float(val)] = &msg.args[..] {
                    state.blend_shapes.insert(name.clone(), *val);
                }
            }
            "/VMC/Ext/Root/Pos" => {
                if let [OscType::String(_name), OscType::Float(px), OscType::Float(py), OscType::Float(pz), OscType::Float(rx), OscType::Float(ry), OscType::Float(rz), OscType::Float(rw)] =
                    &msg.args[..]
                {
                    state.root_transform = BoneTransform {
                        position: Vector3::new(*px, *py, *pz),
                        rotation: UnitQuaternion::from_quaternion(nalgebra::Quaternion::new(
                            *rw, *rx, *ry, *rz,
                        )),
                    };
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rosc::OscMessage;

    #[test]
    fn test_parse_bone_pos() {
        let mut state = VmcState::default();
        let msg = OscMessage {
            addr: "/VMC/Ext/Bone/Pos".to_string(),
            args: vec![
                OscType::String("Head".to_string()),
                OscType::Float(1.0),
                OscType::Float(2.0),
                OscType::Float(3.0),
                OscType::Float(0.0),
                OscType::Float(0.0),
                OscType::Float(0.0),
                OscType::Float(1.0),
            ],
        };

        VmcReceiver::parse_message(msg, &mut state);

        let head = state.bones.get("Head").unwrap();
        assert_eq!(head.position, Vector3::new(1.0, 2.0, 3.0));
        assert_eq!(head.rotation.w, 1.0);
    }

    #[test]
    fn test_parse_blend_val() {
        let mut state = VmcState::default();
        let msg = OscMessage {
            addr: "/VMC/Ext/Blend/Val".to_string(),
            args: vec![OscType::String("Joy".to_string()), OscType::Float(0.5)],
        };

        VmcReceiver::parse_message(msg, &mut state);

        assert_eq!(*state.blend_shapes.get("Joy").unwrap(), 0.5);
    }

    #[test]
    fn test_parse_root_pos() {
        let mut state = VmcState::default();
        let msg = OscMessage {
            addr: "/VMC/Ext/Root/Pos".to_string(),
            args: vec![
                OscType::String("Root".to_string()),
                OscType::Float(10.0),
                OscType::Float(20.0),
                OscType::Float(30.0),
                OscType::Float(0.0),
                OscType::Float(1.0),
                OscType::Float(0.0),
                OscType::Float(0.0),
            ],
        };

        VmcReceiver::parse_message(msg, &mut state);

        assert_eq!(
            state.root_transform.position,
            Vector3::new(10.0, 20.0, 30.0)
        );
        assert_eq!(state.root_transform.rotation.i, 0.0);
        assert_eq!(state.root_transform.rotation.j, 1.0);
    }
}
