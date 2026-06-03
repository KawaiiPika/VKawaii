use naga::{back::wgsl, front::spv, valid::{Capabilities, ValidationFlags, Validator}, Module, AddressSpace, TypeInner};
use anyhow::{Result, Context};

#[derive(Debug, Clone, PartialEq)]
pub enum BindingType {
    UniformBuffer { size: u32 },
    Texture,
    Sampler,
    Other,
}

#[derive(Debug, Clone)]
pub struct ShaderBinding {
    pub name: Option<String>,
    pub group: u32,
    pub binding: u32,
    pub binding_type: BindingType,
}

#[derive(Debug, Default)]
pub struct ShaderReflection {
    pub bindings: Vec<ShaderBinding>,
}

/// Transpiles a raw SPIR-V binary array into a WGSL string that can be fed into BlueEngine/WGPU.
/// Also returns reflection data about the bindings the shader expects.
pub fn transpile_spirv_to_wgsl(spirv_bytes: &[u8]) -> Result<(String, ShaderReflection)> {
    let options = spv::Options {
        adjust_coordinate_space: false,
        strict_capabilities: false,
        block_ctx_dump_prefix: None,
    };
    
    let module = spv::parse_u8_slice(spirv_bytes, &options)
        .context("Failed to parse SPIR-V module")?;

    let reflection = reflect_shader_bindings(&module);

    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    let module_info = validator
        .validate(&module)
        .context("Failed to validate Naga module translated from SPIR-V")?;

    let mut wgsl_out = String::new();
    let mut writer = wgsl::Writer::new(&mut wgsl_out, wgsl::WriterFlags::empty());
    writer
        .write(&module, &module_info)
        .context("Failed to write WGSL output")?;

    Ok((wgsl_out, reflection))
}

pub fn reflect_shader_bindings(module: &Module) -> ShaderReflection {
    let mut reflection = ShaderReflection::default();

    for (_, global_var) in module.global_variables.iter() {
        if let Some(binding) = &global_var.binding {
            let binding_type = match global_var.space {
                AddressSpace::Uniform => {
                    let type_data = &module.types[global_var.ty];
                    let size = type_data.inner.size(module.to_ctx());
                    BindingType::UniformBuffer { size }
                },
                AddressSpace::Handle => {
                    let type_data = &module.types[global_var.ty];
                    match type_data.inner {
                        TypeInner::Image { .. } => BindingType::Texture,
                        TypeInner::Sampler { .. } => BindingType::Sampler,
                        _ => BindingType::Other,
                    }
                },
                _ => BindingType::Other,
            };

            reflection.bindings.push(ShaderBinding {
                name: global_var.name.clone(),
                group: binding.group,
                binding: binding.binding,
                binding_type,
            });
        }
    }

    reflection
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transpile_empty_spirv() {
        let result = transpile_spirv_to_wgsl(&[]);
        assert!(result.is_err(), "Empty SPIR-V should return a parsing error");
    }
}
