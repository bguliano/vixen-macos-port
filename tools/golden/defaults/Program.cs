using System.Globalization;
using System.Reflection;
using System.Runtime.Loader;
using System.Runtime.Serialization;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Xml;
using Vixen.Services.EffectDefaults;

CultureInfo.DefaultThreadCurrentCulture = CultureInfo.InvariantCulture;
AssemblyLoadContext.Default.Resolving += (_, name) => {
    var path = Path.Combine(AppContext.BaseDirectory, name.Name + ".dll");
    return File.Exists(path) ? AssemblyLoadContext.Default.LoadFromAssemblyPath(path) : null;
};
var models = AssemblyLoadContext.Default.LoadFromAssemblyPath(Path.Combine(AppContext.BaseDirectory, "Module.Effect.GoldenExtras.dll"));
var serializer = new DataContractSerializer(typeof(EffectDefaultsStore));
var bytes = File.ReadAllBytes(args[0]);
var store = (EffectDefaultsStore)OriginalBinary.ReadBinary(serializer, bytes);
var readCount = store.Entries.Count;
var outDir = Path.GetFullPath(args[1]);
if (Directory.Exists(outDir) || File.Exists(outDir)) throw new IOException("Fresh output directory required");
Directory.CreateDirectory(outDir);
var typed = new List<object>();
foreach (var entry in store.Entries) {
    var type = models.GetType(entry.DataModelTypeName);
    if (type == null) { typed.Add(new { model = entry.DataModelTypeName, opaque = true }); continue; }
    var modelSerializer = new DataContractSerializer(type);
    var data = OriginalBinary.ReadBinary(modelSerializer, entry.Payload);
    var get = (string name) => type.GetProperty(name)?.GetValue(data);
    if ((Guid)get("ModuleInstanceId") != Guid.Empty || (Guid)get("ModuleTypeId") != entry.TypeId) throw new Exception("Nested module identity mismatch");
    var payload = OriginalBinary.WriteBinary(modelSerializer, data);
    using var xmlBytes = new MemoryStream();
    using (var writer = XmlWriter.Create(xmlBytes, new XmlWriterSettings { Encoding = new UTF8Encoding(false), Indent = true })) modelSerializer.WriteObject(writer, data);
    File.WriteAllBytes(Path.Combine(outDir, entry.TypeId+".xml"), xmlBytes.ToArray());
    typed.Add(new { model = entry.DataModelTypeName, opaque = false, moduleTypeId = get("ModuleTypeId"), moduleInstanceId = get("ModuleInstanceId"),
        originalPayload = Convert.ToBase64String(entry.Payload), typedPayload = Convert.ToBase64String(payload) });
}
var report = new { version = store.Version, inputSha256 = Convert.ToHexStringLower(SHA256.HashData(bytes)),
    entries = store.Entries.Select(e => new { typeId=e.TypeId, name=e.EffectName, model=e.DataModelTypeName, savedUtc=e.SavedUtc.ToString("O"), payload=Convert.ToBase64String(e.Payload) }), typed };
File.WriteAllText(Path.Combine(outDir,"read.json"), JsonSerializer.Serialize(report, new JsonSerializerOptions { WriteIndented = true }));
if (args.Length > 2 && args[2] == "--write") {
    store.Entries.Reverse();
    foreach (var entry in store.Entries) {
        var type = models.GetType(entry.DataModelTypeName, throwOnError: true);
        var modelSerializer = new DataContractSerializer(type);
        entry.Payload = OriginalBinary.WriteBinary(modelSerializer, OriginalBinary.ReadBinary(modelSerializer, entry.Payload));
        entry.EffectName = "原始 / " + entry.EffectName;
        entry.SavedUtc = new DateTime(2026,11,11,12,34,56,DateTimeKind.Utc).AddTicks(1234567);
    }
    store.Entries.Insert(1, new EffectDefaultEntry { TypeId=Guid.Parse("d0f4abf0-d543-4fbc-ad4a-75c6b14ea131"), EffectName="Unknown 🦊", DataModelTypeName="Uninstalled.ExampleData",
        SavedUtc=new DateTime(2024,2,29,1,2,3,DateTimeKind.Utc).AddTicks(7654321), Payload=Enumerable.Range(0,66001).Select(i=>(byte)(i*73+19)).ToArray() });
    File.WriteAllBytes(Path.Combine(outDir,"dotnet.vfd"),OriginalBinary.WriteBinary(serializer,store));
}
Console.WriteLine($"Read {readCount} entries with original binary contracts into {outDir}");
