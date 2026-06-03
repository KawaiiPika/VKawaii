use anyhow::{Result, bail};

use std::os::raw::{c_void, c_int};
use std::ptr;

#[repr(C)]
pub struct dxil_spv_compiled_spirv {
    pub data: *const u8,
    pub size: usize,
}

#[allow(dead_code)]
extern "C" {
    fn dxil_spv_parse_dxil_blob(data: *const c_void, size: usize, blob: *mut *mut c_void) -> c_int;
    fn dxil_spv_parsed_blob_free(blob: *mut c_void);

    fn dxil_spv_create_converter(blob: *mut c_void, converter: *mut *mut c_void) -> c_int;
    fn dxil_spv_converter_free(converter: *mut c_void);
    
    fn dxil_spv_converter_run(converter: *mut c_void) -> c_int;
    fn dxil_spv_converter_get_compiled_spirv(converter: *mut c_void, compiled: *mut dxil_spv_compiled_spirv) -> c_int;
}

/// Translates a raw DXBC/DXIL byte slice into SPIR-V.
pub fn translate_dxbc_to_spirv(dxbc_bytes: &[u8]) -> Result<Vec<u8>> {
    unsafe {
        let mut blob: *mut c_void = ptr::null_mut();
        
        let res = dxil_spv_parse_dxil_blob(dxbc_bytes.as_ptr() as *const c_void, dxbc_bytes.len(), &mut blob);
        if res != 0 || blob.is_null() {
            bail!("dxil_spv_parse_dxil_blob failed with error code: {}", res);
        }

        let mut converter: *mut c_void = ptr::null_mut();
        let res = dxil_spv_create_converter(blob, &mut converter);
        if res != 0 || converter.is_null() {
            dxil_spv_parsed_blob_free(blob);
            bail!("dxil_spv_create_converter failed with error code: {}", res);
        }

        let res = dxil_spv_converter_run(converter);
        if res != 0 {
            dxil_spv_converter_free(converter);
            dxil_spv_parsed_blob_free(blob);
            bail!("dxil_spv_converter_run failed with error code: {}", res);
        }

        let mut compiled = dxil_spv_compiled_spirv {
            data: ptr::null(),
            size: 0,
        };

        let res = dxil_spv_converter_get_compiled_spirv(converter, &mut compiled);
        
        // Copy the SPIR-V data to a Rust Vec before freeing the converter
        let spirv_vec = if res == 0 && !compiled.data.is_null() && compiled.size > 0 {
            let slice = std::slice::from_raw_parts(compiled.data, compiled.size);
            Ok(slice.to_vec())
        } else {
            Err(anyhow::anyhow!("dxil_spv_converter_get_compiled_spirv failed with error code: {}", res))
        };

        dxil_spv_converter_free(converter);
        dxil_spv_parsed_blob_free(blob);

        spirv_vec
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_empty() {
        let result = translate_dxbc_to_spirv(&[]);
        assert!(result.is_err());
    }
}
