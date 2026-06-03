using UnityEngine;
using UnityEditor;
using System.IO;
using System.IO.Compression;
using System.Collections.Generic;

namespace VKawaii
{
    public class VKawaiiExporter : EditorWindow
    {
        private GameObject targetAvatar;
        private string outputDir = "Assets/Exports";

        [MenuItem("VKawaii/Export .VKW Avatar")]
        public static void ShowWindow()
        {
            GetWindow<VKawaiiExporter>("VKawaii Exporter");
        }

        void OnGUI()
        {
            GUILayout.Label("VKawaii .VKW Exporter", EditorStyles.boldLabel);

            targetAvatar = (GameObject)EditorGUILayout.ObjectField("Target Avatar", targetAvatar, typeof(GameObject), true);
            
            GUILayout.BeginHorizontal();
            outputDir = EditorGUILayout.TextField("Output Dir", outputDir);
            if (GUILayout.Button("Browse", GUILayout.Width(70)))
            {
                string path = EditorUtility.OpenFolderPanel("Select Output Directory", outputDir, "");
                if (!string.IsNullOrEmpty(path))
                {
                    outputDir = path;
                }
            }
            GUILayout.EndHorizontal();

            GUILayout.Space(20);

            if (GUILayout.Button("Export to .VKW", GUILayout.Height(40)))
            {
                if (targetAvatar == null)
                {
                    EditorUtility.DisplayDialog("Error", "Please select a target avatar to export.", "OK");
                    return;
                }
                
                ExportAvatar();
            }
        }

        private void ExportAvatar()
        {
            if (!Directory.Exists(outputDir))
            {
                Directory.CreateDirectory(outputDir);
            }

            string avatarName = targetAvatar.name;
            string tempDir = Path.Combine(Application.temporaryCachePath, "VKawaiiExport_" + avatarName);
            
            if (Directory.Exists(tempDir))
            {
                Directory.Delete(tempDir, true);
            }
            Directory.CreateDirectory(tempDir);

            try
            {
                // 1. Export GLB (Assuming UniGLTF or similar is installed in the project)
                // This is a placeholder for the actual GLTF export logic depending on what the user uses (VRM0, VRM1, UnityGLTF)
                string glbPath = Path.Combine(tempDir, "model.glb");
                ExportGLB(targetAvatar, glbPath);

                // 2. Generate manifest.json
                string manifestPath = Path.Combine(tempDir, "manifest.json");
                string manifestJson = $@"{{
    ""version"": ""1.0"",
    ""type"": ""avatar"",
    ""name"": ""{avatarName}"",
    ""materials"": []
}}";
                File.WriteAllText(manifestPath, manifestJson);

                // 3. Extract DXBC Shaders (Dummy implementation for now)
                // We would use BuildPipeline.BuildAssetBundles to compile shaders for Windows Standalone
                // and extract the .dxbc chunks from the resulting AssetBundle.
                string shadersDir = Path.Combine(tempDir, "shaders");
                Directory.CreateDirectory(shadersDir);
                ExtractShaders(targetAvatar, shadersDir);

                // 4. Zip into .vkw
                string finalVkwPath = Path.Combine(outputDir, avatarName + ".vkw");
                if (File.Exists(finalVkwPath))
                {
                    File.Delete(finalVkwPath);
                }
                
                ZipFile.CreateFromDirectory(tempDir, finalVkwPath);

                EditorUtility.DisplayDialog("Success", $"Successfully exported {avatarName}.vkw to {outputDir}!", "OK");
                EditorUtility.RevealInFinder(finalVkwPath);
            }
            catch (System.Exception e)
            {
                Debug.LogError($"[VKawaii Exporter] Failed to export: {e.Message}\n{e.StackTrace}");
                EditorUtility.DisplayDialog("Export Failed", "Check the Unity Console for details.", "OK");
            }
            finally
            {
                // Cleanup temp dir
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private void ExportGLB(GameObject target, string path)
        {
            Debug.Log($"[VKawaii Exporter] Exporting GLB to {path}...");
            // TODO: Hook into VRM/UniGLTF exporter API here
            // Example for UniGLTF:
            // var gltfData = new UniGLTF.glTF();
            // var exporter = new UniGLTF.glTFExporter(gltfData);
            // exporter.Prepare(target);
            // exporter.Export();
            // File.WriteAllBytes(path, gltfData.ToGlbBytes());
            
            // For now, we write a dummy file so the zipping succeeds
            File.WriteAllBytes(path, new byte[] { 0x67, 0x6C, 0x54, 0x46 }); // "glTF" magic bytes
        }

        private void ExtractShaders(GameObject target, string shadersDir)
        {
            Debug.Log($"[VKawaii Exporter] Extracting compiled shaders to {shadersDir}...");
            // Extract all materials from the target's Renderers
            var renderers = target.GetComponentsInChildren<Renderer>(true);
            var processedShaders = new HashSet<Shader>();

            foreach (var renderer in renderers)
            {
                foreach (var mat in renderer.sharedMaterials)
                {
                    if (mat != null && mat.shader != null)
                    {
                        if (processedShaders.Add(mat.shader))
                        {
                            // In a full implementation, we would assign this shader to an AssetBundle,
                            // call BuildPipeline.BuildAssetBundles for StandaloneWindows64,
                            // and read the resulting DXBC chunks.
                            
                            // For now, we write a dummy .dxbc file
                            string shaderName = mat.shader.name.Replace("/", "_");
                            string dummyPath = Path.Combine(shadersDir, shaderName + ".dxbc");
                            File.WriteAllBytes(dummyPath, new byte[] { 0x44, 0x58, 0x42, 0x43 }); // "DXBC" magic bytes
                        }
                    }
                }
            }
        }
    }
}
