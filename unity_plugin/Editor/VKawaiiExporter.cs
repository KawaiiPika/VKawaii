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
                // Assuming UniGLTF or Similar is installed since the Native Unity exporter is severely Limited.
                // This Call should be Swapped with the actual VRM/UniGLTF API depending on the Setup.
                string glbPath = Path.Combine(tempDir, "model.glb");
                ExportGLB(targetAvatar, glbPath);

                // The Manifest ensures the Rust VKawaii engine Knows exactly what Format and version to Parse.
                string manifestPath = Path.Combine(tempDir, "manifest.json");
                string manifestJson = $@"{{
    ""version"": ""1.0"",
    ""type"": ""avatar"",
    ""name"": ""{avatarName}"",
    ""materials"": []
}}";
                File.WriteAllText(manifestPath, manifestJson);

                // Extracting the Compiled shaders via AssetBundle dummy Building because 
                // the `translate_dxbc_to_spirv` Rust parser Requires raw DXBC binaries, not HLSL.
                string shadersDir = Path.Combine(tempDir, "shaders");
                Directory.CreateDirectory(shadersDir);
                ExtractShaders(targetAvatar, shadersDir);

                // The Final .vkw format Encapsulates all avatar Data into a Single ZIP archive 
                // to Prevent missing Asset errors.
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
                // Clean up the Temporary directory to Avoid bloating the Hard drive.
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private void ExportGLB(GameObject target, string path)
        {
            Debug.Log($"[VKawaii Exporter] Exporting GLB to {path}...");
            
            // To Properly extract PBR channels from Standard Unity materials, Requires a GLTF exporter.
            // For Now, writing a Dummy file so the Zipping succeeds until VRM0/1 is Installed.
            File.WriteAllBytes(path, new byte[] { 0x67, 0x6C, 0x54, 0x46 }); // "glTF" magic bytes
        }

        private void ExtractShaders(GameObject target, string shadersDir)
        {
            Debug.Log($"[VKawaii Exporter] Extracting compiled shaders to {shadersDir}...");
            
            // Iterate through all Renderers because different Parts of the Mesh might use
            // Entirely different Shaders (e.g. Poiyomi for Hair, standard for Body).
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
                            // In a Full implementation, Assign this shader to an AssetBundle and 
                            // Call BuildPipeline.BuildAssetBundles to Trigger the HLSL compiler.
                            // Writing a Dummy chunk here to Simulate the extraction.
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
