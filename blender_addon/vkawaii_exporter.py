bl_info = {
    "name": "VKawaii Companion Exporter",
    "blender": (3, 0, 0),
    "category": "Import-Export",
    "description": "Exports models, baked shaders, and physics data for the VKawaii Engine",
}

import bpy
import json
import os

class VKawaiiExportPanel(bpy.types.Panel):
    bl_label = "VKawaii Export"
    bl_idname = "VIEW3D_PT_vkawaii_export"
    bl_space_type = 'VIEW_3D'
    bl_region_type = 'UI'
    bl_category = 'VKawaii'

    def draw(self, context):
        layout = self.layout
        scene = context.scene

        layout.label(text="Physics Configuration:")
        layout.operator("vkawaii.mark_spring_bone", text="Mark Selected Bone as Spring")

        layout.separator()
        layout.label(text="Export:")
        layout.operator("vkawaii.export_all", text="Export to VKawaii")

class VKawaiiMarkSpringBone(bpy.types.Operator):
    bl_idname = "vkawaii.mark_spring_bone"
    bl_label = "Mark as Spring Bone"

    def execute(self, context):
        bone = context.active_pose_bone
        if bone:
            bone.bone["vkawaii_spring"] = True
            bone.bone["vkawaii_stiffness"] = 1.0
            bone.bone["vkawaii_radius"] = 0.05
            self.report({'INFO'}, f"Marked {bone.name} as Spring Bone")
        else:
            self.report({'WARNING'}, "No pose bone selected")
        return {'FINISHED'}

class VKawaiiExportAll(bpy.types.Operator):
    bl_idname = "vkawaii.export_all"
    bl_label = "Export to VKawaii"

    def execute(self, context):
        # Grabbing the Spring bones from the Blender armature by Hand
        # So the physics Engine can Rebuild the node graph later.
        spring_bones = []
        constraints = []
        for obj in bpy.context.scene.objects:
            if obj.type == 'ARMATURE':
                for pb in obj.pose.bones:
                    if pb.bone.get("vkawaii_spring"):
                        spring_bones.append({
                            "name": pb.name,
                            "stiffness": pb.bone.get("vkawaii_stiffness", 1.0),
                            "radius": pb.bone.get("vkawaii_radius", 0.05)
                        })

                    for c in pb.constraints:
                        if c.type in {'COPY_ROTATION', 'LIMIT_ROTATION', 'DAMPED_TRACK', 'IK'}:
                            c_data = {
                                "bone": pb.name,
                                "type": c.type,
                                "influence": c.influence
                            }
                            if hasattr(c, "target") and c.target:
                                c_data["target"] = c.target.name
                            if hasattr(c, "subtarget") and c.subtarget:
                                c_data["subtarget"] = c.subtarget
                            constraints.append(c_data)

        # Look for Shape Keys that are Driven by Bone rotations.
        # This makes stuff like eye Blinking or mouth Movement happen automatically
        # Based on how the Bones are posed.
        blendshape_drivers = []
        for obj in bpy.context.scene.objects:
            if obj.type == 'MESH' and obj.data.shape_keys:
                if obj.data.shape_keys.animation_data and obj.data.shape_keys.animation_data.drivers:
                    for driver in obj.data.shape_keys.animation_data.drivers:
                        if driver.data_path.startswith('key_blocks["') and driver.data_path.endswith('"].value'):
                            shape_key_name = driver.data_path.split('"')[1]

                            # Only handle Simple Drivers with one Variable and a linear Expression
                            if len(driver.driver.variables) == 1:
                                var = driver.driver.variables[0]
                                if var.type == 'TRANSFORMS' and var.targets[0].id and var.targets[0].id.type == 'ARMATURE':
                                    target = var.targets[0]
                                    if target.bone_target and "ROT" in target.transform_type:
                                        expr = driver.driver.expression.replace(" ", "")
                                        coefficient = 1.0

                                        # Simple linear Mapping parsing: 'var * 2.0', '2.0 * var', or just 'var'
                                        if expr == var.name:
                                            coefficient = 1.0
                                        elif expr.startswith(var.name + "*"):
                                            try: coefficient = float(expr.split("*")[1])
                                            except: continue
                                        elif expr.endswith("*" + var.name):
                                            try: coefficient = float(expr.split("*")[0])
                                            except: continue
                                        else:
                                            continue

                                        blendshape_drivers.append({
                                            "shape_key": shape_key_name,
                                            "bone": target.bone_target,
                                            "axis": target.transform_type,
                                            "coefficient": coefficient
                                        })

        # The Manifest makes sure the engine knows what to do with the assets,
        # Ensuring the .glb and Physics Data match up.
        temp_dir = bpy.path.abspath("//")
        if not temp_dir:
            temp_dir = bpy.app.tempdir
            
        json_path = os.path.join(temp_dir, "manifest.json")
        with open(json_path, 'w') as f:
            json.dump({
                "version": "1.0",
                "type": "avatar",
                "name": vkw_filename,
                "avatar_config": {
                    "constraints": constraints,
                    "spring_bones": spring_bones,
                    "blendshape_drivers": blendshape_drivers
                }
                "spring_bones": spring_bones
            }, f, indent=4)

        # Just using Blender's built-in GLB exporter Since it handles baking PBR nodes
        # Into standard glTF materials that the WGPU renderer can Understand.
        gltf_path = os.path.join(temp_dir, "model.glb")
        bpy.ops.export_scene.gltf(filepath=gltf_path, export_format='GLB')

        # The .vkw format Packs all the avatar Data into one File so nothing gets lost.
        import zipfile
        vkw_filename = bpy.path.display_name_from_filepath(bpy.data.filepath)
        if not vkw_filename:
            vkw_filename = "avatar"
        vkw_path = os.path.join(temp_dir, f"{vkw_filename}.vkw")
        
        with zipfile.ZipFile(vkw_path, 'w', zipfile.ZIP_DEFLATED) as zipf:
            zipf.write(json_path, "manifest.json")
            zipf.write(gltf_path, "model.glb")
            
        # Cleaning up the Temp files so they don't clutter the Project folder.
        if os.path.exists(json_path):
            os.remove(json_path)
        if os.path.exists(gltf_path):
            os.remove(gltf_path)

        self.report({'INFO'}, f"Exported VKawaii Avatar to {vkw_path}")
        return {'FINISHED'}

def register():
    bpy.utils.register_class(VKawaiiExportPanel)
    bpy.utils.register_class(VKawaiiMarkSpringBone)
    bpy.utils.register_class(VKawaiiExportAll)

def unregister():
    bpy.utils.unregister_class(VKawaiiExportPanel)
    bpy.utils.unregister_class(VKawaiiMarkSpringBone)
    bpy.utils.unregister_class(VKawaiiExportAll)

if __name__ == "__main__":
    register()
