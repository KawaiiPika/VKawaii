#[cfg(windows)]
use libloading::{Library, Symbol};
#[cfg(windows)]
use std::ffi::CString;
#[cfg(windows)]
use windows::core::Interface;

#[cfg(windows)]
type SpoutCreateSender = unsafe extern "system" fn(name: *const i8, width: u32, height: u32, graphics_mode: u32) -> bool;
#[cfg(windows)]
type SpoutSendTexture = unsafe extern "system" fn(p_resource: *mut std::ffi::c_void) -> bool;
#[cfg(windows)]
type SpoutReleaseSender = unsafe extern "system" fn();
#[cfg(windows)]
type SpoutUpdateSender = unsafe extern "system" fn(name: *const i8, width: u32, height: u32) -> bool;

#[cfg(windows)]
pub struct SpoutSender {
    send_texture_fn: SpoutSendTexture,
    release_sender_fn: SpoutReleaseSender,
    update_sender_fn: SpoutUpdateSender,
    _lib: Library,
    name: String,
    width: u32,
    height: u32,
    initialized: bool,
}

#[cfg(not(windows))]
pub struct SpoutSender {}

impl SpoutSender {
    pub fn new(_name: &str) -> Option<Self> {
        #[cfg(windows)]
        {
            unsafe {
                let lib = Library::new("SpoutLibrary.dll").ok()?;
                let create_sender: Symbol<SpoutCreateSender> = lib.get(b"CreateSender").ok()?;
                let send_texture: Symbol<SpoutSendTexture> = lib.get(b"SendTexture").ok()?;
                let release_sender: Symbol<SpoutReleaseSender> = lib.get(b"ReleaseSender").ok()?;
                let update_sender: Symbol<SpoutUpdateSender> = lib.get(b"UpdateSender").ok()?;

                // Dereference Symbols to get the raw function pointers.
                // This allows the Symbols (which borrow the Library) to be dropped,
                // While the pointers remain Valid as long as we keep the Library alive in the struct.
                let send_texture_fn = *send_texture;
                let release_sender_fn = *release_sender;
                let update_sender_fn = *update_sender;

                let c_name = CString::new(_name).ok()?;
                // 0 = Default (DirectX)
                create_sender(c_name.as_ptr(), 1280, 720, 0);

                Some(Self {
                    send_texture_fn,
                    release_sender_fn,
                    update_sender_fn,
                    _lib: lib,
                    name: _name.to_string(),
                    width: 1280,
                    height: 720,
                    initialized: true,
                })
            }
        }
        #[cfg(not(windows))]
        {
            None
        }
    }

    #[allow(unused_variables)]
    pub fn send_texture(&mut self, view: &blue_engine::TextureView, device: &blue_engine::wgpu::Device) {
        #[cfg(windows)]
        {
            if !self.initialized { return; }

            // Extract the Texture from the view. In Blue Engine 0.10.0, this is a method call.
            let texture = view.texture();
            let width = texture.width();
            let height = texture.height();

            // Handle resolution changes
            if width != self.width || height != self.height {
                unsafe {
                    if let Ok(c_name) = CString::new(self.name.as_str()) {
                        (self.update_sender_fn)(c_name.as_ptr(), width, height);
                        self.width = width;
                        self.height = height;
                    }
                }
            }

            unsafe {
                // Spout2 standard Sendtexture expects a DX11 Resource pointer.
                // On Windows, WGPU typically runs on DX12 in this environment.
                // Extract the raw resource handle and pass it to Spout.
                if let Some(hal_texture) = texture.as_hal::<wgpu_hal::api::Dx12>() {
                    let d3d12_resource = hal_texture.raw_resource();
                    // .as_raw() Requires windows::core::Interface trait
                    let raw_ptr = d3d12_resource.as_raw();
                    (self.send_texture_fn)(raw_ptr as *mut _);
                }
            }
        }
    }
}

#[cfg(windows)]
impl Drop for SpoutSender {
    fn drop(&mut self) {
        unsafe {
            (self.release_sender_fn)();
        }
    }
}

pub struct SpoutSignal {
    #[cfg(windows)]
    pub sender: SpoutSender,
}

impl blue_engine::Signal for SpoutSignal {
    #[allow(unused_variables)]
    fn frame(
        &mut self,
        engine: &mut blue_engine::Engine,
        _encoder: &mut blue_engine::CommandEncoder,
        view: &blue_engine::TextureView,
    ) {
        #[cfg(windows)]
        {
            self.sender.send_texture(view, &engine.renderer.device);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_api_existence() {
        let _ = |view: &blue_engine::TextureView| {
            let tex = view.texture();
            let _w = tex.width();
            let _h = tex.height();
            let _size = tex.size();
            #[cfg(windows)]
            unsafe {
                let _ = tex.as_hal::<wgpu_hal::api::Dx12>();
            }
        };
    }

    #[test]
    fn test_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SpoutSignal>();
    }
}
