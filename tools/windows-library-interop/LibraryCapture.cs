// C# 5: build with installed Framework csc, RUN with existing .NET 8 Desktop.
// Exact installed types only. No application/service/profile/controller startup.
using System;
using System.Collections;
using System.Collections.Generic;
using System.Drawing;
using System.Globalization;
using System.IO;
using System.Reflection;
using System.Runtime.Serialization;
using System.Security.Cryptography;
using System.Text;
using System.Xml;

internal static class LibraryCapture
{
    private static string Bin;
    private static readonly Dictionary<string,string> Pins = new Dictionary<string,string>(StringComparer.OrdinalIgnoreCase);
    private static readonly string[] Permitted = { "Module.App.Curves.dll", "Module.App.ColorGradients.dll", "Vixen.Core.dll", "Controls.dll", "Resources.dll", "DiscreteColorPicker.dll", "NLog.dll", "NCalc.dll", "NCalc2.dll" };
    private static readonly double[][] CurveX = { new double[]{-10,0,.5,1.5,2.5,25,50,75,100,110}, new double[]{-1,0,12.5,25,62.5,100,101}, new double[]{0,25,50,75,100}, new double[]{0,50,100} };
    private static readonly double[][] GradientX = { new double[]{-1,0,.5,1,2}, new double[]{-1,0,.25,.5,.75,1,2}, new double[]{0,.125,.25,.625,1}, new double[]{0,.25,.5,.75,1} };
    private static readonly string[] CurveNames = { "Ramp & ties", "Three points Caf\u00e9", "Clamp", "Flat 13.5" };
    private static readonly string[] GradientNames = { "Red", "Black to white", "Focus Caf\u00e9 & <quarter>", "Alpha ramp" };

    public static int Main(string[] args)
    {
        try {
            if (args.Length == 1 && args[0] == "--help") { Console.WriteLine("LibraryCapture <private DLL folder> <reviewed pins.tsv> <exact fixture folder> <NEW output folder>. Requires Windows x64 .NET8 Desktop; no Vixen startup."); return 0; }
            if (args.Length != 4) throw new ArgumentException("Expected four paths; see --help.");
            // Refuse other platforms before loading any application assembly.
            if (Environment.OSVersion.Platform != PlatformID.Win32NT || IntPtr.Size != 8 || Environment.Version.Major != 8 || Architecture() != "X64") throw new PlatformNotSupportedException("Requires Windows x64 .NET8 Desktop.");
            Bin = Local(args[0]); string manifest = Local(args[1]), fixtures = Local(args[2]), output = Local(args[3]);
            if (Within(output,Bin) || Within(output,fixtures) || Directory.Exists(output) || File.Exists(output) || !Directory.Exists(Path.GetDirectoryName(output))) throw new IOException("Output must be fresh and outside binaries/fixtures, with an existing parent.");
            ReadPins(manifest); CheckBinaries();
            // Mac fixtures are generated and pinned by the reviewed package step.
            var inputs = ReadFixturePins(Path.Combine(fixtures,"fixture-pins.tsv"));
            foreach (var entry in inputs) if (Hash(Local(Path.Combine(fixtures,entry.Key))) != entry.Value) throw new InvalidDataException("Fixture hash mismatch: " + entry.Key);
            AppDomain.CurrentDomain.AssemblyResolve += Resolve;
            Directory.CreateDirectory(output);
            var curves = Assembly.LoadFrom(Path.Combine(Bin,"Module.App.Curves.dll")).GetType("VixenModules.App.Curves.Curve",true);
            var gradients = Assembly.LoadFrom(Path.Combine(Bin,"Module.App.ColorGradients.dll")).GetType("VixenModules.App.ColorGradients.ColorGradient",true);
            var c = Capture(curves, "Mac.vcl", fixtures, output, CurveNames, CurveX, false);
            var g = Capture(gradients, "Mac.vgl", fixtures, output, GradientNames, GradientX, true);
            CheckBinaries();
            foreach (var entry in inputs) if (Hash(Local(Path.Combine(fixtures,entry.Key))) != entry.Value) throw new InvalidDataException("Input changed during capture.");
            var loaded = new List<object>();
            foreach (Assembly a in AppDomain.CurrentDomain.GetAssemblies()) if (!a.IsDynamic && a.Location.Length > 0) loaded.Add(Obj("identity",a.FullName,"path",a.Location,"sha256",Hash(a.Location)));
            Write(Path.Combine(output,"capture.json"), Obj("schema",1,"status","captured-validation-pending","runtime",Environment.Version.ToString(),"architecture",Architecture(),"productCommit","41c3f4f7e3ea13e8b21648a7398753fb60ad9a5a","curves",c,"gradients",g,"loadedAssemblies",loaded,"inputBinariesUnchanged",true,"scope","Actual installed 3.12.4 data types, DataContractSerializer and evaluator only; no Windows GUI/profile/controller proof"));
            Console.WriteLine("CAPTURE_RESULT: " + Path.Combine(output,"capture.json")); return 0;
        } catch (Exception e) { while (e is TargetInvocationException && e.InnerException != null) e=e.InnerException; Console.Error.WriteLine(e.ToString()); return 1; }
    }
    private static object Capture(Type type,string name,string fixtures,string output,string[] names,double[][] xs,bool gradient)
    {
        var dictionary = typeof(Dictionary<,>).MakeGenericType(typeof(string),type);
        var serializer = new DataContractSerializer(dictionary);
        var path = Path.Combine(fixtures,name);
        // Reject links BEFORE original deserialization may access a property getter.
        var xml = new XmlDocument(); xml.XmlResolver=null;
        using(var reader=XmlReader.Create(path,new XmlReaderSettings { DtdProcessing=DtdProcessing.Prohibit, XmlResolver=null, MaxCharactersInDocument=262144 })) xml.Load(reader);
        foreach(XmlNode node in xml.SelectNodes("//*[local-name()='_libraryReferenceName']")) if(node.InnerText.Length != 0) throw new InvalidDataException("Linked library fixtures are forbidden.");
        IDictionary before;
        using(var file=File.OpenRead(path)) before=(IDictionary)serializer.ReadObject(file);
        var first=Sample(before,type,names,xs,gradient);
        string saved=Path.Combine(output,name);
        using(var file=new FileStream(saved,FileMode.CreateNew,FileAccess.Write,FileShare.None))
        using(var writer=XmlWriter.Create(file,new XmlWriterSettings { Indent=true, IndentChars="\t" })) serializer.WriteObject(writer,before);
        IDictionary after;
        using(var file=File.OpenRead(saved)) after=(IDictionary)serializer.ReadObject(file);
        var second=Sample(after,type,names,xs,gradient);
        if(Json(first)!=Json(second)) throw new InvalidDataException("Original serialize/reload changed sampled data.");
        return Obj("inputSha256",Hash(path),"roundtripSha256",Hash(saved),"samples",first,"roundtripSamples",second);
    }
    private static object Sample(IDictionary map,Type type,string[] names,double[][] xs,bool gradient)
    {
        if(map.Count!=names.Length) throw new InvalidDataException("Unexpected library entry count.");
        MethodInfo method=type.GetMethod(gradient?"GetColorAt":"GetValue",new Type[]{typeof(double)});
        MethodInfo integer=gradient?null:type.GetMethod("GetIntValue",new Type[]{typeof(double)});
        if(method==null || (!gradient && integer==null)) throw new MissingMethodException("Installed evaluator signature differs.");
        var rows=new List<object>();
        for(int i=0;i<names.Length;i++) {
            if(!map.Contains(names[i])) throw new InvalidDataException("Missing exact library name: "+names[i]);
            object item=map[names[i]];
            if(item==null || item.GetType()!=type || (string)Prop(item,"LibraryReferenceName")!="") throw new InvalidDataException("Unexpected or linked library value.");
            var samples=new List<object>();
            foreach(double x in xs[i]) {
                object value=method.Invoke(item,new object[]{x});
                if(gradient) samples.Add(Obj("x",x,"argb",unchecked((uint)((Color)value).ToArgb())));
                else samples.Add(Obj("x",x,"value",(double)value,"intValue",(int)integer.Invoke(item,new object[]{x})));
            }
            rows.Add(Obj("name",names[i],"reference",Prop(item,"LibraryReferenceName"),"current",Prop(item,gradient?"IsCurrentLibraryGradient":"IsCurrentLibraryCurve"),"samples",samples));
        }
        return rows;
    }
    private static object Prop(object value,string name) { var p=value.GetType().GetProperty(name); if(p==null) throw new MissingMemberException(name); return p.GetValue(value,null); }
    private static void ReadPins(string file) {
        foreach(string line in File.ReadAllLines(file)) { var p=line.Split('\t'); if(p.Length!=2 || Array.IndexOf(Permitted,p[0])<0 || !HexHash(p[1]) || Pins.ContainsKey(p[0])) throw new InvalidDataException("Invalid approved binary pins."); Pins.Add(p[0],p[1]); }
        foreach(string name in new string[]{"Module.App.Curves.dll","Module.App.ColorGradients.dll","Vixen.Core.dll","Controls.dll"}) if(!Pins.ContainsKey(name)) throw new InvalidDataException("Required original binary missing.");
    }
    private static Dictionary<string,string> ReadFixturePins(string file) {
        var result=new Dictionary<string,string>(); foreach(string line in File.ReadAllLines(file)) {var p=line.Split('\t');if(p.Length!=2 || (p[0]!="Mac.vcl" && p[0]!="Mac.vgl") || !HexHash(p[1]))throw new InvalidDataException("Bad fixture pins.");result.Add(p[0],p[1]);} if(result.Count!=2 || result["Mac.vcl"]!="328060d89d0df7ca2a4bfeef7f2a2de39cb5fab02e1f0a52a39a7815f922537d" || result["Mac.vgl"]!="6c21e203acee4f26152537b39c0cfc2ca9aa0e62e225f661cfa545b94d87a4df")throw new InvalidDataException("Expected exact reviewed Mac fixtures.");return result;
    }
    private static string Architecture() {
        var t=typeof(object).Assembly.GetType("System.Runtime.InteropServices.RuntimeInformation") ?? Type.GetType("System.Runtime.InteropServices.RuntimeInformation, System.Runtime.InteropServices.RuntimeInformation",false);
        var p=t==null?null:t.GetProperty("ProcessArchitecture",BindingFlags.Public|BindingFlags.Static);
        return p==null?"Unknown":Convert.ToString(p.GetValue(null,null),CultureInfo.InvariantCulture);
    }
    private static bool HexHash(string value) {if(value.Length!=64)return false;foreach(char c in value)if(!((c>='0'&&c<='9')||(c>='a'&&c<='f')))return false;return true;}
    private static void CheckBinaries() {if(Directory.GetDirectories(Bin).Length!=0 || Directory.GetFiles(Bin).Length!=Pins.Count)throw new InvalidDataException("Private DLL folder contains unapproved files.");foreach(var p in Pins) if(Hash(Local(Path.Combine(Bin,p.Key)))!=p.Value)throw new InvalidDataException("Original DLL hash changed: "+p.Key);}
    private static Assembly Resolve(object sender,ResolveEventArgs args) {string name=new AssemblyName(args.Name).Name+".dll";if(!Pins.ContainsKey(name))throw new FileNotFoundException("Dependency outside reviewed private set: "+name);string path=Path.Combine(Bin,name);if(Hash(path)!=Pins[name])throw new InvalidDataException("Dependency changed.");return Assembly.LoadFrom(path);}
    private static bool Within(string p,string d) {return p.Equals(d,StringComparison.OrdinalIgnoreCase)||p.StartsWith(d.TrimEnd(Path.DirectorySeparatorChar)+Path.DirectorySeparatorChar,StringComparison.OrdinalIgnoreCase);}
    private static string Local(string path) {path=Path.GetFullPath(path);if(path.StartsWith("\\\\"))throw new ArgumentException("UNC paths are excluded.");for(string p=path;!String.IsNullOrEmpty(p);p=Path.GetDirectoryName(p))if((File.Exists(p)||Directory.Exists(p))&&(File.GetAttributes(p)&FileAttributes.ReparsePoint)!=0)throw new ArgumentException("Links/cloud placeholders are excluded.");return path;}
    private static string Hash(string path) {using(var f=File.OpenRead(path))using(var sha=SHA256.Create())return BitConverter.ToString(sha.ComputeHash(f)).Replace("-","").ToLowerInvariant();}
    private static Dictionary<string,object> Obj(params object[] values) {var r=new Dictionary<string,object>();for(int i=0;i<values.Length;i+=2)r.Add((string)values[i],values[i+1]);return r;}
    private static void Write(string path,object value) {using(var f=new FileStream(path,FileMode.CreateNew,FileAccess.Write,FileShare.None))using(var w=new StreamWriter(f,new UTF8Encoding(false)))w.WriteLine(Json(value));}
    private static string Json(object value) {var s=new StringBuilder();Append(s,value);return s.ToString();}
    private static void Append(StringBuilder s,object v) {
        if(v==null){s.Append("null");return;}var text=v as string;if(text!=null){s.Append('"');foreach(char c in text){if(c=='"'||c=='\\'){s.Append('\\');s.Append(c);}else if(c<32||c>=127){s.Append("\\u");s.Append(((int)c).ToString("x4"));}else s.Append(c);}s.Append('"');return;}
        if(v is bool){s.Append((bool)v?"true":"false");return;}var map=v as IDictionary;if(map!=null){s.Append('{');bool comma=false;foreach(DictionaryEntry e in map){if(comma)s.Append(',');comma=true;Append(s,e.Key);s.Append(':');Append(s,e.Value);}s.Append('}');return;}
        var list=v as IEnumerable;if(list!=null){s.Append('[');bool comma=false;foreach(object e in list){if(comma)s.Append(',');comma=true;Append(s,e);}s.Append(']');return;}
        if(v is double){double d=(double)v;if(Double.IsNaN(d)||Double.IsInfinity(d))throw new InvalidDataException("Nonfinite sample.");s.Append(d.ToString("R",CultureInfo.InvariantCulture));return;}
        if(v is int||v is uint||v is long){s.Append(Convert.ToString(v,CultureInfo.InvariantCulture));return;}throw new InvalidDataException("Unsupported JSON type.");
    }
}
