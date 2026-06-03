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
        # 1. Gather all bones marked as spring bones
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
        
        # 2. Write the JSON config sidecar file
        export_dir = bpy.path.abspath("//")
        json_path = os.path.join(export_dir, "avatar_config.json")
        with open(json_path, 'w') as f:
            json.dump({"spring_bones": spring_bones}, f, indent=4)
            
        # 3. Export GLTF (which automatically bakes standard PBR nodes)
        # Assuming the user has set up their materials to use standard Principled BSDF
        gltf_path = os.path.join(export_dir, "avatar.glb")
        bpy.ops.export_scene.gltf(filepath=gltf_path, export_format='GLB')
        
        self.report({'INFO'}, f"Exported VKawaii Avatar to {export_dir}")
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
