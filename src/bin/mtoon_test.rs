use gltf::Gltf;

fn main() {
    let path = "VRM/Vita_clothing.vrm";
    let gltf = Gltf::open(path).unwrap();
    let json = gltf.document.into_json();

    if let Some(extensions) = &json.extensions {
        println!(
            "Root Extensions: {:?}",
            extensions.others.keys().collect::<Vec<_>>()
        );
        if let Some(vrm) = extensions.others.get("VRM") {
            println!("Found VRM 0.0 Extension!");
            if let Some(materials) = vrm.get("materialProperties") {
                if let Some(materials_array) = materials.as_array() {
                    for (i, mat) in materials_array.iter().enumerate() {
                        if let Some(name) = mat.get("name") {
                            println!("  Material {}: {}", i, name);

                            // Printing Shade color and Texture if it has it
                            if let Some(float_props) = mat.get("floatProperties") {
                                if let Some(shade_shift) = float_props.get("_ShadeShift") {
                                    println!("    ShadeShift: {:?}", shade_shift);
                                }
                            }
                            if let Some(vector_props) = mat.get("vectorProperties") {
                                if let Some(color) = vector_props.get("_Color") {
                                    println!("    Color: {:?}", color);
                                }
                                if let Some(shade_color) = vector_props.get("_ShadeColor") {
                                    println!("    ShadeColor: {:?}", shade_color);
                                }
                            }
                        }
                    }
                }

                if let Some(secondary_animation) = vrm.get("secondaryAnimation") {
                    println!("Found VRM 0.0 secondaryAnimation (Spring Bones)!");
                    if let Some(bone_groups) = secondary_animation.get("boneGroups") {
                        if let Some(groups) = bone_groups.as_array() {
                            println!("  Spring Bone Groups: {}", groups.len());
                            for (i, group) in groups.iter().enumerate() {
                                let comment = group
                                    .get("comment")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("No comment");
                                let stiff = group
                                    .get("stiffiness")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                let drag = group
                                    .get("dragForce")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                let hit_radius = group
                                    .get("hitRadius")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                println!(
                                    "    Group {}: '{}' (stiffness: {}, drag: {}, radius: {})",
                                    i, comment, stiff, drag, hit_radius
                                );
                                if let Some(bones) = group.get("bones").and_then(|v| v.as_array()) {
                                    println!("      Bones: {:?}", bones);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
