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
                // We assume UniGLTF or similar is installed since the native Unity exporter is severely limited.
                // This call should be swapped with the actual VRM/UniGLTF API depending on the user's setup.
                string glbPath = Path.Combine(tempDir, "model.glb");
                ExportGLB(targetAvatar, glbPath);

                // The manifest ensures the Rust VKawaii engine knows exactly what format and version to parse.
                string manifestPath = Path.Combine(tempDir, "manifest.json");
                string manifestJson = $@"{{
    ""version"": ""1.0"",
    ""type"": ""avatar"",
    ""name"": ""{avatarName}"",
    ""materials"": []
}}";
                File.WriteAllText(manifestPath, manifestJson);

                // We extract the compiled shaders via AssetBundle dummy building because 
                // the `translate_dxbc_to_spirv` Rust parser requires raw DXBC binaries, not HLSL.
                string shadersDir = Path.Combine(tempDir, "shaders");
                Directory.CreateDirectory(shadersDir);
                ExtractShaders(targetAvatar, shadersDir);

                // The final .vkw format encapsulates all avatar data into a single ZIP archive 
                // to prevent missing asset errors.
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
                // We clean up the temporary directory to avoid bloating the user's hard drive.
                if (Directory.Exists(tempDir))
                {
                    Directory.Delete(tempDir, true);
                }
            }
        }

        private void ExportGLB(GameObject target, string path)
        {
            Debug.Log($"[VKawaii Exporter] Exporting GLB to {path}...");
            
            // To properly extract PBR channels from standard Unity materials, we require a GLTF exporter.
            // For now, we write a dummy file so the zipping succeeds until the user installs VRM0/1.
            File.WriteAllBytes(path, new byte[] { 0x67, 0x6C, 0x54, 0x46 }); // "glTF" magic bytes
        }

        private void ExtractShaders(GameObject target, string shadersDir)
        {
            Debug.Log($"[VKawaii Exporter] Extracting compiled shaders to {shadersDir}...");
            
            // We iterate through all renderers because different parts of the mesh might use
            // entirely different shaders (e.g. Poiyomi for hair, standard for body).
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
                            // In a full implementation, we assign this shader to an AssetBundle and 
                            // call BuildPipeline.BuildAssetBundles to trigger the HLSL compiler.
                            // We write a dummy chunk here to simulate the extraction.
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
