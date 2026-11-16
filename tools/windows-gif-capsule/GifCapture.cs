// C#5 / built-in Windows Framework: GDI+ decode and draw only.
// The scheduling below is SOURCE-DERIVED, not installed Picture.SetupRender.
using System;
using System.Collections;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Imaging;
using System.Globalization;
using System.IO;
using System.Security.Cryptography;
using System.Text;
internal static class GifCapture {
    static readonly string[] Names={"full","keep","background","previous"};
    static readonly string[] Hashes={"136e4e1f7d3da6d96887a1800d72607bd4dbf0c709a39ccd25ab59a9b3323a1e","f202a87255e1222a4c19ad12a13b5af44a1436b3d9ef50f49baa57a42055e8ab","6db71651f3e109b81ad22e139a11ef6f4e060dedfffd797577a02e2f5fc48130","68c553420880e5a85bc16234ddc739edbbd7d34387f2285dc71576d12e464018"};
    static int Main(string[] args) {
        try {
            if(args.Length==1 && args[0]=="--schedules") {var all=new List<object>();foreach(string n in Names)foreach(int speed in new int[]{1,3})all.Add(Obj("id",n+"-speed"+speed,"schedule",Schedule(speed)));Console.WriteLine(Json(all));return 0;}
            if(args.Length!=3)throw new ArgumentException("GifCapture <fixture directory> <exact case id> <NEW output JSON>");
            if(Environment.OSVersion.Platform!=PlatformID.Win32NT)throw new PlatformNotSupportedException("Windows GDI+ required.");
            string folder=Local(args[0]),id=args[1],output=Local(args[2]);
            if(File.Exists(output)||Directory.Exists(output)||!Directory.Exists(Path.GetDirectoryName(output))||Within(output,folder))throw new IOException("Output must be fresh and outside fixture directory.");
            int which=-1,speedValue=0;for(int i=0;i<Names.Length;i++)foreach(int speed in new int[]{1,3})if(id==Names[i]+"-speed"+speed){which=i;speedValue=speed;}
            if(which<0)throw new ArgumentException("Unknown case.");
            string input=Local(Path.Combine(folder,Names[which]+".gif"));
            if(new FileInfo(input).Length>1024||Hash(input)!=Hashes[which])throw new InvalidDataException("Fixture hash/size mismatch.");
            var decoded=new List<object>();var cached=new List<object>();
            byte[] delays;
            using(var stream=File.OpenRead(input))using(var image=Image.FromStream(stream)) {
                var dim=Dimension(image);delays=image.GetPropertyItem(0x5100).Value;
                if(delays.Length!=16)throw new InvalidDataException("Expected four GIF delays.");
                for(int i=0;i<4;i++){image.SelectActiveFrame(dim,i);decoded.Add(Frame((Bitmap)image,"sourceFrame",i));}
            }
            // A fresh GDI image follows the original call order, including frame
            // jumps at speed3. Keep these pixels distinct from sequential decode.
            using(var stream=File.OpenRead(input))using(var image=Image.FromStream(stream)) {
                var dim=Dimension(image);
                foreach(Dictionary<string,object> s in Schedule(speedValue)) {
                    image.SelectActiveFrame(dim,(int)s["sourceFrame"]);
                    using(var bitmap=OriginalPictureScale.ScaleImage(image,1.0))cached.Add(Frame(bitmap,"effectFrame",(int)s["effectFrame"]));
                }
            }
            if(Hash(input)!=Hashes[which])throw new InvalidDataException("Input changed during capture.");
            var delayValues=new List<int>();for(int i=0;i<4;i++)delayValues.Add(BitConverter.ToInt32(delays,i*4));
            var value=Obj("schema",1,"id",id,"file",Names[which]+".gif","gifSpeed",speedValue,"inputSha256",Hashes[which],"runtime",Environment.Version.ToString(),"os",Environment.OSVersion.ToString(),"pointerSize",IntPtr.Size,"drawingAssembly",typeof(Bitmap).Assembly.FullName,"decodedFrames",decoded,"cache",cached,"delaysCentiseconds",delayValues,"sourceSchedule",Schedule(speedValue),"scope","Windows GDI+ decode + unchanged original ScaleImage; SOURCE-DERIVED Picture schedule; installed effect not executed","installedEffectExecuted",false);
            using(var f=new FileStream(output,FileMode.CreateNew,FileAccess.Write,FileShare.None))using(var w=new StreamWriter(f,new UTF8Encoding(false)))w.WriteLine(Json(value));
            Console.WriteLine("CAPTURED "+id);return 0;
        }catch(Exception e){Console.Error.WriteLine(e);return 1;}
    }
    static FrameDimension Dimension(Image image){if(image.Width!=4||image.Height!=4||image.FrameDimensionsList.Length!=1)throw new InvalidDataException("Unexpected GIF canvas/dimensions.");var d=new FrameDimension(image.FrameDimensionsList[0]);if(image.GetFrameCount(d)!=4)throw new InvalidDataException("Unexpected frame count.");return d;}
    static List<object> Schedule(int speed){var list=new List<object>();int previous=-1;for(int frame=0;frame<12;frame++){int selected=(int)Math.Round(((frame*50.0/600*speed)%1)*3);if(selected>previous)list.Add(Obj("effectFrame",frame,"sourceFrame",selected));previous=selected;}return list;}
    static object Frame(Bitmap b,string key,int index){var px=new List<int>();for(int y=0;y<b.Height;y++)for(int x=0;x<b.Width;x++){Color c=b.GetPixel(x,y);px.Add(c.A);px.Add(c.R);px.Add(c.G);px.Add(c.B);}return Obj(key,index,"width",b.Width,"height",b.Height,"argb",px);}
    static bool Within(string p,string root){return p.Equals(root,StringComparison.OrdinalIgnoreCase)||p.StartsWith(root.TrimEnd(Path.DirectorySeparatorChar)+Path.DirectorySeparatorChar,StringComparison.OrdinalIgnoreCase);}
    static string Local(string path){path=Path.GetFullPath(path);if(path.StartsWith("\\\\"))throw new ArgumentException("UNC excluded.");for(string p=path;!String.IsNullOrEmpty(p);p=Path.GetDirectoryName(p))if((File.Exists(p)||Directory.Exists(p))&&(File.GetAttributes(p)&FileAttributes.ReparsePoint)!=0)throw new ArgumentException("Links/cloud placeholders excluded.");return path;}
    static string Hash(string path){using(var f=File.OpenRead(path))using(var sha=SHA256.Create())return BitConverter.ToString(sha.ComputeHash(f)).Replace("-","").ToLowerInvariant();}
    static Dictionary<string,object> Obj(params object[] values){var r=new Dictionary<string,object>();for(int i=0;i<values.Length;i+=2)r.Add((string)values[i],values[i+1]);return r;}
    static string Json(object value){var s=new StringBuilder();Append(s,value);return s.ToString();}
    static void Append(StringBuilder s,object v){
        if(v==null){s.Append("null");return;}var text=v as string;if(text!=null){s.Append('"');foreach(char c in text){if(c=='"'||c=='\\'){s.Append('\\');s.Append(c);}else if(c<32||c>=127){s.Append("\\u");s.Append(((int)c).ToString("x4"));}else s.Append(c);}s.Append('"');return;}
        if(v is bool){s.Append((bool)v?"true":"false");return;}var map=v as IDictionary;if(map!=null){s.Append('{');bool comma=false;foreach(DictionaryEntry e in map){if(comma)s.Append(',');comma=true;Append(s,e.Key);s.Append(':');Append(s,e.Value);}s.Append('}');return;}
        var list=v as IEnumerable;if(list!=null){s.Append('[');bool comma=false;foreach(object e in list){if(comma)s.Append(',');comma=true;Append(s,e);}s.Append(']');return;}
        if(v is int){s.Append(Convert.ToString(v,CultureInfo.InvariantCulture));return;}throw new InvalidDataException("Unsupported JSON type.");
    }
}
