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
        # We need to manually capture spring bone configurations from the active Blender armature 
        # so the physics engine can reconstruct the node graph at runtime.
        spring_bones = []
        for obj in bpy.context.scene.objects:
            if obj.type == 'ARMATURE':
                for bone in obj.pose.bones:
                    if bone.bone.get("vkawaii_spring"):
                        spring_bones.append({
                            "name": bone.name,
                            "stiffness": bone.bone.get("vkawaii_stiffness", 1.0),
                            "radius": bone.bone.get("vkawaii_radius", 0.05)
                        })

        # The manifest acts as the source of truth for the engine's asset parser,
        # ensuring the .glb and physics data are version-compatible.
        temp_dir = bpy.path.abspath("//")
        if not temp_dir:
            temp_dir = bpy.app.tempdir
            
        json_path = os.path.join(temp_dir, "manifest.json")
        with open(json_path, 'w') as f:
            json.dump({
                "version": "1.0",
                "type": "avatar",
                "spring_bones": spring_bones
            }, f, indent=4)

        # We rely on Blender's native GLB exporter because it automatically bakes standard PBR nodes
        # into standard glTF materials, which our WGPU renderer natively supports.
        gltf_path = os.path.join(temp_dir, "model.glb")
        bpy.ops.export_scene.gltf(filepath=gltf_path, export_format='GLB')

        # The .vkw format encapsulates all avatar data into a single file to prevent missing asset errors.
        import zipfile
        vkw_filename = bpy.path.display_name_from_filepath(bpy.data.filepath)
        if not vkw_filename:
            vkw_filename = "avatar"
        vkw_path = os.path.join(temp_dir, f"{vkw_filename}.vkw")
        
        with zipfile.ZipFile(vkw_path, 'w', zipfile.ZIP_DEFLATED) as zipf:
            zipf.write(json_path, "manifest.json")
            zipf.write(gltf_path, "model.glb")
            
        # We delete the intermediate files to avoid polluting the user's project directory.
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
